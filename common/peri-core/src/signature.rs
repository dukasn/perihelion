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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::profile::LayoutMapping;

/// Declarative layout signature defining game engine directory structures,
/// executable candidate heuristics, and VFS path remapping rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSignature {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub anchors: Vec<String>,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub binary_subdirs: Vec<String>,
    #[serde(default)]
    pub binary_extensions: Vec<String>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(default)]
    pub mappings: Vec<LayoutMapping>,
    #[serde(default)]
    pub content_extensions: Vec<String>,
}

impl EngineSignature {
    /// Evaluates whether this engine signature matches an abstract path probe.
    ///
    /// An engine matches if:
    /// 1. All structural `anchors` are confirmed by the probe.
    /// 2. If `markers` are specified, at least one marker is confirmed by the probe.
    pub fn matches<F>(&self, mut probe: F) -> bool
    where
        F: FnMut(&str) -> bool,
    {
        if self.anchors.is_empty() && self.markers.is_empty() {
            return false;
        }

        // All anchors must be present
        for anchor in &self.anchors {
            if !probe(anchor) {
                return false;
            }
        }

        // If markers are defined, at least one must match
        if !self.markers.is_empty() {
            let mut marker_found = false;
            for marker in &self.markers {
                if probe(marker) {
                    marker_found = true;
                    break;
                }
            }
            if !marker_found {
                return false;
            }
        }

        true
    }
}

/// Registry of known game engine signatures.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureRegistry {
    #[serde(default)]
    signatures: BTreeMap<String, EngineSignature>,
}

impl SignatureRegistry {
    pub fn new() -> Self {
        Self {
            signatures: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, sig: EngineSignature) {
        self.signatures.insert(sig.id.clone(), sig);
    }

    pub fn get(&self, id: &str) -> Option<&EngineSignature> {
        self.signatures.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<EngineSignature> {
        self.signatures.remove(id)
    }

    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EngineSignature> {
        self.signatures.values()
    }

    /// Returns registered engine signatures sorted by priority descending.
    pub fn sorted_by_priority(&self) -> Vec<&EngineSignature> {
        let mut list: Vec<&EngineSignature> = self.signatures.values().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.priority));
        list
    }

    /// Evaluates an abstract probe against registered signatures in priority order,
    /// returning the highest-priority matching engine signature.
    pub fn detect<F>(&self, mut probe: F) -> Option<&EngineSignature>
    where
        F: FnMut(&str) -> bool,
    {
        self.sorted_by_priority()
            .into_iter()
            .find(|sig| sig.matches(&mut probe))
    }

    /// Merges signatures from another registry into this one, overwriting duplicates.
    pub fn merge(&mut self, other: SignatureRegistry) {
        for (id, sig) in other.signatures {
            self.signatures.insert(id, sig);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_matching_logic() {
        let sig = EngineSignature {
            id: "bethesda_creation".to_string(),
            name: "Bethesda Creation Engine".to_string(),
            priority: 100,
            anchors: vec!["Data".to_string()],
            markers: vec!["Data/*.esm".to_string(), "Data/*.esp".to_string()],
            binary_subdirs: vec![String::new(), "bin".to_string()],
            binary_extensions: vec!["exe".to_string()],
            environment: vec![],
            mappings: vec![LayoutMapping::new("Data", "Data")],
            content_extensions: vec!["esp".to_string(), "esm".to_string()],
        };

        // Matches when anchor and at least one marker match
        let matches = sig.matches(|path| path == "Data" || path == "Data/*.esm");
        assert!(matches);

        // Fails when anchor is missing
        let no_anchor = sig.matches(|path| path == "Data/*.esm");
        assert!(!no_anchor);

        // Fails when marker is missing
        let no_marker = sig.matches(|path| path == "Data");
        assert!(!no_marker);
    }

    #[test]
    fn test_registry_priority_order_detection() {
        let mut registry = SignatureRegistry::new();

        let sig_generic = EngineSignature {
            id: "generic".to_string(),
            name: "Generic Engine".to_string(),
            priority: 10,
            anchors: vec!["bin".to_string()],
            markers: vec![],
            binary_subdirs: vec!["bin".to_string()],
            binary_extensions: vec!["exe".to_string()],
            environment: vec![],
            mappings: vec![],
            content_extensions: vec![],
        };

        let sig_specialized = EngineSignature {
            id: "redengine_4".to_string(),
            name: "REDengine 4".to_string(),
            priority: 150,
            anchors: vec!["bin".to_string(), "bin/x64/Cyberpunk2077.exe".to_string()],
            markers: vec!["r6/config".to_string()],
            binary_subdirs: vec!["bin/x64".to_string()],
            binary_extensions: vec!["exe".to_string()],
            environment: vec![],
            mappings: vec![LayoutMapping::new("mods", "archive/pc/mod")],
            content_extensions: vec!["archive".to_string()],
        };

        registry.register(sig_generic);
        registry.register(sig_specialized);

        // When probing a directory that matches both, the higher priority one wins
        let detected = registry
            .detect(|path| matches!(path, "bin" | "bin/x64/Cyberpunk2077.exe" | "r6/config"));

        assert!(detected.is_some());
        assert_eq!(detected.unwrap().id, "redengine_4");
    }
}
