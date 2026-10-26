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

//! Asynchronous Virtual File Index (VFI) Worker
//!
//! Offloads file collision detection and waterfall conflict resolution
//! to a background blocking thread. Emits `Action::VfiScanCompleted`
//! back to the presentation loop with the total collision count.

use crate::action::Action;
use crate::worker::WorkerSender;
use peri_core::vfi::VirtualFileIndex;

/// Computes the total number of detected file collisions for a given VFI.
pub fn compute_vfi_collisions(vfi: &VirtualFileIndex) -> usize {
    vfi.detect_conflicts().len()
}

/// Dispatches VFI collision scanning on a background blocking thread.
/// Returns a `JoinHandle` to the async dispatch task.
pub fn spawn_vfi_scanner(vfi: VirtualFileIndex, tx: WorkerSender) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let collision_count = tokio::task::spawn_blocking(move || compute_vfi_collisions(&vfi))
            .await
            .unwrap_or(0);

        let _ = tx.send(Action::VfiScanCompleted { collision_count });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::create_worker_channel;
    use peri_core::manifest::Priority;

    #[tokio::test]
    async fn test_spawn_vfi_scanner_with_conflicts() {
        let (tx, mut rx) = create_worker_channel();

        let mut vfi = VirtualFileIndex::new();
        // Add two mods providing the same file
        vfi.add_entry("textures/water.dds", "mod_a", Priority::Default);
        vfi.add_entry("textures/water.dds", "mod_b", Priority::Default);

        let handle = spawn_vfi_scanner(vfi, tx);
        handle.await.expect("Worker finished");

        let action = rx.recv().await.expect("Received action");
        match action {
            Action::VfiScanCompleted { collision_count } => {
                assert_eq!(collision_count, 1);
            }
            other => panic!("Unexpected action: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_vfi_scanner_clean() {
        let (tx, mut rx) = create_worker_channel();

        let mut vfi = VirtualFileIndex::new();
        vfi.add_entry("textures/grass.dds", "mod_a", Priority::Default);
        vfi.add_entry("textures/stone.dds", "mod_b", Priority::Default);

        let handle = spawn_vfi_scanner(vfi, tx);
        handle.await.expect("Worker finished");

        let action = rx.recv().await.expect("Received action");
        match action {
            Action::VfiScanCompleted { collision_count } => {
                assert_eq!(collision_count, 0);
            }
            other => panic!("Unexpected action: {:?}", other),
        }
    }
}
