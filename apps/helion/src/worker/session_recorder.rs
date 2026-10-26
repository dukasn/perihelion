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

//! Crash-Resilient Session Persistence Worker (Debounced WAL)
//!
//! Provides an asynchronous actor that buffers workspace and UI modifications in memory
//! and writes them to a Write-Ahead Journal (`.perihelion/draft.json`) using a 300ms
//! sliding debounce window and two-step atomic filesystem flush (`.tmp` -> `sync_all` -> `rename`).

use peri_core::draft::{SessionDraft, DRAFT_FILENAME, DRAFT_TMP_FILENAME};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::{sleep, Instant};

/// Commands sent to the background session recorder actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceCommand {
    /// Update the pending draft to be persisted to disk.
    UpdateDraft(Box<SessionDraft>),
    /// Invalidate and remove the session draft on disk (e.g. after commit or discard).
    PurgeDraft,
}

/// Recovery reconciliation status discovered on application startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// Session draft matches current `peri.toml` SHA-256 hash cleanly.
    CleanResume(Box<SessionDraft>),
    /// Session draft hash differs from on-disk `peri.toml`, indicating an external modification.
    Conflict {
        on_disk_hash: String,
        draft_hash: String,
        draft: Box<SessionDraft>,
    },
}

/// Inspects the workspace root for existing recovery drafts and reconciles against `peri.toml`.
pub async fn check_recovery_draft(workspace_root: &Path) -> Option<RecoveryStatus> {
    let peri_dir = workspace_root.join(".perihelion");
    let draft_path = peri_dir.join(DRAFT_FILENAME);

    if !draft_path.exists() {
        return None;
    }

    let Ok(draft_bytes) = fs::read(&draft_path).await else {
        return None;
    };

    let Ok(draft) = serde_json::from_slice::<SessionDraft>(&draft_bytes) else {
        tracing::warn!("Found malformed .perihelion/draft.json, ignoring.");
        return None;
    };

    let manifest_path = workspace_root.join("peri.toml");
    if manifest_path.exists() {
        if let Ok(manifest_content) = fs::read_to_string(&manifest_path).await {
            let on_disk_hash = SessionDraft::compute_manifest_hash(&manifest_content);
            if on_disk_hash == draft.base_manifest_hash {
                return Some(RecoveryStatus::CleanResume(Box::new(draft)));
            } else {
                return Some(RecoveryStatus::Conflict {
                    on_disk_hash,
                    draft_hash: draft.base_manifest_hash.clone(),
                    draft: Box::new(draft),
                });
            }
        }
    }

    Some(RecoveryStatus::CleanResume(Box::new(draft)))
}

/// The session recorder worker instance.
pub struct SessionRecorder {
    workspace_root: PathBuf,
    rx: mpsc::Receiver<PersistenceCommand>,
    debounce_duration: Duration,
}

impl SessionRecorder {
    pub fn new(workspace_root: PathBuf, rx: mpsc::Receiver<PersistenceCommand>) -> Self {
        Self {
            workspace_root,
            rx,
            debounce_duration: Duration::from_millis(300),
        }
    }

    /// Sets a custom debounce duration (useful for faster unit tests).
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce_duration = debounce;
        self
    }

    /// Runs the asynchronous persistence actor loop.
    pub async fn run(mut self) {
        let peri_dir = self.workspace_root.join(".perihelion");
        let draft_path = peri_dir.join(DRAFT_FILENAME);
        let tmp_path = peri_dir.join(DRAFT_TMP_FILENAME);

        let mut pending_draft: Option<Box<SessionDraft>> = None;
        let mut last_activity = Instant::now();

        loop {
            tokio::select! {
                command = self.rx.recv() => {
                    match command {
                        Some(PersistenceCommand::UpdateDraft(draft)) => {
                            pending_draft = Some(draft);
                            last_activity = Instant::now();
                        }
                        Some(PersistenceCommand::PurgeDraft) => {
                            pending_draft = None;
                            let _ = fs::remove_file(&draft_path).await;
                            let _ = fs::remove_file(&tmp_path).await;
                        }
                        None => {
                            // Channel closed by sender: flush pending changes before exit
                            if let Some(draft) = pending_draft.take() {
                                let _ = Self::atomic_flush(&draft, &tmp_path, &draft_path).await;
                            }
                            break;
                        }
                    }
                }

                _ = sleep(self.debounce_duration), if pending_draft.is_some() => {
                    if last_activity.elapsed() >= self.debounce_duration {
                        if let Some(draft) = pending_draft.take() {
                            if let Err(err) = Self::atomic_flush(&draft, &tmp_path, &draft_path).await {
                                tracing::error!("Failed to persist session draft: {}", err);
                            } else {
                                tracing::trace!("Session draft journaled successfully.");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Executes the two-step atomic filesystem flush:
    /// 1. Write payload to temporary sibling file (`draft.json.tmp`) and sync pages to disk.
    /// 2. Atomically rename temporary file to target path (`draft.json`).
    pub async fn atomic_flush(
        draft: &SessionDraft,
        tmp_path: &Path,
        target_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let serialized = serde_json::to_vec_pretty(draft)?;

        if let Some(parent) = tmp_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(tmp_path)
            .await?;

        file.write_all(&serialized).await?;
        file.flush().await?;
        file.sync_all().await?;
        drop(file);

        fs::rename(tmp_path, target_path).await?;
        Ok(())
    }
}

/// Spawns the session recorder worker as a background Tokio task.
pub fn spawn_session_recorder(
    workspace_root: PathBuf,
) -> (
    mpsc::Sender<PersistenceCommand>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel(64);
    let recorder = SessionRecorder::new(workspace_root, rx);
    let handle = tokio::spawn(recorder.run());
    (tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use peri_core::manifest::Priority;

    #[tokio::test]
    async fn test_session_recorder_atomic_flush_and_purge() {
        let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let workspace = temp_dir.path().to_path_buf();

        let (tx, rx) = mpsc::channel(64);
        let recorder =
            SessionRecorder::new(workspace.clone(), rx).with_debounce(Duration::from_millis(50));
        let handle = tokio::spawn(recorder.run());

        let mut draft = SessionDraft::new("test_hash");
        draft.staged_changes.stage_toggle("skyui", true);
        draft
            .staged_changes
            .stage_priority("water_mod", Priority::High);

        tx.send(PersistenceCommand::UpdateDraft(Box::new(draft.clone())))
            .await
            .expect("Send succeeded");

        // Poll rather than fixed sleep: slow CI runners can exceed the debounce margin.
        let draft_path = workspace.join(".perihelion").join(DRAFT_FILENAME);
        let tmp_path = workspace.join(".perihelion").join(DRAFT_TMP_FILENAME);

        for _ in 0..100 {
            if draft_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert!(draft_path.exists(), "draft.json must exist after flush");
        assert!(!tmp_path.exists(), "draft.json.tmp must have been renamed");

        let read_bytes = fs::read(&draft_path).await.expect("Read draft");
        let loaded_draft: SessionDraft = serde_json::from_slice(&read_bytes).expect("Parsed draft");
        assert_eq!(loaded_draft, draft);

        // Test Purge
        tx.send(PersistenceCommand::PurgeDraft)
            .await
            .expect("Purge command succeeded");

        for _ in 0..100 {
            if !draft_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(!draft_path.exists(), "draft.json must be removed on purge");

        drop(tx);
        handle.await.expect("Worker terminated cleanly");
    }

    #[tokio::test]
    async fn test_check_recovery_draft_reconciliation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let workspace = temp_dir.path().to_path_buf();

        let peri_dir = workspace.join(".perihelion");
        fs::create_dir_all(&peri_dir).await.expect("Created dir");

        let manifest_content = "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n";
        let manifest_path = workspace.join("peri.toml");
        fs::write(&manifest_path, manifest_content)
            .await
            .expect("Wrote manifest");

        let matching_hash = SessionDraft::compute_manifest_hash(manifest_content);
        let mut draft = SessionDraft::new(matching_hash);
        draft.staged_changes.stage_toggle("mod1", true);

        let draft_path = peri_dir.join(DRAFT_FILENAME);
        let serialized = serde_json::to_vec_pretty(&draft).expect("Serialized");
        fs::write(&draft_path, serialized)
            .await
            .expect("Wrote draft");

        // 1. Clean resume test
        let status = check_recovery_draft(&workspace).await;
        assert_eq!(
            status,
            Some(RecoveryStatus::CleanResume(Box::new(draft.clone())))
        );

        // 2. Conflict test: modify peri.toml externally
        let modified_manifest = "[package]\nname = \"demo\"\nversion = \"2.0.0\"\n";
        fs::write(&manifest_path, modified_manifest)
            .await
            .expect("Wrote manifest");
        let new_hash = SessionDraft::compute_manifest_hash(modified_manifest);

        let status = check_recovery_draft(&workspace).await;
        match status {
            Some(RecoveryStatus::Conflict {
                on_disk_hash,
                draft_hash,
                draft: loaded_draft,
            }) => {
                assert_eq!(on_disk_hash, new_hash);
                assert_eq!(draft_hash, draft.base_manifest_hash);
                assert_eq!(*loaded_draft, draft);
            }
            other => panic!("Expected Conflict status, got: {:?}", other),
        }
    }
}
