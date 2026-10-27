// Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
// Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use miette::Result;
use tracing::{debug, info, info_span};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod cli;
mod config;
mod error;
mod profile;

use cli::{
    CheckArgs, Cli, Commands, ConfigAction, ConfigArgs, GetArgs, InitArgs, PackageArgs, RunArgs,
    SyncArgs,
};
use config::{load_global_config, resolve_config_dir, resolve_home_dir};
use error::CliError;
use peri_archive::{
    compute_file_sha256, create_peca, extract_peca, generate_package_file_index,
    load_package_vfi_cache, read_manifest_from_peca, save_package_vfi_cache,
};
use peri_core::{
    solve_workspace, validate_safe_path, DependencyGraph, GameProfile, Lockfile, Manifest,
    OverrideRule, PackageCatalog, PackageFileIndex, PackageMetadata, ResolutionSource, TargetGame,
    VirtualFileIndex, LOCKFILE_FILENAME, MANIFEST_FILENAME,
};
use peri_vfs::VfsPlan;
use profile::resolve_game_profile;

const CACHE_MODS_DIR: &str = ".perihelion/cache/mods";
const VFI_CACHE_DIR: &str = ".perihelion/cache/vfi";
const EXTRACTED_DIR: &str = ".perihelion/extracted";
const UPPER_DIR: &str = ".perihelion/upper";
const WORK_DIR: &str = ".perihelion/work";
const MOUNT_DIR: &str = ".perihelion/mnt";
pub(crate) const PROFILES_DIR: &str = ".perihelion/profiles";

fn init_tracing(verbose: bool) {
    let default_filter = if verbose {
        "peri=debug,peri_vfs=debug,peri_net=debug,peri_archive=debug,info"
    } else {
        "peri=warn,peri_vfs=warn,peri_net=warn,peri_archive=warn,warn"
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).compact())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Commands::Init(args) => handle_init(args).await?,
        Commands::Get(args) => handle_get(args).await?,
        Commands::Sync(args) => handle_sync(args).await?,
        Commands::Package(args) => handle_package(args).await?,
        Commands::Check(args) => handle_check(args).await?,
        Commands::Run(args) => handle_run(args).await?,
        Commands::Config(args) => handle_config(args).await?,
        Commands::Profile(args) => profile::handle_profile(args).await?,
    }

    Ok(())
}

fn load_or_cache_vfi(
    vfi_cache_dir: &Path,
    peca_path: &Path,
    manifest: &Manifest,
) -> Result<PackageFileIndex, CliError> {
    if let Ok(Some(cached)) = load_package_vfi_cache(vfi_cache_dir, &manifest.package.name) {
        return Ok(cached);
    }

    let index = generate_package_file_index(peca_path)?;
    let _ = save_package_vfi_cache(vfi_cache_dir, &index);
    Ok(index)
}

pub(crate) fn validate_workspace_path(path: &Path) -> Result<PathBuf, CliError> {
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| CliError::Workspace(e.to_string()))?
            .join(path)
    };
    if let Some(home) = resolve_home_dir() {
        Ok(validate_safe_path(&abs_path, &home)?)
    } else {
        Ok(abs_path)
    }
}

async fn handle_config(args: ConfigArgs) -> Result<(), CliError> {
    let span = info_span!("config");
    let _enter = span.enter();

    let action = args.action.unwrap_or(ConfigAction::Show);
    let config_dir = resolve_config_dir();
    let config_file = config_dir.join(peri_core::CONFIG_FILENAME);

    match action {
        ConfigAction::Path => {
            println!("{}", config_file.display());
        }
        ConfigAction::Show => {
            let config = load_global_config();
            let toml = config.to_toml_str()?;
            println!(
                "# Global configuration from: {}\n{}",
                config_file.display(),
                toml
            );
        }
        ConfigAction::Init => {
            if config_file.exists() {
                println!(
                    "Global configuration file already exists at {}",
                    config_file.display()
                );
            } else {
                fs::create_dir_all(&config_dir).map_err(|e| CliError::Workspace(e.to_string()))?;
                let config = if let Some(home) = resolve_home_dir() {
                    peri_core::GlobalConfig::default_for_home(&home)
                } else {
                    peri_core::GlobalConfig::default()
                };
                let toml = config.to_toml_str()?;
                fs::write(&config_file, toml).map_err(|e| CliError::Workspace(e.to_string()))?;
                println!(
                    "Created default configuration file at {}",
                    config_file.display()
                );
            }
        }
    }

    Ok(())
}

async fn handle_init(args: InitArgs) -> Result<(), CliError> {
    let span = info_span!("init", game_id = %args.game_id);
    let _enter = span.enter();

    let workspace_path = validate_workspace_path(&args.path)?;
    fs::create_dir_all(&workspace_path).map_err(|e| CliError::Workspace(e.to_string()))?;

    let manifest_path = workspace_path.join(MANIFEST_FILENAME);
    if manifest_path.exists() {
        info!(
            path = %manifest_path.display(),
            "peri.toml already exists in target workspace"
        );
    } else {
        let name = args.name.unwrap_or_else(|| {
            workspace_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed_modlist".to_string())
        });

        let manifest = Manifest {
            package: PackageMetadata {
                name,
                author: std::env::var("USER").unwrap_or_else(|_| "author".to_string()),
                version: "0.1.0".to_string(),
                priority: Default::default(),
                description: Some(format!("Perihelion mod configuration for {}", args.game_id)),
            },
            target: TargetGame {
                game_id: args.game_id.clone(),
                engine_version: None,
                game_path: None,
            },
            assets: None,
            dependencies: Default::default(),
            overrides: Default::default(),
            load_order: None,
        };

        let toml_str = manifest.to_toml_str()?;
        fs::write(&manifest_path, toml_str).map_err(|e| CliError::Workspace(e.to_string()))?;
        info!(path = %manifest_path.display(), "Created peri.toml manifest");
    }

    // Create .perihelion cache, profile, and runtime state directories
    fs::create_dir_all(workspace_path.join(CACHE_MODS_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(VFI_CACHE_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(PROFILES_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(EXTRACTED_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(UPPER_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(WORK_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(MOUNT_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;

    let profile = resolve_game_profile(&workspace_path, &args.game_id);
    println!(
        "Initialized Perihelion workspace for '{}' ({}) at {}",
        profile.name,
        profile.id,
        workspace_path
            .canonicalize()
            .unwrap_or(workspace_path)
            .display()
    );
    if let Some(app_id) = profile.steam_app_id {
        println!("   Steam App ID: {}", app_id);
    }

    Ok(())
}

async fn handle_package(args: PackageArgs) -> Result<(), CliError> {
    let span = info_span!("package");
    let _enter = span.enter();

    let source_dir = validate_workspace_path(&args.source)?;
    let manifest_path = source_dir.join(MANIFEST_FILENAME);

    if !manifest_path.exists() {
        return Err(CliError::Archive(format!(
            "Missing '{}' in source directory {}",
            MANIFEST_FILENAME,
            source_dir.display()
        )));
    }

    let manifest_content =
        fs::read_to_string(&manifest_path).map_err(|e| CliError::Workspace(e.to_string()))?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    let output_path = args.output.unwrap_or_else(|| {
        let filename = format!(
            "{}-{}.peca",
            manifest.package.name.replace(' ', "_"),
            manifest.package.version
        );
        source_dir.join(filename)
    });
    let output_path = validate_workspace_path(&output_path)?;

    info!(source = %source_dir.display(), target = %output_path.display(), "Packaging .peca archive");

    let packaged_manifest = create_peca(&source_dir, &output_path, args.level)?;
    let sha256 = compute_file_sha256(&output_path)?;

    println!("Successfully packaged .peca archive:");
    println!("   File:    {}", output_path.display());
    println!(
        "   Mod:     {} v{}",
        packaged_manifest.package.name, packaged_manifest.package.version
    );
    println!("   Target:  {}", packaged_manifest.target.game_id);
    println!("   SHA-256: {}", sha256);

    Ok(())
}

async fn handle_get(args: GetArgs) -> Result<(), CliError> {
    let span = info_span!("get", uri = %args.uri);
    let _enter = span.enter();

    let _global_config = load_global_config();
    let cache_dir = Path::new(CACHE_MODS_DIR);
    fs::create_dir_all(cache_dir).map_err(|e| CliError::Workspace(e.to_string()))?;

    let uri = &args.uri;
    let dest_file: PathBuf;

    if uri.starts_with("http://") || uri.starts_with("https://") {
        let filename = args.output.unwrap_or_else(|| {
            uri.split('/')
                .next_back()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("downloaded.peca"))
        });
        dest_file = cache_dir.join(filename);
        if let Some(home) = config::resolve_home_dir() {
            validate_safe_path(&dest_file, &home)?;
        }
        info!(url = %uri, "Downloading archive over HTTP");
        peri_net::fetch_http(uri, &dest_file, None).await?;
    } else if uri.starts_with("magnet:") {
        let filename = args.output.unwrap_or_else(|| PathBuf::from("torrent.peca"));
        dest_file = cache_dir.join(filename);
        if let Some(home) = config::resolve_home_dir() {
            validate_safe_path(&dest_file, &home)?;
        }
        info!(magnet = %uri, "Downloading archive via BitTorrent");
        peri_net::fetch_magnet(uri, &dest_file).await?;
    } else {
        // Local file path
        let source_path = Path::new(uri);
        if !source_path.exists() {
            return Err(CliError::Archive(format!("Local file not found: {uri}")));
        }
        let filename = args.output.unwrap_or_else(|| {
            source_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("mod.peca"))
        });
        dest_file = cache_dir.join(filename);
        if let Some(home) = config::resolve_home_dir() {
            validate_safe_path(&dest_file, &home)?;
        }
        fs::copy(source_path, &dest_file).map_err(|e| CliError::Archive(e.to_string()))?;
    }

    let manifest = read_manifest_from_peca(&dest_file)?;
    let vfi_cache_dir = Path::new(VFI_CACHE_DIR);
    let index = load_or_cache_vfi(vfi_cache_dir, &dest_file, &manifest)?;

    debug!(
        mod_id = %manifest.package.name,
        version = %manifest.package.version,
        files_indexed = index.files.len(),
        path = %dest_file.display(),
        "Package successfully cached"
    );

    println!(
        "Successfully cached mod '{}' v{} ({} files indexed) -> {}",
        manifest.package.name,
        manifest.package.version,
        index.files.len(),
        dest_file.display()
    );

    Ok(())
}

async fn handle_sync(args: SyncArgs) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let span = info_span!("sync", workspace = %workspace.display());
    let _enter = span.enter();

    let cache_dir = workspace.join(CACHE_MODS_DIR);
    let extracted_dir = workspace.join(EXTRACTED_DIR);

    if !cache_dir.exists() {
        return Err(CliError::Workspace(format!(
            "Workspace cache not found at {}. Run 'peri init' first.",
            cache_dir.display()
        )));
    }

    fs::create_dir_all(&extracted_dir).map_err(|e| CliError::Workspace(e.to_string()))?;

    let root_manifest_path = workspace.join(MANIFEST_FILENAME);
    let lockfile_path = workspace.join(LOCKFILE_FILENAME);
    let mut overrides: HashMap<String, OverrideRule> = HashMap::new();
    let mut game_profile = GameProfile::generic("generic");
    let mut root_manifest_opt = None;

    if root_manifest_path.exists() {
        let content = fs::read_to_string(&root_manifest_path)
            .map_err(|e| CliError::Workspace(e.to_string()))?;
        let root_manifest = Manifest::from_toml_str(&content)?;
        overrides = root_manifest.overrides.clone().into_iter().collect();
        game_profile = resolve_game_profile(&workspace, &root_manifest.target.game_id);
        root_manifest_opt = Some(root_manifest);
    }

    let mut catalog = PackageCatalog::new();
    let mut peca_files: Vec<(Manifest, PathBuf)> = Vec::new();
    let vfi_cache_dir = workspace.join(VFI_CACHE_DIR);

    for entry in fs::read_dir(&cache_dir).map_err(|e| CliError::Workspace(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::Workspace(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("peca") {
            let manifest = read_manifest_from_peca(&path)?;
            catalog.add_package(manifest.clone());
            peca_files.push((manifest, path));
        }
    }

    // Stage 1: Version Selection (PubGrub CDCL SAT Solver & peri.lock)
    let selected_versions = if let Some(ref root_manifest) = root_manifest_opt {
        if !root_manifest.dependencies.is_empty() {
            let mut use_lock = false;
            let mut resolved = BTreeMap::new();

            if lockfile_path.exists() {
                if let Ok(content) = fs::read_to_string(&lockfile_path) {
                    if let Ok(lock) = Lockfile::from_toml_str(&content) {
                        if lock.satisfies_requirements(&root_manifest.dependencies) {
                            resolved = lock.package_map();
                            use_lock = true;
                            info!("Reusing locked package versions from peri.lock");
                        }
                    }
                }
            }

            if !use_lock {
                info!("Running PubGrub CDCL SAT solver for dependencies...");
                resolved = solve_workspace(&catalog, root_manifest)?;
                let lockfile = Lockfile::from_solution(&resolved);
                let lock_toml = lockfile.to_toml_str()?;
                fs::write(&lockfile_path, lock_toml)
                    .map_err(|e| CliError::Workspace(e.to_string()))?;
                println!("Generated updated {}", LOCKFILE_FILENAME);
            }
            Some(resolved)
        } else {
            None
        }
    } else {
        None
    };

    // Filter to active packages based on PubGrub version selection if resolved
    let active_peca_files: Vec<(Manifest, PathBuf)> = if let Some(ref selected) = selected_versions
    {
        peca_files
            .into_iter()
            .filter(|(m, _)| {
                if let Some(target_v) = selected.get(&m.package.name) {
                    if let Ok(v) = semver::Version::parse(&m.package.version) {
                        return &v == target_v;
                    }
                }
                false
            })
            .collect()
    } else {
        peca_files
    };

    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();

    for (manifest, path) in &active_peca_files {
        let index = load_or_cache_vfi(&vfi_cache_dir, path, manifest)?;
        vfi.add_package_index(&index, |p| {
            game_profile
                .map_path(Path::new(p))
                .to_string_lossy()
                .into_owned()
        });
        graph.add_manifest(manifest.clone());
    }

    // Stage 3: Apply the 3-Tier Resolution Waterfall
    let _reports = vfi.resolve_conflicts_waterfall(&mut graph, &overrides)?;

    // Stage 2: Validate DAG and resolve load order
    let resolved_order = graph.resolve_load_order()?;
    println!("Resolving load order and synchronizing VFS layers...");

    for name in &resolved_order {
        if let Some((manifest, peca_path)) = active_peca_files
            .iter()
            .find(|(m, _)| &m.package.name == name)
        {
            let unpack_span = info_span!(
                "unpack_layer",
                mod_id = %manifest.package.name,
                version = %manifest.package.version
            );
            let _unpack_enter = unpack_span.enter();

            let dest_layer = extracted_dir.join(&manifest.package.name);
            extract_peca(peca_path, &dest_layer)?;
            println!(
                "   Unpacked {} v{} -> {}",
                manifest.package.name,
                manifest.package.version,
                dest_layer.display()
            );
        }
    }

    println!(
        "Workspace synchronization complete: {} mods active.",
        resolved_order.len()
    );
    Ok(())
}

async fn handle_check(args: CheckArgs) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let span = info_span!("check", workspace = %workspace.display());
    let _enter = span.enter();

    let root_manifest_path = workspace.join(MANIFEST_FILENAME);
    let mut overrides: HashMap<String, OverrideRule> = HashMap::new();
    let mut game_profile = GameProfile::generic("generic");
    let mut root_manifest_opt = None;

    if root_manifest_path.exists() {
        let content = fs::read_to_string(&root_manifest_path)
            .map_err(|e| CliError::Workspace(e.to_string()))?;
        let root_manifest = Manifest::from_toml_str(&content)?;
        overrides = root_manifest.overrides.clone().into_iter().collect();
        game_profile = resolve_game_profile(&workspace, &root_manifest.target.game_id);
        root_manifest_opt = Some(root_manifest);
    }

    let cache_dir = workspace.join(CACHE_MODS_DIR);
    if !cache_dir.exists() {
        println!("No mods cached in {}", cache_dir.display());
        return Ok(());
    }

    let mut catalog = PackageCatalog::new();
    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();
    let mut count = 0;
    let vfi_cache_dir = workspace.join(VFI_CACHE_DIR);

    for entry in fs::read_dir(&cache_dir).map_err(|e| CliError::Workspace(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::Workspace(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("peca") {
            let manifest = read_manifest_from_peca(&path)?;
            catalog.add_package(manifest.clone());
            let index = load_or_cache_vfi(&vfi_cache_dir, &path, &manifest)?;
            vfi.add_package_index(&index, |p| {
                game_profile
                    .map_path(Path::new(p))
                    .to_string_lossy()
                    .into_owned()
            });
            graph.add_manifest(manifest);
            count += 1;
        }
    }

    // Stage 1: Validate version constraints with PubGrub
    if let Some(ref root_manifest) = root_manifest_opt {
        if !root_manifest.dependencies.is_empty() {
            let solved = solve_workspace(&catalog, root_manifest)?;
            println!(
                "Stage 1: PubGrub SAT solver verified {} version constraints.",
                solved.len()
            );
        }
    }

    println!(
        "Stage 2 & 3: Validating dependency DAG and Virtual File Index (VFI) for {} packages...",
        count
    );

    // Apply the 3-Tier Resolution Waterfall
    let reports = vfi.resolve_conflicts_waterfall(&mut graph, &overrides)?;
    if !reports.is_empty() {
        println!(
            "Detected and resolved {} file conflict(s) via 3-Tier Waterfall:",
            reports.len()
        );
        for report in &reports {
            match &report.source {
                ResolutionSource::ExplicitEdge { winner } => {
                    println!(
                        "   • [Tier 1: Explicit Edge] '{}' resolved -> '{}' wins",
                        report.file_path, winner
                    );
                }
                ResolutionSource::PriorityWeight { winner, priority } => {
                    println!(
                        "   • [Tier 2: Priority {:?}] '{}' resolved -> '{}' wins",
                        priority, report.file_path, winner
                    );
                }
            }
        }
    } else {
        println!("Virtual File Index check clean: no file conflicts detected.");
    }

    let order = graph.resolve_load_order()?;
    println!("DAG check passed successfully. No cycles detected!");
    println!("Calculated Deterministic Load Order (OverlayFS stack):");
    for (idx, mod_name) in order.iter().enumerate() {
        println!("   [{}] {}", idx + 1, mod_name);
    }

    Ok(())
}

async fn handle_run(args: RunArgs) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let root_manifest_path = workspace.join(MANIFEST_FILENAME);

    if !root_manifest_path.exists() {
        return Err(CliError::Workspace(format!(
            "Missing {} in workspace {}. Run 'peri init' first.",
            MANIFEST_FILENAME,
            workspace.display()
        )));
    }

    let manifest_content =
        fs::read_to_string(&root_manifest_path).map_err(|e| CliError::Workspace(e.to_string()))?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    let span = info_span!("run", game_id = %manifest.target.game_id);
    let _enter = span.enter();

    let profile = resolve_game_profile(&workspace, &manifest.target.game_id);

    let cache_dir = workspace.join(CACHE_MODS_DIR);
    let vfi_cache_dir = workspace.join(VFI_CACHE_DIR);
    let extracted_dir = workspace.join(EXTRACTED_DIR);
    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();
    let overrides: HashMap<String, OverrideRule> = manifest.overrides.clone().into_iter().collect();

    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir).map_err(|e| CliError::Workspace(e.to_string()))? {
            let entry = entry.map_err(|e| CliError::Workspace(e.to_string()))?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("peca") {
                let m = read_manifest_from_peca(&path)?;
                let index = load_or_cache_vfi(&vfi_cache_dir, &path, &m)?;
                vfi.add_package_index(&index, |p| {
                    profile
                        .map_path(Path::new(p))
                        .to_string_lossy()
                        .into_owned()
                });
                graph.add_manifest(m);
            }
        }
    }

    let _reports = vfi.resolve_conflicts_waterfall(&mut graph, &overrides)?;
    let resolved_order = graph.resolve_load_order()?;
    let layers: Vec<PathBuf> = resolved_order
        .iter()
        .map(|name| extracted_dir.join(name))
        .collect();

    let global_config = crate::config::load_global_config_with_workspace(Some(&workspace));
    let provider = global_config.provider("steam");

    let resolved_base = peri_vfs::resolve_base_game_path(&peri_vfs::BaseGameQuery {
        cli_path: args.base_path.as_deref(),
        env_var: std::env::var("PERI_BASE_GAME").ok().as_deref(),
        manifest_path: manifest.target.game_path.as_deref(),
        profile_path: profile.game_path.as_deref(),
        steam_app_id: profile.steam_app_id,
        provider: Some(&provider),
        workspace: Some(&workspace),
    })
    .map_err(|e| CliError::Vfs(e.to_string()))?;

    let base_dir = resolved_base.path;
    let mount_point = workspace.join(MOUNT_DIR);
    let upper_dir = workspace.join(UPPER_DIR);
    let work_dir = workspace.join(WORK_DIR);

    let plan = VfsPlan::new(base_dir.clone(), mount_point)
        .with_layers(layers)
        .with_upper(upper_dir, work_dir);

    let mount_options = plan.format_mount_options()?;
    let exec_cmd = args
        .exec
        .or(profile.default_executable)
        .unwrap_or_else(|| "echo 'No executable configured'".to_string());

    let use_proton = !args.native
        && (profile.steam_app_id.is_some()
            || args.proton.is_some()
            || args.prefix.is_some()
            || exec_cmd.to_lowercase().ends_with(".exe"));

    let proton_launch = if use_proton {
        match peri_vfs::SteamDiscovery::resolve_proton_launch_with_config(
            profile.steam_app_id,
            args.proton.as_deref(),
            args.prefix.as_deref(),
            &provider,
        ) {
            Ok(launch) => Some(launch),
            Err(e) => {
                if args.proton.is_some() || args.prefix.is_some() {
                    return Err(CliError::Vfs(e.to_string()));
                }
                info!(
                    "Proton auto-discovery did not find an active installation ({e}); falling back to native runner"
                );
                None
            }
        }
    } else {
        None
    };

    println!("Preparing Perihelion Virtual File System sandbox:");
    println!("   Game:          {} ({})", profile.name, profile.id);
    println!(
        "   Base Game:     {} ({})",
        base_dir.display(),
        resolved_base.source
    );
    println!("   Active Layers: {}", resolved_order.len());
    println!("   Mount Point:   {}", plan.mount_point.display());
    println!("   Mount Options: {}", mount_options);

    if let Some(ref proton) = proton_launch {
        println!("Compatibility Layer: Proton / Wine");
        println!("   Proton Runner:       {}", proton.runner.display());
        println!(
            "   Compat Data:         {}",
            proton.compat_data_path.display()
        );
        println!("   Wine Prefix:         {}", proton.wine_prefix.display());
        println!("   Target Binary:       {} {:?}", exec_cmd, args.args);
    } else {
        println!("Compatibility Layer: Native Linux process");
        println!("   Target Binary:       {} {:?}", exec_cmd, args.args);
    }

    if args.dry_run {
        println!("[DRY-RUN] Verification complete. Sandboxed mount and execution ready.");
        if let Some(ref proton) = proton_launch {
            println!(
                "   [DRY-RUN] Command payload: {} run {} {:?}",
                proton.runner.display(),
                exec_cmd,
                args.args
            );
            println!("   [DRY-RUN] Injected environment:");
            for (k, v) in &proton.env_vars {
                if k.starts_with("STEAM_") || k == "WINEPREFIX" || k.starts_with("Steam") {
                    println!("             {k}={v}");
                }
            }
        }
        return Ok(());
    }

    let exit_code = if let Some(ref proton) = proton_launch {
        peri_vfs::execute_proton_in_isolated_namespace(&plan, proton, &exec_cmd, &args.args)?
    } else {
        peri_vfs::execute_in_isolated_namespace(&plan, &exec_cmd, &args.args)?
    };
    info!(exit_code = exit_code, "Execution finished");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_run_with_dynamic_base_path() {
        let cur = std::env::current_dir().unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".tmp_peri_test_")
            .tempdir_in(&cur)
            .unwrap();
        let ws = temp.path().join("workspace");
        fs::create_dir_all(&ws).unwrap();

        let manifest_content = r#"
[package]
name = "my_modlist"
author = "Tester"
version = "0.1.0"

[target]
game_id = "test_game"
"#;
        fs::write(ws.join(MANIFEST_FILENAME), manifest_content).unwrap();

        let custom_game = temp.path().join("custom_game_root");
        fs::create_dir_all(&custom_game).unwrap();

        let run_args = RunArgs {
            workspace: ws,
            base_path: Some(custom_game),
            dry_run: true,
            exec: Some("echo hello".to_string()),
            proton: None,
            prefix: None,
            native: true,
            args: Vec::new(),
        };

        let result = handle_run(run_args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_run_missing_base_game_fails_descriptively() {
        let cur = std::env::current_dir().unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".tmp_peri_test_")
            .tempdir_in(&cur)
            .unwrap();
        let ws = temp.path().join("workspace");
        fs::create_dir_all(&ws).unwrap();

        let manifest_content = r#"
[package]
name = "my_modlist"
author = "Tester"
version = "0.1.0"

[target]
game_id = "test_game"
"#;
        fs::write(ws.join(MANIFEST_FILENAME), manifest_content).unwrap();

        let run_args = RunArgs {
            workspace: ws,
            base_path: None,
            dry_run: true,
            exec: Some("echo hello".to_string()),
            proton: None,
            prefix: None,
            native: true,
            args: Vec::new(),
        };

        let result = handle_run(run_args).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Could not resolve base game directory"));
        assert!(err_msg.contains("--base-path"));
        assert!(err_msg.contains("PERI_BASE_GAME"));
    }
}
