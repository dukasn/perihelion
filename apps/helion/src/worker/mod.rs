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

//! Asynchronous Worker Actor Pipelines for `helion`
//!
//! Enforces physical isolation between CPU-intensive/IO tasks (DAG graph resolution,
//! Virtual File Index collision scanning, process supervision) and the 60 FPS
//! presentation thread by offloading work to background Tokio tasks and communicating
//! via non-blocking channels.

pub mod session_recorder;
pub mod solver;
pub mod supervisor;
pub mod vfi;

pub use session_recorder::{
    check_recovery_draft, spawn_session_recorder, PersistenceCommand, RecoveryStatus,
    SessionRecorder,
};
pub use supervisor::{
    check_active_game_session, is_pid_alive, read_session_metadata, remove_session_metadata,
    save_session_metadata, spawn_game_supervisor, terminate_process, SESSION_FILENAME,
};

use crate::action::Action;
use tokio::sync::mpsc;

/// Sender type for background workers to deliver computed actions back to the presentation loop.
pub type WorkerSender = mpsc::UnboundedSender<Action>;

/// Receiver type for the presentation loop to receive worker results.
pub type WorkerReceiver = mpsc::UnboundedReceiver<Action>;

/// Channel pair for bi-directional or outbound worker communication.
pub fn create_worker_channel() -> (WorkerSender, WorkerReceiver) {
    mpsc::unbounded_channel()
}
