#[cfg(feature = "bittorrent")]
pub mod bittorrent {
    use crate::NetError;
    use librqbit::{AddTorrent, AddTorrentOptions, Session, SessionOptions};
    use std::path::Path;
    use std::sync::Arc;
    use tracing::info;

    pub async fn create_torrent_session(download_dir: &Path) -> Result<Arc<Session>, NetError> {
        let opts = SessionOptions::default();
        let session = Session::new_with_opts(download_dir.to_path_buf(), opts)
            .await
            .map_err(|e| NetError::P2pError(e.to_string()))?;
        Ok(session)
    }

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
