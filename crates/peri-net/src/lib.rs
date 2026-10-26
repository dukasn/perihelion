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

//! Perihelion Network Adapter (`peri-net`)
//!
//! Handles decentralized (BitTorrent) and direct (HTTP) package fetching,
//! with automated SHA-256 integrity validation and fallback strategies.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use peri_core::Assets;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, info, info_span, warn};

pub mod stream;
pub mod torrent;

pub use stream::{
    bridge_to_sync, fetch_http_stream, response_to_async_read, BoxedAsyncRead, PieceStreamReceiver,
};

#[derive(Debug, Error)]
pub enum NetError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Checksum mismatch for '{path}': expected {expected}, calculated {calculated}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        calculated: String,
    },

    #[error("No download source available in manifest assets")]
    NoSourceAvailable,

    #[error("P2P download error: {0}")]
    P2pError(String),
}

/// Downloads a file over HTTP(S), verifying its SHA-256 checksum if provided.
pub async fn fetch_http(
    url: &str,
    output_path: &Path,
    expected_sha256: Option<&str>,
) -> Result<(), NetError> {
    let span = info_span!("fetch_http", url = %url, path = %output_path.display());
    let _enter = span.enter();
    info!("Initiating HTTP download");

    let client = reqwest::Client::builder().build()?;
    let response = client.get(url).send().await?.error_for_status()?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(output_path)?;
    let mut hasher = Sha256::new();
    let bytes = response.bytes().await?;

    hasher.update(&bytes);
    file.write_all(&bytes)?;
    file.flush()?;

    let calculated_hash = hex::encode(hasher.finalize());

    if let Some(expected) = expected_sha256 {
        if !calculated_hash.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(output_path);
            return Err(NetError::ChecksumMismatch {
                path: output_path.to_path_buf(),
                expected: expected.to_string(),
                calculated: calculated_hash,
            });
        }
        info!(hash = %calculated_hash, "Checksum verified successfully");
    } else {
        debug!(hash = %calculated_hash, "Downloaded file without checksum verification");
    }

    Ok(())
}

/// Downloads package assets from either BitTorrent or HTTP with automatic fallback.
pub async fn fetch_asset(assets: &Assets, destination_file: &Path) -> Result<(), NetError> {
    let span = info_span!("fetch_asset", destination = %destination_file.display());
    let _enter = span.enter();
    let mut downloaded = false;

    // Try BitTorrent first if magnet is defined
    if let Some(magnet) = &assets.magnet {
        info!(magnet = %magnet, "Attempting P2P download");
        match fetch_magnet(magnet, destination_file).await {
            Ok(()) => {
                downloaded = true;
            }
            Err(e) => {
                warn!(error = %e, "P2P download unsuccessful, falling back to HTTP");
            }
        }
    }

    // Fall back to direct HTTP
    if !downloaded {
        if let Some(url) = &assets.url {
            fetch_http(url, destination_file, assets.sha256.as_deref()).await?;
            downloaded = true;
        }
    }

    if !downloaded {
        return Err(NetError::NoSourceAvailable);
    }

    Ok(())
}

/// Fetches via BitTorrent magnet link.
#[allow(unused_variables)]
pub async fn fetch_magnet(magnet: &str, destination: &Path) -> Result<(), NetError> {
    let span = info_span!("fetch_magnet", magnet = %magnet, destination = %destination.display());
    let _enter = span.enter();
    #[cfg(feature = "bittorrent")]
    {
        let download_dir = destination.parent().unwrap_or_else(|| Path::new("."));
        let session = torrent::bittorrent::create_torrent_session(download_dir).await?;
        torrent::bittorrent::download_magnet(&session, magnet).await?;
        Ok(())
    }
    #[cfg(not(feature = "bittorrent"))]
    {
        Err(NetError::P2pError(
            "BitTorrent feature not enabled in build".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fetch_asset_no_source() {
        let assets = Assets {
            magnet: None,
            url: None,
            sha256: None,
        };
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("out.peca");

        let err = fetch_asset(&assets, &target).await.unwrap_err();
        match err {
            NetError::NoSourceAvailable => (),
            _ => panic!("Expected NoSourceAvailable error, got {err:?}"),
        }
    }
}
