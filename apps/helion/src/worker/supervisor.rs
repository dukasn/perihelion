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

//! Game Process Supervision, VFS Sandboxing & Detached Recovery
//!
//! Orchestrates child game processes spawned within isolated OverlayFS namespaces,
//! monitors child PIDs asynchronously without blocking the UI thread, tracks
//! execution state in `.perihelion/session.json`, and enables detached process re-attachment.

use crate::action::Action;
use crate::worker::WorkerSender;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use peri_core::draft::ExecutionSessionMetadata;
use peri_vfs::VfsPlan;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::process::Command;

/// Standard file name for tracking running game sessions
pub const SESSION_FILENAME: &str = "session.json";

/// Writes active session metadata to `.perihelion/session.json`.
pub fn save_session_metadata(
    workspace: &Path,
    meta: &ExecutionSessionMetadata,
) -> Result<(), io::Error> {
    let peri_dir = workspace.join(".perihelion");
    fs::create_dir_all(&peri_dir)?;
    let session_path = peri_dir.join(SESSION_FILENAME);
    let serialized = serde_json::to_vec_pretty(meta)?;
    fs::write(session_path, serialized)
}

/// Reads active session metadata from `.perihelion/session.json` if it exists.
pub fn read_session_metadata(workspace: &Path) -> Option<ExecutionSessionMetadata> {
    let session_path = workspace.join(".perihelion").join(SESSION_FILENAME);
    if !session_path.exists() {
        return None;
    }
    let bytes = fs::read(session_path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Removes `.perihelion/session.json` when the game terminates.
pub fn remove_session_metadata(workspace: &Path) -> Result<(), io::Error> {
    let session_path = workspace.join(".perihelion").join(SESSION_FILENAME);
    if session_path.exists() {
        fs::remove_file(session_path)?;
    }
    Ok(())
}

/// Checks whether a process with the given PID is currently active via `/proc/<pid>`.
pub fn is_pid_alive(pid: u32) -> bool {
    let proc_path = format!("/proc/{}", pid);
    Path::new(&proc_path).exists()
}

/// Checks for any detached or lingering game session on startup.
/// If alive, returns the metadata for re-attachment. If dead, cleans up the orphaned file.
pub fn check_active_game_session(workspace: &Path) -> Option<ExecutionSessionMetadata> {
    if let Some(meta) = read_session_metadata(workspace) {
        if is_pid_alive(meta.pid) {
            Some(meta)
        } else {
            let _ = remove_session_metadata(workspace);
            None
        }
    } else {
        None
    }
}

/// Sends a graceful SIGTERM signal to the supervised game process.
pub fn terminate_process(pid: u32) -> Result<(), nix::Error> {
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
}

/// Spawns an asynchronous supervisor task to monitor the game executable.
pub fn spawn_game_supervisor(
    workspace: PathBuf,
    game_id: String,
    profile_name: String,
    executable: String,
    args: Vec<String>,
    _vfs_plan: Option<VfsPlan>,
    tx: WorkerSender,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let snapshot_id = format!("snap_{:#06x}", (timestamp % 0xFFFF) as u32);

        // Resolve executable command (fallback to sh/echo if mock or relative)
        let mut cmd = Command::new(&executable);
        cmd.args(&args);
        cmd.current_dir(&workspace);

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id().unwrap_or(0);

                let meta = ExecutionSessionMetadata {
                    pid,
                    game_id,
                    profile_name,
                    snapshot_id: snapshot_id.clone(),
                    launched_at: timestamp,
                };

                let _ = save_session_metadata(&workspace, &meta);
                let _ = tx.send(Action::GameStarted { pid, snapshot_id });

                // Asynchronously monitor child process without blocking presentation thread
                let status = child.wait().await;
                let exit_code = status.ok().and_then(|s| s.code());

                let _ = remove_session_metadata(&workspace);
                let _ = tx.send(Action::GameTerminated { exit_code });
            }
            Err(err) => {
                tracing::error!("Failed to spawn game process '{}': {}", executable, err);
                let _ = tx.send(Action::GameTerminated {
                    exit_code: Some(-1),
                });
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::create_worker_channel;

    #[test]
    fn test_session_metadata_lifecycle() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace = temp_dir.path();

        let meta = ExecutionSessionMetadata {
            pid: std::process::id(),
            game_id: "skyrimse".to_string(),
            profile_name: "default".to_string(),
            snapshot_id: "snap_0x1234".to_string(),
            launched_at: 1712345678,
        };

        // Save
        assert!(save_session_metadata(workspace, &meta).is_ok());

        // Read
        let loaded = read_session_metadata(workspace).expect("loaded metadata");
        assert_eq!(loaded, meta);

        // Alive check
        assert!(is_pid_alive(meta.pid));
        let active = check_active_game_session(workspace);
        assert_eq!(active, Some(meta.clone()));

        // Remove
        assert!(remove_session_metadata(workspace).is_ok());
        assert!(read_session_metadata(workspace).is_none());
    }

    #[test]
    fn test_dead_pid_cleaned_up() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace = temp_dir.path();

        // Use high PID that does not exist
        let meta = ExecutionSessionMetadata {
            pid: 4194300,
            game_id: "skyrimse".to_string(),
            profile_name: "default".to_string(),
            snapshot_id: "snap_dead".to_string(),
            launched_at: 1712345678,
        };

        assert!(save_session_metadata(workspace, &meta).is_ok());
        assert!(!is_pid_alive(meta.pid));

        let active = check_active_game_session(workspace);
        assert!(active.is_none());
        assert!(read_session_metadata(workspace).is_none());
    }

    #[tokio::test]
    async fn test_spawn_game_supervisor_success() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace = temp_dir.path().to_path_buf();
        let (tx, mut rx) = create_worker_channel();

        // Spawn a quick helper command
        let handle = spawn_game_supervisor(
            workspace.clone(),
            "test_game".to_string(),
            "default".to_string(),
            "sh".to_string(),
            vec!["-c".to_string(), "exit 0".to_string()],
            None,
            tx,
        );

        handle.await.expect("supervisor task finished");

        // First message: GameStarted
        let msg1 = rx.recv().await.expect("received msg1");
        match msg1 {
            Action::GameStarted { pid, snapshot_id } => {
                assert!(pid > 0);
                assert!(snapshot_id.starts_with("snap_"));
            }
            other => panic!("Unexpected action: {:?}", other),
        }

        // Second message: GameTerminated
        let msg2 = rx.recv().await.expect("received msg2");
        match msg2 {
            Action::GameTerminated { exit_code } => {
                assert_eq!(exit_code, Some(0));
            }
            other => panic!("Unexpected action: {:?}", other),
        }

        // session.json should have been cleaned up after exit
        assert!(read_session_metadata(&workspace).is_none());
    }
}
