use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use miette::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod error;

use cli::{CheckArgs, Cli, Commands, GetArgs, InitArgs, PackageArgs, RunArgs, SyncArgs};
use error::CliError;
use peri_archive::{
    compute_file_sha256, create_peca, extract_peca, list_peca_files, read_manifest_from_peca,
};
use peri_core::{
    DependencyGraph, GameProfile, Manifest, OverrideRule, PackageMetadata, ResolutionSource,
    TargetGame, VirtualFileIndex, MANIFEST_FILENAME,
};
use peri_vfs::VfsPlan;

const CACHE_MODS_DIR: &str = ".perihelion/cache/mods";
const EXTRACTED_DIR: &str = ".perihelion/extracted";
const UPPER_DIR: &str = ".perihelion/upper";
const WORK_DIR: &str = ".perihelion/work";
const MOUNT_DIR: &str = ".perihelion/mnt";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .without_time()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    match cli.command {
        Commands::Init(args) => handle_init(args).await?,
        Commands::Get(args) => handle_get(args).await?,
        Commands::Sync(args) => handle_sync(args).await?,
        Commands::Package(args) => handle_package(args).await?,
        Commands::Check(args) => handle_check(args).await?,
        Commands::Run(args) => handle_run(args).await?,
    }

    Ok(())
}

async fn handle_init(args: InitArgs) -> Result<(), CliError> {
    let workspace_path = args.path;
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

    // Create .perihelion cache and runtime state directories
    fs::create_dir_all(workspace_path.join(CACHE_MODS_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(EXTRACTED_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(UPPER_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(WORK_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;
    fs::create_dir_all(workspace_path.join(MOUNT_DIR))
        .map_err(|e| CliError::Workspace(e.to_string()))?;

    println!(
        "✨ Initialized Perihelion workspace for game '{}' at {}",
        args.game_id,
        workspace_path
            .canonicalize()
            .unwrap_or(workspace_path)
            .display()
    );

    Ok(())
}

async fn handle_package(args: PackageArgs) -> Result<(), CliError> {
    let source_dir = args.source;
    let manifest_path = source_dir.join(MANIFEST_FILENAME);

    if !manifest_path.exists() {
        return Err(CliError::Archive(format!(
            "Missing '{}' in source directory {}",
            MANIFEST_FILENAME,
            source_dir.display()
        )));
    }

    let manifest_content =
        fs::read_to_string(&manifest_path).map_err(|e| CliError::Manifest(e.to_string()))?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    let output_path = args.output.unwrap_or_else(|| {
        let filename = format!(
            "{}-{}.peca",
            manifest.package.name.replace(' ', "_"),
            manifest.package.version
        );
        source_dir.join(filename)
    });

    info!(source = %source_dir.display(), target = %output_path.display(), "Packaging .peca archive");

    let packaged_manifest = create_peca(&source_dir, &output_path, args.level)?;
    let sha256 = compute_file_sha256(&output_path)?;

    println!("📦 Successfully packaged .peca archive:");
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
        info!(url = %uri, "Downloading archive over HTTP");
        peri_net::fetch_http(uri, &dest_file, None).await?;
    } else if uri.starts_with("magnet:") {
        let filename = args.output.unwrap_or_else(|| PathBuf::from("torrent.peca"));
        dest_file = cache_dir.join(filename);
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
        fs::copy(source_path, &dest_file).map_err(|e| CliError::Archive(e.to_string()))?;
    }

    let manifest = read_manifest_from_peca(&dest_file)?;
    println!(
        "📥 Successfully cached mod '{}' v{} -> {}",
        manifest.package.name,
        manifest.package.version,
        dest_file.display()
    );

    Ok(())
}

async fn handle_sync(args: SyncArgs) -> Result<(), CliError> {
    let workspace = args.workspace;
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
    let mut overrides: HashMap<String, OverrideRule> = HashMap::new();
    let mut game_profile = GameProfile::generic("generic");

    if root_manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&root_manifest_path) {
            if let Ok(root_manifest) = Manifest::from_toml_str(&content) {
                overrides = root_manifest.overrides.into_iter().collect();
                game_profile = if root_manifest.target.game_id == "cyberpunk_2077" {
                    GameProfile::cyberpunk_2077()
                } else {
                    GameProfile::generic(&root_manifest.target.game_id)
                };
            }
        }
    }

    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();
    let mut peca_files: Vec<(Manifest, PathBuf)> = Vec::new();

    for entry in fs::read_dir(&cache_dir).map_err(|e| CliError::Workspace(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::Workspace(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("peca") {
            let manifest = read_manifest_from_peca(&path)?;
            let files = list_peca_files(&path)?;
            for file in files {
                let mapped = game_profile.map_path(Path::new(&file));
                vfi.add_entry(
                    mapped.to_string_lossy(),
                    &manifest.package.name,
                    manifest.package.priority,
                );
            }
            graph.add_manifest(manifest.clone());
            peca_files.push((manifest, path));
        }
    }

    // Apply the 3-Tier Resolution Waterfall
    let _reports = vfi.resolve_conflicts_waterfall(&mut graph, &overrides)?;

    // Validate DAG and resolve order
    let resolved_order = graph.resolve_load_order()?;
    println!("🔄 Resolving load order and synchronizing VFS layers...");

    for name in &resolved_order {
        if let Some((manifest, peca_path)) =
            peca_files.iter().find(|(m, _)| &m.package.name == name)
        {
            let dest_layer = extracted_dir.join(&manifest.package.name);
            extract_peca(peca_path, &dest_layer)?;
            println!(
                "   ✓ Unpacked {} v{} -> {}",
                manifest.package.name,
                manifest.package.version,
                dest_layer.display()
            );
        }
    }

    println!(
        "✨ Workspace synchronization complete: {} mods active.",
        resolved_order.len()
    );
    Ok(())
}

async fn handle_check(args: CheckArgs) -> Result<(), CliError> {
    let workspace = args.workspace;
    let cache_dir = workspace.join(CACHE_MODS_DIR);

    if !cache_dir.exists() {
        println!("ℹ️  No mods cached in {}", cache_dir.display());
        return Ok(());
    }

    let root_manifest_path = workspace.join(MANIFEST_FILENAME);
    let mut overrides: HashMap<String, OverrideRule> = HashMap::new();
    let mut game_profile = GameProfile::generic("generic");

    if root_manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&root_manifest_path) {
            if let Ok(root_manifest) = Manifest::from_toml_str(&content) {
                overrides = root_manifest.overrides.into_iter().collect();
                game_profile = if root_manifest.target.game_id == "cyberpunk_2077" {
                    GameProfile::cyberpunk_2077()
                } else {
                    GameProfile::generic(&root_manifest.target.game_id)
                };
            }
        }
    }

    let mut graph = DependencyGraph::new();
    let mut vfi = VirtualFileIndex::new();
    let mut count = 0;

    for entry in fs::read_dir(&cache_dir).map_err(|e| CliError::Workspace(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::Workspace(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("peca") {
            let manifest = read_manifest_from_peca(&path)?;
            let files = list_peca_files(&path)?;
            for file in files {
                let mapped = game_profile.map_path(Path::new(&file));
                vfi.add_entry(
                    mapped.to_string_lossy(),
                    &manifest.package.name,
                    manifest.package.priority,
                );
            }
            graph.add_manifest(manifest);
            count += 1;
        }
    }

    println!(
        "🔍 Validating dependency DAG and Virtual File Index (VFI) for {} packages...",
        count
    );

    // Apply the 3-Tier Resolution Waterfall
    let reports = vfi.resolve_conflicts_waterfall(&mut graph, &overrides)?;
    if !reports.is_empty() {
        println!(
            "⚡ Detected and resolved {} file conflict(s) via 3-Tier Waterfall:",
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
        println!("✨ Virtual File Index check clean: no file conflicts detected.");
    }

    let order = graph.resolve_load_order()?;
    println!("✅ DAG check passed successfully. No cycles detected!");
    println!("📋 Calculated Deterministic Load Order (OverlayFS stack):");
    for (idx, mod_name) in order.iter().enumerate() {
        println!("   [{}] {}", idx + 1, mod_name);
    }

    Ok(())
}

async fn handle_run(args: RunArgs) -> Result<(), CliError> {
    let workspace = args.workspace;
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

    let profile = if manifest.target.game_id == "cyberpunk_2077" {
        GameProfile::cyberpunk_2077()
    } else {
        GameProfile::generic(&manifest.target.game_id)
    };

    let cache_dir = workspace.join(CACHE_MODS_DIR);
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
                let files = list_peca_files(&path)?;
                for file in files {
                    let mapped = profile.map_path(Path::new(&file));
                    vfi.add_entry(
                        mapped.to_string_lossy(),
                        &m.package.name,
                        m.package.priority,
                    );
                }
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

    let base_dir = workspace.join("base_game");
    let mount_point = workspace.join(MOUNT_DIR);
    let upper_dir = workspace.join(UPPER_DIR);
    let work_dir = workspace.join(WORK_DIR);

    let plan = VfsPlan::new(base_dir, mount_point)
        .with_layers(layers)
        .with_upper(upper_dir, work_dir);

    let mount_options = plan.format_mount_options()?;
    let exec_cmd = args
        .exec
        .or(profile.default_executable)
        .unwrap_or_else(|| "echo 'No executable configured'".to_string());

    println!("🚀 Preparing Perihelion Virtual File System sandbox:");
    println!("   Game:          {} ({})", profile.name, profile.id);
    println!("   Active Layers: {}", resolved_order.len());
    println!("   Mount Point:   {}", plan.mount_point.display());
    println!("   Mount Options: {}", mount_options);
    println!("   Command:       {} {:?}", exec_cmd, args.args);

    if args.dry_run {
        println!("🔎 [DRY-RUN] Verification complete. Sandboxed mount ready.");
        return Ok(());
    }

    let exit_code = peri_vfs::execute_in_isolated_namespace(&plan, &exec_cmd, &args.args)?;
    info!(exit_code = exit_code, "Execution finished");

    Ok(())
}
