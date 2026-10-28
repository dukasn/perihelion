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
    CheckArgs, CiFlags, Cli, CliFormat, Commands, ConfigAction, ConfigArgs, GetArgs, InitArgs,
    PackageArgs, RunArgs, SyncArgs,
};
use config::{load_global_config, resolve_config_dir, resolve_home_dir};
use error::CliError;
use peri_archive::{
    compute_file_sha256, create_peca, extract_peca_layer, generate_package_file_index,
    load_package_vfi_cache, read_manifest_from_peca, save_package_vfi_cache,
};
use peri_core::{
    solve_workspace, validate_safe_path, CheckReport, DependencyGraph, DryRunPlanReport,
    FileConflictSummary, GameProfile, Lockfile, Manifest, OverrideRule, PackageCatalog,
    PackageFileIndex, PackageMetadata, Priority, ResolutionSource, ResolvedPackageSummary,
    TargetGame, VirtualFileIndex, LOCKFILE_FILENAME, MANIFEST_FILENAME,
};
use peri_vfs::{CommandWrapper, RunnerKind, RunnerPipeline, VfsPlan};
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
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .compact(),
        )
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    let ci_flags = CiFlags::from_cli(&cli);

    if let Err(err) = run_command(cli.command, &ci_flags).await {
        let code = err.exit_code();
        eprintln!("{err}");
        std::process::exit(code);
    }

    Ok(())
}

async fn run_command(command: Commands, ci_flags: &CiFlags) -> Result<(), CliError> {
    match command {
        Commands::Init(args) => handle_init(args).await,
        Commands::Get(args) => handle_get(args).await,
        Commands::Sync(args) => handle_sync(args, ci_flags).await,
        Commands::Package(args) => handle_package(args).await,
        Commands::Check(args) => handle_check(args, ci_flags).await,
        Commands::Run(args) => handle_run(args, ci_flags).await,
        Commands::Config(args) => handle_config(args).await,
        Commands::Profile(args) => profile::handle_profile(args, ci_flags).await,
    }
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

async fn handle_sync(args: SyncArgs, ci_flags: &CiFlags) -> Result<(), CliError> {
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

    let existing_lockfile: Option<Lockfile> = if lockfile_path.exists() {
        let content =
            fs::read_to_string(&lockfile_path).map_err(|e| CliError::Workspace(e.to_string()))?;
        Some(Lockfile::from_toml_str(&content)?)
    } else {
        None
    };

    // If --locked or --frozen is active, verify lockfile drift before doing any work
    if ci_flags.locked {
        if let Some(ref root_manifest) = root_manifest_opt {
            peri_core::lockfile::verify_lockfile_sync(root_manifest, existing_lockfile.as_ref())?;
        } else if existing_lockfile.is_none() {
            return Err(peri_core::LockfileError::MissingLockfile.into());
        }
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

    // If --offline or --frozen is active, ensure all required packages are present in local cache
    if ci_flags.offline {
        if let Some(ref root_manifest) = root_manifest_opt {
            for dep_name in root_manifest.dependencies.keys() {
                let found = peca_files.iter().any(|(m, _)| &m.package.name == dep_name);
                if !found {
                    return Err(CliError::OfflineNetworkRequired {
                        package: dep_name.clone(),
                    });
                }
            }
        }
    }

    // Stage 1: Version Selection (PubGrub CDCL SAT Solver & peri.lock)
    let selected_versions = if let Some(ref root_manifest) = root_manifest_opt {
        if !root_manifest.dependencies.is_empty() {
            let mut use_lock = false;
            let mut resolved = BTreeMap::new();

            if let Some(ref lock) = existing_lockfile {
                if lock.satisfies_requirements(&root_manifest.dependencies) {
                    resolved = lock.package_map();
                    use_lock = true;
                    info!("Reusing locked package versions from peri.lock");
                }
            }

            if !use_lock {
                if ci_flags.locked {
                    return Err(peri_core::LockfileError::MissingLockfile.into());
                }
                info!("Running PubGrub CDCL SAT solver for dependencies...");
                resolved = solve_workspace(&catalog, root_manifest)?;
                let lockfile = Lockfile::from_solution(&resolved);
                let lock_toml = lockfile.to_toml_str()?;
                fs::write(&lockfile_path, lock_toml)
                    .map_err(|e| CliError::Workspace(e.to_string()))?;
                if ci_flags.format == CliFormat::Text {
                    println!("Generated updated {}", LOCKFILE_FILENAME);
                }
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

    // If offline, check that all selected packages actually exist in active_peca_files
    if ci_flags.offline {
        if let Some(ref selected) = selected_versions {
            for pkg_name in selected.keys() {
                if !active_peca_files
                    .iter()
                    .any(|(m, _)| &m.package.name == pkg_name)
                {
                    return Err(CliError::OfflineNetworkRequired {
                        package: pkg_name.clone(),
                    });
                }
            }
        }
    }

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
            // Drop any previous extraction so remapped or removed files don't linger.
            if dest_layer.exists() {
                fs::remove_dir_all(&dest_layer).map_err(|e| CliError::Workspace(e.to_string()))?;
            }
            extract_peca_layer(peca_path, &dest_layer, |p| game_profile.map_path(p))?;
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

async fn handle_check(args: CheckArgs, ci_flags: &CiFlags) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let span = info_span!("check", workspace = %workspace.display());
    let _enter = span.enter();

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

    if ci_flags.locked {
        let existing_lockfile: Option<Lockfile> = if lockfile_path.exists() {
            let content = fs::read_to_string(&lockfile_path)
                .map_err(|e| CliError::Workspace(e.to_string()))?;
            Some(Lockfile::from_toml_str(&content)?)
        } else {
            None
        };
        if let Some(ref root_manifest) = root_manifest_opt {
            peri_core::lockfile::verify_lockfile_sync(root_manifest, existing_lockfile.as_ref())?;
        } else if existing_lockfile.is_none() {
            return Err(peri_core::LockfileError::MissingLockfile.into());
        }
    }

    let cache_dir = workspace.join(CACHE_MODS_DIR);
    if !cache_dir.exists() {
        if ci_flags.format == CliFormat::Json {
            let report = CheckReport {
                success: true,
                game_id: game_profile.id.clone(),
                package_count: 0,
                active_layers: Vec::new(),
                file_conflicts: Vec::new(),
                has_unresolved_conflicts: false,
            };
            let json_str = serde_json::to_string_pretty(&report)
                .map_err(|e| CliError::Workspace(e.to_string()))?;
            println!("{json_str}");
        } else {
            println!("No mods cached in {}", cache_dir.display());
        }
        return Ok(());
    }

    let mut catalog = PackageCatalog::new();
    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();
    let mut count = 0;
    let vfi_cache_dir = workspace.join(VFI_CACHE_DIR);
    let extracted_dir = workspace.join(EXTRACTED_DIR);
    let mut cached_manifests: HashMap<String, Manifest> = HashMap::new();

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
            graph.add_manifest(manifest.clone());
            cached_manifests.insert(manifest.package.name.clone(), manifest);
            count += 1;
        }
    }

    // Stage 1: Validate version constraints with PubGrub
    if let Some(ref root_manifest) = root_manifest_opt {
        if !root_manifest.dependencies.is_empty() {
            let solved = solve_workspace(&catalog, root_manifest)?;
            if ci_flags.format == CliFormat::Text {
                println!(
                    "Stage 1: PubGrub SAT solver verified {} version constraints.",
                    solved.len()
                );
            }
        }
    }

    if ci_flags.format == CliFormat::Text {
        println!(
            "Stage 2 & 3: Validating dependency DAG and Virtual File Index (VFI) for {} packages...",
            count
        );
    }

    // Apply the 3-Tier Resolution Waterfall
    let reports_result = vfi.resolve_conflicts_waterfall(&mut graph, &overrides);
    let reports = match reports_result {
        Ok(r) => r,
        Err(e) => {
            if ci_flags.format == CliFormat::Json {
                let (file, mod_a, mod_b) = match &e {
                    peri_core::ResolveError::HardConflict {
                        file, mod_a, mod_b, ..
                    } => (file.clone(), mod_a.clone(), mod_b.clone()),
                    _ => (String::new(), String::new(), String::new()),
                };
                let report = CheckReport {
                    success: false,
                    game_id: game_profile.id.clone(),
                    package_count: count,
                    active_layers: Vec::new(),
                    file_conflicts: vec![FileConflictSummary {
                        relative_path: file,
                        winning_package: String::new(),
                        shadowed_packages: vec![mod_a, mod_b],
                        resolution_tier: "unresolved".to_string(),
                    }],
                    has_unresolved_conflicts: true,
                };
                if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                    println!("{json_str}");
                }
            }
            return Err(e.into());
        }
    };

    let order = graph.resolve_load_order()?;

    if ci_flags.format == CliFormat::Json {
        let active_layers: Vec<ResolvedPackageSummary> = order
            .iter()
            .map(|name| {
                let (version, priority) = cached_manifests
                    .get(name)
                    .map(|m| (m.package.version.clone(), m.package.priority))
                    .unwrap_or_else(|| ("unknown".to_string(), Priority::Default));
                ResolvedPackageSummary {
                    name: name.clone(),
                    version,
                    priority,
                    path_in_lowerdir: Some(extracted_dir.join(name)),
                }
            })
            .collect();

        let file_conflicts: Vec<FileConflictSummary> = reports
            .iter()
            .map(|r| {
                let (winner, resolution_tier) = match &r.source {
                    ResolutionSource::ExplicitEdge { winner } => {
                        (winner.clone(), "explicit_edge".to_string())
                    }
                    ResolutionSource::PriorityWeight { winner, .. } => {
                        (winner.clone(), "priority_weight".to_string())
                    }
                };
                let shadowed = if r.mod_a == winner {
                    vec![r.mod_b.clone()]
                } else {
                    vec![r.mod_a.clone()]
                };
                FileConflictSummary {
                    relative_path: r.file_path.clone(),
                    winning_package: winner,
                    shadowed_packages: shadowed,
                    resolution_tier,
                }
            })
            .collect();

        let report = CheckReport {
            success: true,
            game_id: game_profile.id.clone(),
            package_count: count,
            active_layers,
            file_conflicts,
            has_unresolved_conflicts: false,
        };
        let json_str = serde_json::to_string_pretty(&report)
            .map_err(|e| CliError::Workspace(e.to_string()))?;
        println!("{json_str}");
        return Ok(());
    }

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

    println!("DAG check passed successfully. No cycles detected!");
    println!("Calculated Deterministic Load Order (OverlayFS stack):");
    for (idx, mod_name) in order.iter().enumerate() {
        println!("   [{}] {}", idx + 1, mod_name);
    }

    Ok(())
}

async fn handle_run(args: RunArgs, ci_flags: &CiFlags) -> Result<(), CliError> {
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
        .with_layers(layers.clone())
        .with_upper(upper_dir.clone(), work_dir.clone());

    let mount_options = plan.format_mount_options()?;
    let exec_cmd = args
        .exec
        .or(profile.default_executable)
        .unwrap_or_else(|| "echo 'No executable configured'".to_string());

    // 1. Assemble wrapper pipeline
    let mut wrappers = Vec::new();
    if args.gamemode {
        wrappers.push(CommandWrapper::gamemode());
    }
    if args.mangohud {
        wrappers.push(CommandWrapper::mangohud());
    }
    if let Some(ref gs_opts) = args.gamescope {
        let custom_opts = if gs_opts.is_empty() {
            None
        } else {
            Some(gs_opts.as_str())
        };
        wrappers.push(CommandWrapper::gamescope(custom_opts));
    }
    for raw_wrapper in &args.wrappers {
        if let Some(wrapper) = CommandWrapper::parse(raw_wrapper) {
            wrappers.push(wrapper);
        }
    }

    // 2. Resolve compatibility runner
    let runner_kind = if args.native {
        RunnerKind::Native
    } else if let Some(ref wine_bin) = args.wine {
        let wine_prefix = if let Some(ref pfx) = args.prefix {
            pfx.clone()
        } else if let Ok(env_pfx) = std::env::var("WINEPREFIX") {
            PathBuf::from(env_pfx)
        } else if let Some(app_id) = profile.steam_app_id {
            let steam_root =
                peri_vfs::SteamDiscovery::find_active_steam_root_with_config(&provider)
                    .unwrap_or_else(|| {
                        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                            .join(".local/share/Steam")
                    });
            steam_root.join(format!("steamapps/compatdata/{app_id}/pfx"))
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".wine")
        };
        RunnerKind::Wine {
            runner: wine_bin.clone(),
            prefix: wine_prefix,
        }
    } else {
        let use_proton = profile.steam_app_id.is_some()
            || args.proton.is_some()
            || args.prefix.is_some()
            || exec_cmd.to_lowercase().ends_with(".exe");

        if use_proton {
            match peri_vfs::SteamDiscovery::resolve_proton_launch_with_config(
                profile.steam_app_id,
                args.proton.as_deref(),
                args.prefix.as_deref(),
                &provider,
            ) {
                Ok(launch) => RunnerKind::Proton(launch),
                Err(e) => {
                    if args.proton.is_some() || args.prefix.is_some() {
                        return Err(CliError::Vfs(e.to_string()));
                    }
                    info!(
                        "Proton auto-discovery did not find an active installation ({e}); falling back to native runner"
                    );
                    RunnerKind::Native
                }
            }
        } else {
            RunnerKind::Native
        }
    };

    // 3. Assemble environment variables
    let mut env_vars = Vec::new();
    if args.mangohud {
        env_vars.push(("MANGOHUD".to_string(), "1".to_string()));
    }

    // 4. Construct complete RunnerPipeline
    let pipeline = RunnerPipeline::new(&exec_cmd)
        .with_runner(runner_kind)
        .with_args(args.args.clone())
        .with_wrappers(wrappers)
        .with_envs(env_vars);

    if args.dry_run {
        if ci_flags.format == CliFormat::Json {
            let mut lowerdir: Vec<PathBuf> = layers.iter().rev().cloned().collect();
            lowerdir.push(base_dir.clone());

            let mut env_map = HashMap::new();
            for (k, v) in pipeline.collected_env_vars() {
                env_map.insert(k, v);
            }

            let report = DryRunPlanReport {
                game_id: profile.id.clone(),
                base_game_path: base_dir.clone(),
                mount_point: plan.mount_point.clone(),
                lowerdir,
                upper_dir: upper_dir.clone(),
                work_dir: work_dir.clone(),
                mount_options_string: mount_options.clone(),
                executable: PathBuf::from(&exec_cmd),
                arguments: args.args.clone(),
                environment: env_map,
                runner: Some(pipeline.runner_name().to_string()),
                wrappers: pipeline.wrapper_names(),
                command_line: pipeline.build_argv(),
            };
            let json_str = serde_json::to_string_pretty(&report)
                .map_err(|e| CliError::Workspace(e.to_string()))?;
            println!("{json_str}");
            return Ok(());
        }

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

        println!("Runner Pipeline:");
        match &pipeline.runner {
            RunnerKind::Native => {
                println!("   Compatibility: Native Linux process");
            }
            RunnerKind::Wine { runner, prefix } => {
                println!("   Compatibility: Wine");
                println!("   Wine Runner:   {}", runner.display());
                println!("   Wine Prefix:   {}", prefix.display());
            }
            RunnerKind::Proton(p) => {
                println!("   Compatibility: Valve Proton");
                println!("   Proton Runner: {}", p.runner.display());
                println!("   Compat Data:   {}", p.compat_data_path.display());
                println!("   Wine Prefix:   {}", p.wine_prefix.display());
            }
            RunnerKind::Custom { runner, .. } => {
                println!("   Compatibility: Custom ({})", runner.display());
            }
        }
        if !pipeline.wrappers.is_empty() {
            println!("   Wrappers:      {}", pipeline.wrapper_names().join(", "));
        }
        println!("   Full Command:  {}", pipeline.build_argv().join(" "));

        let envs = pipeline.collected_env_vars();
        if !envs.is_empty() {
            println!("   Injected environment:");
            for (k, v) in &envs {
                if k.starts_with("STEAM_")
                    || k == "WINEPREFIX"
                    || k.starts_with("Steam")
                    || k == "MANGOHUD"
                {
                    println!("             {k}={v}");
                }
            }
        }

        println!("[DRY-RUN] Verification complete. Sandboxed mount and execution ready.");
        return Ok(());
    }

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

    println!("Runner Pipeline:");
    match &pipeline.runner {
        RunnerKind::Native => {
            println!("   Compatibility: Native Linux process");
        }
        RunnerKind::Wine { runner, prefix } => {
            println!("   Compatibility: Wine");
            println!("   Wine Runner:   {}", runner.display());
            println!("   Wine Prefix:   {}", prefix.display());
        }
        RunnerKind::Proton(p) => {
            println!("   Compatibility: Valve Proton");
            println!("   Proton Runner: {}", p.runner.display());
            println!("   Compat Data:   {}", p.compat_data_path.display());
            println!("   Wine Prefix:   {}", p.wine_prefix.display());
        }
        RunnerKind::Custom { runner, .. } => {
            println!("   Compatibility: Custom ({})", runner.display());
        }
    }
    if !pipeline.wrappers.is_empty() {
        println!("   Wrappers:      {}", pipeline.wrapper_names().join(", "));
    }
    println!("   Full Command:  {}", pipeline.build_argv().join(" "));

    let exit_code = peri_vfs::execute_pipeline_in_isolated_namespace(&plan, &pipeline)?;
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
            wrappers: Vec::new(),
            gamemode: false,
            gamescope: None,
            mangohud: false,
            proton: None,
            wine: None,
            prefix: None,
            native: true,
            args: Vec::new(),
        };

        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Text,
        };

        let result = handle_run(run_args, &ci_flags).await;
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
            wrappers: Vec::new(),
            gamemode: false,
            gamescope: None,
            mangohud: false,
            proton: None,
            wine: None,
            prefix: None,
            native: true,
            args: Vec::new(),
        };

        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Text,
        };

        let result = handle_run(run_args, &ci_flags).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Could not resolve base game directory"));
        assert!(err_msg.contains("--base-path"));
        assert!(err_msg.contains("PERI_BASE_GAME"));
    }

    #[tokio::test]
    async fn test_handle_run_dry_run_json_format() {
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
            wrappers: Vec::new(),
            gamemode: false,
            gamescope: None,
            mangohud: false,
            proton: None,
            wine: None,
            prefix: None,
            native: true,
            args: vec!["--arg1".to_string()],
        };

        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Json,
        };

        let result = handle_run(run_args, &ci_flags).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_run_with_runner_pipeline_and_wrappers() {
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

        let pfx = temp.path().join("wine_prefix");
        fs::create_dir_all(&pfx).unwrap();

        let run_args = RunArgs {
            workspace: ws,
            base_path: Some(custom_game),
            dry_run: true,
            exec: Some("SkyrimSE.exe".to_string()),
            wrappers: vec!["strace -f".to_string()],
            gamemode: true,
            gamescope: Some("-w 1920 -h 1080".to_string()),
            mangohud: true,
            proton: None,
            wine: Some(PathBuf::from("/usr/bin/wine")),
            prefix: Some(pfx),
            native: false,
            args: vec!["--skip-intro".to_string()],
        };

        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Json,
        };

        let result = handle_run(run_args, &ci_flags).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_check_json_format() {
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

        let check_args = CheckArgs { workspace: ws };
        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Json,
        };

        let result = handle_check(check_args, &ci_flags).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_sync_locked_missing_lockfile_exit_code_5() {
        let cur = std::env::current_dir().unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".tmp_peri_test_")
            .tempdir_in(&cur)
            .unwrap();
        let ws = temp.path().join("workspace");
        fs::create_dir_all(ws.join(CACHE_MODS_DIR)).unwrap();

        let manifest_content = r#"
[package]
name = "my_modlist"
author = "Tester"
version = "0.1.0"

[target]
game_id = "test_game"

[dependencies]
skyui = "^5.2"
"#;
        fs::write(ws.join(MANIFEST_FILENAME), manifest_content).unwrap();

        let sync_args = SyncArgs { workspace: ws };
        let ci_flags = CiFlags {
            locked: true,
            offline: false,
            format: CliFormat::Text,
        };

        let result = handle_sync(sync_args, &ci_flags).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 5);
    }

    #[tokio::test]
    async fn test_handle_sync_offline_missing_package_exit_code_6() {
        let cur = std::env::current_dir().unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".tmp_peri_test_")
            .tempdir_in(&cur)
            .unwrap();
        let ws = temp.path().join("workspace");
        fs::create_dir_all(ws.join(CACHE_MODS_DIR)).unwrap();

        let manifest_content = r#"
[package]
name = "my_modlist"
author = "Tester"
version = "0.1.0"

[target]
game_id = "test_game"

[dependencies]
skyui = "^5.2"
"#;
        fs::write(ws.join(MANIFEST_FILENAME), manifest_content).unwrap();

        let sync_args = SyncArgs { workspace: ws };
        let ci_flags = CiFlags {
            locked: false,
            offline: true,
            format: CliFormat::Text,
        };

        let result = handle_sync(sync_args, &ci_flags).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 6);
    }

    #[tokio::test]
    async fn test_manifest_syntax_error_exit_code_2() {
        let cur = std::env::current_dir().unwrap();
        let temp = tempfile::Builder::new()
            .prefix(".tmp_peri_test_")
            .tempdir_in(&cur)
            .unwrap();
        let ws = temp.path().join("workspace");
        fs::create_dir_all(&ws).unwrap();

        fs::write(ws.join(MANIFEST_FILENAME), "invalid toml syntax [[[").unwrap();

        let check_args = CheckArgs { workspace: ws };
        let ci_flags = CiFlags {
            locked: false,
            offline: false,
            format: CliFormat::Text,
        };

        let result = handle_check(check_args, &ci_flags).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_exit_codes_mapping() {
        let hard_conflict = peri_core::ResolveError::HardConflict {
            file: "Data/file.txt".to_string(),
            mod_a: "mod_a".to_string(),
            mod_b: "mod_b".to_string(),
            priority: "default".to_string(),
        };
        assert_eq!(CliError::Resolve(hard_conflict).exit_code(), 3);

        let cycle = peri_core::ResolveError::CyclicDependencyDetected("a -> b -> a".to_string());
        assert_eq!(CliError::Resolve(cycle).exit_code(), 4);

        let lock_err = peri_core::LockfileError::MissingLockfile;
        assert_eq!(CliError::Lockfile(lock_err).exit_code(), 5);

        let offline_err = CliError::OfflineNetworkRequired {
            package: "foo".to_string(),
        };
        assert_eq!(offline_err.exit_code(), 6);

        let io_err = CliError::Workspace("io failed".to_string());
        assert_eq!(io_err.exit_code(), 1);
    }
}
