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

//! Session Draft, Transactional Staging & Recovery Models
//!
//! Provides pure in-memory data structures for transactionally staging workspace modifications
//! (mod active toggles, priority overrides, conflict resolutions) and serializing crash-resilient
//! write-ahead session journals.
//!
//! Conforms strictly to **Invariant A**: Pure functional models, zero I/O, zero async, zero side-effects.

use crate::manifest::Priority;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Current schema version for session drafts
pub const DRAFT_SCHEMA_VERSION: u32 = 1;

/// Standard file name used for session drafts on disk
pub const DRAFT_FILENAME: &str = "draft.json";

/// Standard temporary file name used during atomic session draft writing
pub const DRAFT_TMP_FILENAME: &str = "draft.json.tmp";

/// Represents the persisted recovery payload stored on disk in `.perihelion/draft.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDraft {
    /// Schema version for backward compatibility
    pub version: u32,
    /// Unix timestamp (seconds or milliseconds) when the journal was recorded
    pub timestamp: u64,
    /// SHA-256 hex digest of `peri.toml` at the time this draft was spawned
    pub base_manifest_hash: String,
    /// User modifications staged in memory
    pub staged_changes: StagedChanges,
    /// Terminal viewport context for seamless visual resumption
    pub ui_context: UiContext,
    /// Metadata regarding any detached running game session
    pub active_execution: Option<ExecutionSessionMetadata>,
}

impl SessionDraft {
    /// Constructs a new empty `SessionDraft` pegged to the given base manifest SHA-256 hash.
    pub fn new(base_manifest_hash: impl Into<String>) -> Self {
        Self {
            version: DRAFT_SCHEMA_VERSION,
            timestamp: 0,
            base_manifest_hash: base_manifest_hash.into(),
            staged_changes: StagedChanges::default(),
            ui_context: UiContext::default(),
            active_execution: None,
        }
    }

    /// Computes the deterministic SHA-256 hex digest of a manifest string.
    pub fn compute_manifest_hash(manifest_content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(manifest_content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Serializes this draft to a JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a draft from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Returns `true` if there are no staged modifications in this draft.
    pub fn is_empty(&self) -> bool {
        self.staged_changes.is_empty()
    }
}

/// Delta container holding staged, uncommitted workspace changes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StagedChanges {
    /// Overrides for mod active state: mod_id -> is_enabled
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub toggles: HashMap<String, bool>,
    /// Overrides for priority rules: mod_id -> Priority
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub priorities: HashMap<String, Priority>,
    /// Manually resolved file conflicts: relative_path -> winning_mod_id
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub manual_conflict_resolutions: HashMap<String, String>,
}

impl StagedChanges {
    /// Creates a new, empty set of staged changes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if no staged modifications are recorded.
    pub fn is_empty(&self) -> bool {
        self.toggles.is_empty()
            && self.priorities.is_empty()
            && self.manual_conflict_resolutions.is_empty()
    }

    /// Returns the total count of staged atomic operations.
    pub fn delta_count(&self) -> usize {
        self.toggles.len() + self.priorities.len() + self.manual_conflict_resolutions.len()
    }

    /// Clears all staged modifications.
    pub fn clear(&mut self) {
        self.toggles.clear();
        self.priorities.clear();
        self.manual_conflict_resolutions.clear();
    }

    /// Stages a toggle state for a specific mod.
    pub fn stage_toggle(&mut self, mod_id: impl Into<String>, active: bool) {
        self.toggles.insert(mod_id.into(), active);
    }

    /// Stages a priority override for a specific mod.
    pub fn stage_priority(&mut self, mod_id: impl Into<String>, priority: Priority) {
        self.priorities.insert(mod_id.into(), priority);
    }

    /// Stages a conflict winner for a file collision.
    pub fn stage_conflict_winner(
        &mut self,
        path: impl Into<String>,
        winning_mod: impl Into<String>,
    ) {
        self.manual_conflict_resolutions
            .insert(path.into(), winning_mod.into());
    }
}

/// Viewport and cursor context preserved across crashes or unexpected terminations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiContext {
    /// Currently active tab index (0 = Overview, 1 = Mods, etc.)
    pub active_tab_index: usize,
    /// Active cursor index in the focused list/table
    pub cursor_row: usize,
    /// Active search filter string
    pub active_search_query: String,
    /// Currently active category filter, if any
    pub selected_category_filter: Option<String>,
    /// Whether the inspector drawer or sidebar was expanded
    pub inspector_drawer_open: bool,
}

/// Metadata for orphaned process recovery across terminal restarts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionSessionMetadata {
    /// Process identifier of the launched game
    pub pid: u32,
    /// Target game identifier
    pub game_id: String,
    /// Active profile name
    pub profile_name: String,
    /// Unique snapshot identifier for the active OverlayFS stack
    pub snapshot_id: String,
    /// Timestamp when the process was spawned
    pub launched_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staged_changes_delta_count_and_clear() {
        let mut staged = StagedChanges::new();
        assert!(staged.is_empty());
        assert_eq!(staged.delta_count(), 0);

        staged.stage_toggle("skyui", true);
        assert_eq!(staged.delta_count(), 1);
        assert!(!staged.is_empty());

        staged.stage_priority("water_mod", Priority::High);
        assert_eq!(staged.delta_count(), 2);

        staged.stage_conflict_winner("textures/water.dds", "water_mod");
        assert_eq!(staged.delta_count(), 3);

        staged.clear();
        assert!(staged.is_empty());
        assert_eq!(staged.delta_count(), 0);
    }

    #[test]
    fn test_manifest_hash_deterministic() {
        let manifest = r#"
[package]
name = "test-pkg"
version = "1.0.0"

[target]
game_id = "skyrimse"
"#;
        let hash1 = SessionDraft::compute_manifest_hash(manifest);
        let hash2 = SessionDraft::compute_manifest_hash(manifest);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex string length
    }

    #[test]
    fn test_session_draft_json_roundtrip() {
        let mut draft = SessionDraft::new(
            "abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        );
        draft.timestamp = 1712345678;
        draft.staged_changes.stage_toggle("mod_a", false);
        draft
            .staged_changes
            .stage_priority("mod_b", Priority::Absolute);
        draft.ui_context = UiContext {
            active_tab_index: 1,
            cursor_row: 14,
            active_search_query: "water".to_string(),
            selected_category_filter: Some("Visuals".to_string()),
            inspector_drawer_open: true,
        };
        draft.active_execution = Some(ExecutionSessionMetadata {
            pid: 41209,
            game_id: "skyrimse".to_string(),
            profile_name: "default".to_string(),
            snapshot_id: "snap_0x9af4".to_string(),
            launched_at: 1712345600,
        });

        let json = draft.to_json_string().expect("Serialization failed");
        let deserialized = SessionDraft::from_json_str(&json).expect("Deserialization failed");

        assert_eq!(draft, deserialized);
        assert_eq!(deserialized.version, DRAFT_SCHEMA_VERSION);
        assert_eq!(deserialized.staged_changes.delta_count(), 2);
    }
}
