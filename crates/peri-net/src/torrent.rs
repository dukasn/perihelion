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

#[cfg(feature = "bittorrent")]
pub use bittorrent::*;

#[cfg(feature = "bittorrent")]
pub mod bittorrent {
    use std::num::NonZeroU32;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use librqbit::limits::LimitsConfig;
    use librqbit::{AddTorrent, AddTorrentOptions, CreateTorrentOptions, Session, SessionOptions};
    use tracing::info;

    use crate::NetError;

    /// Configuration for background and P2P seeding sessions.
    #[derive(Debug, Clone, Default)]
    pub struct SeedingConfig {
        /// Maximum upload limit in KB/s (0 = unlimited).
        pub upload_limit_kbs: u32,
    }

    /// High-level BitTorrent engine wrapping a `librqbit::Session`.
    pub struct TorrentEngine {
        session: Arc<Session>,
    }

    impl TorrentEngine {
        /// Initializes a new torrent engine with the given seeding configuration.
        pub async fn new(config: SeedingConfig) -> Result<Self, NetError> {
            let default_dir = match std::env::var("HOME") {
                Ok(home) => PathBuf::from(home).join(".config/perihelion/cache/mods"),
                Err(_) => std::env::temp_dir().join("perihelion_seeding"),
            };
            Self::new_with_download_dir(&default_dir, config).await
        }

        /// Initializes a new torrent engine with custom download/storage directory.
        pub async fn new_with_download_dir(
            download_dir: &Path,
            config: SeedingConfig,
        ) -> Result<Self, NetError> {
            let mut opts = SessionOptions::default();
            if config.upload_limit_kbs > 0 {
                let bytes_per_sec = config.upload_limit_kbs.saturating_mul(1024);
                if let Some(nonzero) = NonZeroU32::new(bytes_per_sec) {
                    opts.ratelimits = LimitsConfig {
                        upload_bps: Some(nonzero),
                        download_bps: None,
                    };
                }
            }

            let session = Session::new_with_opts(download_dir.to_path_buf(), opts)
                .await
                .map_err(|e| NetError::P2pError(e.to_string()))?;

            Ok(Self { session })
        }

        /// Returns a reference to the inner `librqbit::Session`.
        pub fn session(&self) -> &Arc<Session> {
            &self.session
        }

        /// Generates torrent metadata for a local file and begins seeding it to the BitTorrent DHT.
        pub async fn seed_file(&self, path: &Path) -> Result<(), NetError> {
            info!(path = ?path, "Registering archive for P2P seeding");
            let opts = CreateTorrentOptions {
                name: None,
                trackers: Vec::new(),
                piece_length: None,
            };
            self.session
                .create_and_serve_torrent(path, opts)
                .await
                .map_err(|e| NetError::P2pError(e.to_string()))?;
            Ok(())
        }

        /// Downloads an archive from a magnet URI to the session storage.
        pub async fn download_magnet(&self, magnet: &str) -> Result<(), NetError> {
            download_magnet(&self.session, magnet).await
        }
    }

    /// Legacy / lower-level helper to create an unthrottled session.
    pub async fn create_torrent_session(download_dir: &Path) -> Result<Arc<Session>, NetError> {
        let opts = SessionOptions::default();
        let session = Session::new_with_opts(download_dir.to_path_buf(), opts)
            .await
            .map_err(|e| NetError::P2pError(e.to_string()))?;
        Ok(session)
    }

    /// Lower-level helper to add a magnet link to an active session and await completion.
    pub async fn download_magnet(session: &Arc<Session>, magnet: &str) -> Result<(), NetError> {
        info!(magnet = %magnet, "Adding magnet link to session");
        let add = AddTorrent::from_url(magnet);
        let opts = AddTorrentOptions {
            paused: false,
            ..Default::default()
        };
        let handle = session
            .add_torrent(add, Some(opts))
            .await
            .map_err(|e| NetError::P2pError(e.to_string()))?;

        // Wait until download completes
        let response = handle
            .into_handle()
            .ok_or_else(|| NetError::P2pError("Torrent handle unavailable".to_string()))?;

        response
            .wait_until_completed()
            .await
            .map_err(|e| NetError::P2pError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(all(test, feature = "bittorrent"))]
mod tests {
    use super::bittorrent::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_torrent_engine_initialization_and_seeding() {
        let tmp = tempdir().unwrap();
        let config = SeedingConfig {
            upload_limit_kbs: 512,
        };
        let engine = TorrentEngine::new_with_download_dir(tmp.path(), config)
            .await
            .expect("should initialize engine");

        // Create a dummy file to seed
        let file_path = tmp.path().join("test_mod.peca");
        std::fs::write(&file_path, b"dummy mod content for test seeding").unwrap();

        let seed_res = engine.seed_file(&file_path).await;
        assert!(seed_res.is_ok(), "should register file for seeding");
    }
}
