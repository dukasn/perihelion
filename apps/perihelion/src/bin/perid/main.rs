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

use std::path::{Path, PathBuf};

use clap::{Parser, ValueHint};
use directories::ProjectDirs;
use helion::cli_styles;
use miette::Result;
use peri_core::{validate_safe_path, GlobalConfig, CONFIG_FILENAME};
use peri_net::torrent::{SeedingConfig, TorrentEngine};
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    name = "perid",
    author = "Eduardo Marques Santos and Perihelion Contributors",
    version,
    about = "Background P2P seeding daemon for Perihelion package archives",
    long_about = "perid - Background P2P Seeding Daemon for Perihelion.\n\n\
                  Monitors the local cache directory and seeds completed .peca\n\
                  archives back to the BitTorrent DHT network. Designed to run as an\n\
                  unobtrusive background process or systemd user service.",
    styles = cli_styles(),
    color = clap::ColorChoice::Auto,
    after_help = "EXAMPLES:\n  \
                  perid\n  \
                  perid --upload-limit 500\n  \
                  perid --cache-dir ~/.config/perihelion/cache/mods -v"
)]
struct Args {
    /// Custom path to the Perihelion package cache directory
    #[arg(
        short,
        long,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        env = "PERID_CACHE_DIR",
        help_heading = "Storage Options"
    )]
    cache_dir: Option<PathBuf>,

    /// Maximum upload bandwidth throttle in KB/s (0 for uncapped upstream)
    #[arg(
        short,
        long,
        value_name = "KBPS",
        env = "PERID_UPLOAD_LIMIT",
        help_heading = "Bandwidth Options"
    )]
    upload_limit: Option<u32>,

    /// Enable verbose debug and peer tracing logs
    #[arg(short, long, global = true, help_heading = "Logging")]
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

fn init_tracing(verbose: bool) {
    let default_filter = if verbose {
        "perid=debug,peri_net=debug,info"
    } else {
        "perid=warn,peri_net=warn,warn"
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
    let args = Args::parse();
    let global_config = load_global_config();

    // 1. Initialize logging
    init_tracing(args.verbose);

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

    info!("Starting perid - Perihelion Seeding Daemon");
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
                info!(mod_archive = %filename, "Seeding archive");
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
        "Active seeder: serving .peca archives over BitTorrent DHT"
    );

    // 4. Wait for termination signals (SIGINT / SIGTERM)
    wait_for_shutdown().await;

    info!("Shutting down perid gracefully...");
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
