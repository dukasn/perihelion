use std::path::PathBuf;

use clap::Parser;
use miette::Result;
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
    #[arg(short, long, default_value = "~/.config/perihelion/cache/mods")]
    cache_dir: PathBuf,

    /// Maximum upload speed in KB/s (0 for unlimited)
    #[arg(short, long, default_value_t = 1024)]
    upload_limit: u32,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    let expanded_cache = shellexpand::tilde(&args.cache_dir.to_string_lossy()).into_owned();
    let cache_path = PathBuf::from(expanded_cache);

    if !cache_path.exists() {
        warn!(
            path = %cache_path.display(),
            "Cache directory does not exist. Nothing to seed."
        );
        return Ok(());
    }

    info!("🚀 Starting perid - Perihelion Seeding Daemon");
    info!(cache = %cache_path.display(), "Monitoring cache");
    info!(
        limit_kbs = args.upload_limit,
        "Upload rate limit configured"
    );

    // 2. Initialize the P2P Engine from our peri-net adapter
    let config = SeedingConfig {
        upload_limit_kbs: args.upload_limit,
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
