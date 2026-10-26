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

//! Helion Architectural Invariants Integration Tests
//!
//! Validates:
//! 1. Non-blocking UI presentation clock maintenance under heavy background DAG load.
//! 2. Crash-resilient write-ahead journal durability and atomicity.
//! 3. Namespace execution snapshot isolation from mutable staging edits.
//! 4. Detached game process recovery and supervisor re-attachment.

use helion::action::Action;
use helion::app::App;
use helion::worker::create_worker_channel;
use helion::worker::session_recorder::SessionRecorder;
use helion::worker::solver::spawn_dag_solver;
use helion::worker::supervisor::{
    check_active_game_session, is_pid_alive, read_session_metadata, spawn_game_supervisor,
    terminate_process,
};
use peri_core::draft::{SessionDraft, DRAFT_FILENAME};
use peri_core::manifest::{Manifest, Priority};
use std::fs;
use std::time::{Duration, Instant};

fn create_mock_manifest(name: &str, deps: Vec<(&str, &str)>) -> Manifest {
    let mut toml = format!(
        r#"
[package]
name = "{name}"
version = "1.0.0"

[target]
game_id = "skyrimse"
"#
    );
    if !deps.is_empty() {
        toml.push_str("\n[dependencies]\n");
        for (dep, req) in deps {
            toml.push_str(&format!("{dep} = \"{req}\"\n"));
        }
    }
    Manifest::from_toml_str(&toml).expect("Valid test manifest")
}

#[tokio::test]
async fn test_invariant_non_blocking_ui_presentation_clock_under_dag_load() {
    let (worker_tx, mut worker_rx) = create_worker_channel();

    // 1. Generate a large graph of 150 interdependent mods to simulate heavy load
    let mut manifests = Vec::new();
    for i in 0..150 {
        let name = format!("mod_{:03}", i);
        let deps = if i > 0 {
            vec![(format!("mod_{:03}", i - 1), ">=1.0.0")]
        } else {
            vec![]
        };
        let dep_refs: Vec<(&str, &str)> = deps.iter().map(|(d, r)| (d.as_str(), *r)).collect();
        manifests.push(create_mock_manifest(&name, dep_refs));
    }

    // 2. Dispatch DAG solving onto background worker pool
    let _solver_handle = spawn_dag_solver(manifests, worker_tx);

    // 3. Measure UI presentation clock ticks (16ms interval, 60 FPS)
    let mut frame_timer = tokio::time::interval(Duration::from_millis(16));
    frame_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut tick_intervals = Vec::new();
    let mut last_tick = Instant::now();
    let mut app = App::new();

    for _ in 0..10 {
        tokio::select! {
            _ = frame_timer.tick() => {
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick);
                last_tick = now;
                tick_intervals.push(elapsed);
                app.handle_action(Action::Tick);
            }
            Some(worker_msg) = worker_rx.recv() => {
                app.handle_action(worker_msg);
            }
        }
    }

    // Assert that frames were pumped without being blocked by the background solver task
    assert!(tick_intervals.len() >= 8);
    for interval in tick_intervals.iter().skip(1) {
        // Ticks should remain roughly 16ms (allowing a generous margin under test virtual machines)
        assert!(
            interval.as_millis() < 100,
            "Frame tick stalled! Duration: {:?}",
            interval
        );
    }
}

#[tokio::test]
async fn test_invariant_atomic_session_journal_durability_and_atomicity() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let workspace = temp_dir.path().to_path_buf();
    let peri_dir = workspace.join(".perihelion");
    fs::create_dir_all(&peri_dir).expect("create dir");

    let tmp_path = peri_dir.join("draft.json.tmp");
    let target_path = peri_dir.join(DRAFT_FILENAME);

    let mut draft = SessionDraft::new("base_hash_abc");
    draft.staged_changes.stage_toggle("skyui", true);
    draft.staged_changes.stage_priority("water", Priority::High);

    // Test two-step atomic flush
    let flush_result = SessionRecorder::atomic_flush(&draft, &tmp_path, &target_path).await;
    assert!(flush_result.is_ok());

    // Target must exist, temporary file must have been cleanly renamed
    assert!(target_path.exists());
    assert!(!tmp_path.exists());

    // The written journal must be 100% valid JSON and match the staged draft exactly
    let raw = fs::read(&target_path).expect("read journal");
    let parsed: SessionDraft = serde_json::from_slice(&raw).expect("valid JSON deserialization");
    assert_eq!(parsed, draft);
}

#[tokio::test]
async fn test_invariant_namespace_isolation_during_active_gameplay() {
    let mut app = App::new();
    app.populate_test_fixture();

    // 1. Simulate game launched with frozen snapshot
    let snapshot_id = "snap_0x9af4".to_string();
    app.handle_action(Action::GameStarted {
        pid: 12345,
        snapshot_id: snapshot_id.clone(),
    });

    assert!(app.active_session.is_some());
    let active = app.active_session.as_ref().unwrap();
    assert_eq!(active.snapshot_id, snapshot_id);

    // 2. User stages mutations while game process is alive
    app.handle_action(Action::ToggleMod);
    app.handle_action(Action::CyclePriority);
    app.handle_action(Action::ResolveConflictChoice {
        index: 0,
        winner_id: "mod_x".to_string(),
    });

    // Staged buffer is dirty
    assert_eq!(app.staged.delta_count(), 3);

    // 3. Active execution session remains completely frozen and unchanged
    let active_after = app.active_session.as_ref().unwrap();
    assert_eq!(active_after.snapshot_id, snapshot_id);
    assert_eq!(active_after.pid, 12345);
}

#[tokio::test]
async fn test_invariant_detached_process_recovery_and_reattachment() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let workspace = temp_dir.path().to_path_buf();
    let (tx, mut rx) = create_worker_channel();

    // 1. Spawn a background child process that runs for a couple seconds
    let handle = spawn_game_supervisor(
        workspace.clone(),
        "skyrimse".to_string(),
        "default".to_string(),
        "sleep".to_string(),
        vec!["2".to_string()],
        None,
        tx,
    );

    // Receive GameStarted action
    let start_msg = rx.recv().await.expect("received start action");
    let child_pid = match start_msg {
        Action::GameStarted { pid, snapshot_id } => {
            assert!(pid > 0);
            assert!(snapshot_id.starts_with("snap_"));
            pid
        }
        other => panic!("Unexpected action: {:?}", other),
    };

    // 2. Verify metadata is safely recorded in session.json
    let meta = read_session_metadata(&workspace).expect("session.json exists");
    assert_eq!(meta.pid, child_pid);
    assert!(is_pid_alive(child_pid));

    // 3. Simulate terminal crash and relaunch: test check_active_game_session reattachment
    let reattached = check_active_game_session(&workspace);
    assert_eq!(reattached, Some(meta.clone()));

    // 4. Terminate process gracefully (SIGTERM)
    assert!(terminate_process(child_pid).is_ok());

    // Wait for supervisor to finish
    handle.await.expect("supervisor finished");

    let end_msg = rx.recv().await.expect("received end action");
    match end_msg {
        Action::GameTerminated { .. } => {}
        other => panic!("Unexpected action: {:?}", other),
    }

    // session.json must be safely unlinked
    assert!(read_session_metadata(&workspace).is_none());
}
