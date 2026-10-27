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

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::error::ResolveError;
use crate::graph::DependencyGraph;
use crate::manifest::{OverrideRule, Priority};

pub const VFI_FILENAME: &str = "vfi.json";

/// A persistent file index for a single mod package, serializable to cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageFileIndex {
    pub package_name: String,
    pub version: String,
    pub priority: Priority,
    pub files: Vec<String>,
}

impl PackageFileIndex {
    pub fn new(
        package_name: impl Into<String>,
        version: impl Into<String>,
        priority: Priority,
        files: Vec<String>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            version: version.into(),
            priority,
            files,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOwner {
    pub mod_name: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileConflict {
    pub file_path: String,
    pub owners: Vec<FileOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionSource {
    ExplicitEdge { winner: String },
    PriorityWeight { winner: String, priority: Priority },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictResolutionReport {
    pub file_path: String,
    pub mod_a: String,
    pub mod_b: String,
    pub source: ResolutionSource,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualFileIndex {
    pub entries: BTreeMap<String, Vec<FileOwner>>,
}

impl VirtualFileIndex {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn add_entry(
        &mut self,
        file_path: impl Into<String>,
        mod_name: impl Into<String>,
        priority: Priority,
    ) {
        let path = file_path.into();
        let normalized = path.trim_start_matches('/').to_string();
        self.entries.entry(normalized).or_default().push(FileOwner {
            mod_name: mod_name.into(),
            priority,
        });
    }

    /// Adds all files from a persistent `PackageFileIndex`, applying an optional path mapping function.
    pub fn add_package_index<F>(&mut self, index: &PackageFileIndex, map_path: F)
    where
        F: Fn(&str) -> String,
    {
        for file in &index.files {
            let mapped = map_path(file);
            self.add_entry(mapped, &index.package_name, index.priority);
        }
    }

    pub fn detect_conflicts(&self) -> Vec<FileConflict> {
        let mut conflicts = Vec::new();
        for (file_path, owners) in &self.entries {
            if owners.len() > 1 {
                conflicts.push(FileConflict {
                    file_path: file_path.clone(),
                    owners: owners.clone(),
                });
            }
        }
        conflicts
    }

    /// Applies the 3-Tier Resolution Waterfall:
    /// Tier 1: Explicit Edge (loads_after or requires)
    /// Tier 2: Priority Weight (higher named priority wins)
    /// Tier 3: Hard Conflict (halt and throw diagnostic error)
    pub fn resolve_conflicts_waterfall(
        &self,
        graph: &mut DependencyGraph,
        workspace_overrides: &HashMap<String, OverrideRule>,
    ) -> Result<Vec<ConflictResolutionReport>, ResolveError> {
        let conflicts = self.detect_conflicts();
        let mut reports = Vec::new();

        for conflict in conflicts {
            for i in 0..conflict.owners.len() {
                for j in (i + 1)..conflict.owners.len() {
                    let owner_a = &conflict.owners[i];
                    let owner_b = &conflict.owners[j];

                    if owner_a.mod_name == owner_b.mod_name {
                        continue;
                    }

                    let mod_a = &owner_a.mod_name;
                    let mod_b = &owner_b.mod_name;

                    // Tier 1: Check if an explicit edge or path already exists in the DAG
                    if graph.has_path(mod_b, mod_a) {
                        // mod_a loads after mod_b -> mod_a wins
                        reports.push(ConflictResolutionReport {
                            file_path: conflict.file_path.clone(),
                            mod_a: mod_a.clone(),
                            mod_b: mod_b.clone(),
                            source: ResolutionSource::ExplicitEdge {
                                winner: mod_a.clone(),
                            },
                        });
                        continue;
                    } else if graph.has_path(mod_a, mod_b) {
                        // mod_b loads after mod_a -> mod_b wins
                        reports.push(ConflictResolutionReport {
                            file_path: conflict.file_path.clone(),
                            mod_a: mod_a.clone(),
                            mod_b: mod_b.clone(),
                            source: ResolutionSource::ExplicitEdge {
                                winner: mod_b.clone(),
                            },
                        });
                        continue;
                    }

                    // Tier 2: Priority Weight
                    let prio_a = match workspace_overrides.get(mod_a) {
                        Some(OverrideRule::Priority { priority }) => *priority,
                        _ => owner_a.priority,
                    };

                    let prio_b = match workspace_overrides.get(mod_b) {
                        Some(OverrideRule::Priority { priority }) => *priority,
                        _ => owner_b.priority,
                    };

                    if prio_a > prio_b {
                        // Mod A wins: Mod A must load AFTER Mod B (higher OverlayFS layer)
                        graph.add_edge_by_name(mod_b, mod_a);
                        reports.push(ConflictResolutionReport {
                            file_path: conflict.file_path.clone(),
                            mod_a: mod_a.clone(),
                            mod_b: mod_b.clone(),
                            source: ResolutionSource::PriorityWeight {
                                winner: mod_a.clone(),
                                priority: prio_a,
                            },
                        });
                    } else if prio_b > prio_a {
                        // Mod B wins: Mod B must load AFTER Mod A
                        graph.add_edge_by_name(mod_a, mod_b);
                        reports.push(ConflictResolutionReport {
                            file_path: conflict.file_path.clone(),
                            mod_a: mod_a.clone(),
                            mod_b: mod_b.clone(),
                            source: ResolutionSource::PriorityWeight {
                                winner: mod_b.clone(),
                                priority: prio_b,
                            },
                        });
                    } else {
                        // Tier 3: Hard Conflict
                        return Err(ResolveError::HardConflict {
                            file: conflict.file_path.clone(),
                            mod_a: mod_a.clone(),
                            mod_b: mod_b.clone(),
                            priority: format!("{prio_a:?}").to_lowercase(),
                        });
                    }
                }
            }
        }

        Ok(reports)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{LoadOrderRules, Manifest, PackageMetadata, TargetGame};
    use std::collections::BTreeMap;

    fn make_test_manifest(name: &str, priority: Priority) -> Manifest {
        Manifest {
            package: PackageMetadata {
                name: name.to_string(),
                author: "Tester".to_string(),
                version: "1.0.0".to_string(),
                priority,
                description: None,
            },
            target: TargetGame::new("test_game"),
            assets: None,
            dependencies: BTreeMap::new(),
            overrides: BTreeMap::new(),
            load_order: Some(LoadOrderRules::default()),
        }
    }

    #[test]
    fn test_waterfall_tier2_priority_weight() {
        let mut dags = DependencyGraph::new();
        dags.add_manifest(make_test_manifest("ModA", Priority::Low));
        dags.add_manifest(make_test_manifest("ModB", Priority::High));

        let mut vfi = VirtualFileIndex::new();
        vfi.add_entry("textures/sword.dds", "ModA", Priority::Low);
        vfi.add_entry("textures/sword.dds", "ModB", Priority::High);

        let overrides = HashMap::new();
        let reports = vfi
            .resolve_conflicts_waterfall(&mut dags, &overrides)
            .expect("Priority weight should resolve conflict");

        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].source,
            ResolutionSource::PriorityWeight {
                winner: "ModB".to_string(),
                priority: Priority::High
            }
        );

        let order = dags.resolve_load_order().expect("Should resolve DAG");
        // ModA must load before ModB (so ModB is higher in OverlayFS)
        assert_eq!(order, vec!["ModA", "ModB"]);
    }

    #[test]
    fn test_waterfall_tier3_hard_conflict() {
        let mut dags = DependencyGraph::new();
        dags.add_manifest(make_test_manifest("ModA", Priority::Default));
        dags.add_manifest(make_test_manifest("ModB", Priority::Default));

        let mut vfi = VirtualFileIndex::new();
        vfi.add_entry("textures/sword.dds", "ModA", Priority::Default);
        vfi.add_entry("textures/sword.dds", "ModB", Priority::Default);

        let overrides = HashMap::new();
        let err = vfi
            .resolve_conflicts_waterfall(&mut dags, &overrides)
            .unwrap_err();

        match err {
            ResolveError::HardConflict {
                file,
                mod_a,
                mod_b,
                priority,
            } => {
                assert_eq!(file, "textures/sword.dds");
                assert_eq!(priority, "default");
                assert!(mod_a == "ModA" || mod_a == "ModB");
                assert!(mod_b == "ModA" || mod_b == "ModB");
            }
            _ => panic!("Expected HardConflict error, got: {err:?}"),
        }
    }

    #[test]
    fn test_waterfall_tier3_resolved_by_override() {
        let mut dags = DependencyGraph::new();
        dags.add_manifest(make_test_manifest("ModA", Priority::Default));
        dags.add_manifest(make_test_manifest("ModB", Priority::Default));

        let mut vfi = VirtualFileIndex::new();
        vfi.add_entry("textures/sword.dds", "ModA", Priority::Default);
        vfi.add_entry("textures/sword.dds", "ModB", Priority::Default);

        let mut overrides = HashMap::new();
        overrides.insert(
            "ModB".to_string(),
            OverrideRule::Priority {
                priority: Priority::Highest,
            },
        );

        let reports = vfi
            .resolve_conflicts_waterfall(&mut dags, &overrides)
            .expect("Override should resolve conflict");

        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].source,
            ResolutionSource::PriorityWeight {
                winner: "ModB".to_string(),
                priority: Priority::Highest
            }
        );
    }

    #[test]
    fn test_package_file_index_serde_and_merge() {
        let pkg_index = PackageFileIndex::new(
            "TestMod",
            "1.2.0",
            Priority::High,
            vec![
                "textures/diffuse.png".to_string(),
                "meshes/model.nif".to_string(),
            ],
        );

        let json = pkg_index.to_json().expect("Serialization should succeed");
        let deserialized =
            PackageFileIndex::from_json(&json).expect("Deserialization should succeed");
        assert_eq!(pkg_index, deserialized);

        let mut vfi = VirtualFileIndex::new();
        vfi.add_package_index(&deserialized, |p| format!("data/{p}"));

        assert_eq!(vfi.entries.len(), 2);
        assert!(vfi.entries.contains_key("data/textures/diffuse.png"));
        assert!(vfi.entries.contains_key("data/meshes/model.nif"));
    }
}
