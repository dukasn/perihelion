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

//! Asynchronous DAG Solver Worker
//!
//! Offloads `peri-core` graph construction, cycle detection, and topological sorting
//! to a dedicated blocking worker thread using `tokio::task::spawn_blocking`.
//! Emits `Action::DagSolved` back to the UI presentation loop upon completion.

use crate::action::Action;
use crate::worker::WorkerSender;
use peri_core::error::ResolveError;
use peri_core::graph::DependencyGraph;
use peri_core::manifest::Manifest;

/// Pure computation function that runs on a blocking thread.
/// Constructs the `DependencyGraph` and calculates the topological load order.
pub fn compute_dag_order(manifests: Vec<Manifest>) -> (Vec<String>, bool) {
    let mut graph = DependencyGraph::new();
    for manifest in manifests {
        graph.add_manifest(manifest);
    }

    match graph.resolve_load_order() {
        Ok(ordered) => (ordered, false),
        Err(ResolveError::CyclicDependencyDetected(_)) => (Vec::new(), true),
        Err(_) => (Vec::new(), false),
    }
}

/// Dispatches DAG resolution on a background blocking thread.
/// Returns a `JoinHandle` to the async dispatch task.
pub fn spawn_dag_solver(manifests: Vec<Manifest>, tx: WorkerSender) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let (ordered_mod_ids, has_cycles) =
            tokio::task::spawn_blocking(move || compute_dag_order(manifests))
                .await
                .unwrap_or_else(|_| (Vec::new(), false));

        let _ = tx.send(Action::DagSolved {
            ordered_mod_ids,
            has_cycles,
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::create_worker_channel;

    fn make_test_manifest(name: &str, dependencies: Vec<(&str, &str)>) -> Manifest {
        let mut toml = format!(
            r#"
[package]
name = "{name}"
version = "1.0.0"

[target]
game_id = "skyrimse"
"#
        );

        if !dependencies.is_empty() {
            toml.push_str("\n[dependencies]\n");
            for (dep, req) in dependencies {
                toml.push_str(&format!("{dep} = \"{req}\"\n"));
            }
        }

        Manifest::from_toml_str(&toml).expect("Valid test manifest")
    }

    #[tokio::test]
    async fn test_spawn_dag_solver_success() {
        let (tx, mut rx) = create_worker_channel();

        let mod_a = make_test_manifest("mod-a", vec![]);
        let mod_b = make_test_manifest("mod-b", vec![("mod-a", ">=1.0.0")]);

        let handle = spawn_dag_solver(vec![mod_b, mod_a], tx);
        handle.await.expect("Worker finished");

        let action = rx.recv().await.expect("Received action");
        match action {
            Action::DagSolved {
                ordered_mod_ids,
                has_cycles,
            } => {
                assert!(!has_cycles);
                assert_eq!(ordered_mod_ids, vec!["mod-a", "mod-b"]);
            }
            other => panic!("Unexpected action: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_dag_solver_cycle_detection() {
        let (tx, mut rx) = create_worker_channel();

        let mod_a = make_test_manifest("mod-a", vec![("mod-b", ">=1.0.0")]);
        let mod_b = make_test_manifest("mod-b", vec![("mod-a", ">=1.0.0")]);

        let handle = spawn_dag_solver(vec![mod_a, mod_b], tx);
        handle.await.expect("Worker finished");

        let action = rx.recv().await.expect("Received action");
        match action {
            Action::DagSolved {
                ordered_mod_ids,
                has_cycles,
            } => {
                assert!(has_cycles);
                assert!(ordered_mod_ids.is_empty());
            }
            other => panic!("Unexpected action: {:?}", other),
        }
    }
}
