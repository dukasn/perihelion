use std::path::{Path, PathBuf};

use clap::Parser;
use directories::ProjectDirs;
use miette::Result;
use peri_core::{validate_safe_path, GlobalConfig, CONFIG_FILENAME};
use peri_net::torrent::{SeedingConfig, TorrentEngine};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(
    name = "perid",
    author,
    version,
    about = "Perihelion Background Seeding Daemon"
)]
struct Args {
    /// Path to the Perihelion cache directory
    #[arg(short, long)]
    cache_dir: Option<PathBuf>,

    /// Maximum upload speed in KB/s (0 for unlimited)
    #[arg(short, long)]
    upload_limit: Option<u32>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,
}

fn load_global_config() -> GlobalConfig {
    let config_dir = if let Some(proj_dirs) = ProjectDirs::from("com", "perihelion", "perihelion") {
        proj_dirs.config_dir().to_path_buf()
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/perihelion")
    } else {
        PathBuf::from(".config/perihelion")
    };
    let config_file = config_dir.join(CONFIG_FILENAME);
    if config_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(cfg) = GlobalConfig::from_toml_str(&content) {
                return cfg;
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        GlobalConfig::default_for_home(Path::new(&home))
    } else {
        GlobalConfig::default()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let global_config = load_global_config();

    // 1. Initialize logging
    let max_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(max_level)
        .with_target(false)
        .without_time()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let raw_cache = args
        .cache_dir
        .unwrap_or(global_config.storage.global_cache_dir);
    let expanded_cache = shellexpand::tilde(&raw_cache.to_string_lossy()).into_owned();
    let cache_path = PathBuf::from(expanded_cache);

    // Sandbox boundary validation: Cache must reside within user's home
    if let Ok(home) = std::env::var("HOME") {
        validate_safe_path(&cache_path, Path::new(&home))
            .map_err(|e| miette::miette!("Security violation: {}", e))?;
    }

    if !cache_path.exists() {
        warn!(
            path = %cache_path.display(),
            "Cache directory does not exist. Nothing to seed."
        );
        return Ok(());
    }

    let upload_limit = args
        .upload_limit
        .unwrap_or(global_config.network.daemon_upload_limit_kbs);

    info!("🚀 Starting perid - Perihelion Seeding Daemon");
    info!(cache = %cache_path.display(), "Monitoring cache");
    info!(limit_kbs = upload_limit, "Upload rate limit configured");

    // 2. Initialize the P2P Engine from our peri-net adapter
    let config = SeedingConfig {
        upload_limit_kbs: upload_limit,
    };

    let engine = TorrentEngine::new_with_download_dir(&cache_path, config)
        .await
        .map_err(|e| miette::miette!("Failed to start P2P engine: {}", e))?;

    // 3. Scan the cache and start seeding
    let mut seeded_count = 0;
    if let Ok(entries) = std::fs::read_dir(&cache_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("peca") {
                let filename = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown.peca");
                info!(mod_archive = %filename, "🌱 Seeding archive");
                if let Err(e) = engine.seed_file(&path).await {
                    warn!(error = %e, mod_archive = %filename, "Failed to register archive for seeding");
                } else {
                    seeded_count += 1;
                }
            }
        }
    }

    info!(
        count = seeded_count,
        "✅ Active seeder: serving .peca archives over BitTorrent DHT"
    );

    // 4. Wait for termination signals (SIGINT / SIGTERM)
    wait_for_shutdown().await;

    info!("🛑 Shutting down perid gracefully...");
    Ok(())
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received interrupt signal (SIGINT)");
            }
            _ = sigterm.recv() => {
                info!("Received termination signal (SIGTERM)");
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        info!("Received interrupt signal (SIGINT)");
    }
}
