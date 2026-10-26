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

use pubgrub::{
    resolve, DefaultStringReporter, DependencyConstraints, OfflineDependencyProvider, Ranges,
    Reporter, SemanticVersion,
};
use semver::{Comparator, Op, Version, VersionReq};

use crate::error::ResolveError;
use crate::manifest::Manifest;

/// Converts a standard `semver::Version` into a `pubgrub::SemanticVersion`.
pub fn to_pubgrub_version(v: &Version) -> SemanticVersion {
    SemanticVersion::new(v.major as u32, v.minor as u32, v.patch as u32)
}

/// Converts a `pubgrub::SemanticVersion` back into a `semver::Version`.
pub fn to_semver_version(sv: &SemanticVersion) -> Version {
    let s = sv.to_string();
    Version::parse(&s).unwrap_or_else(|_| Version::new(0, 0, 0))
}

/// Converts a single `semver::Comparator` into an exact `pubgrub::Ranges<SemanticVersion>`.
pub fn comparator_to_ranges(c: &Comparator) -> Ranges<SemanticVersion> {
    let major = c.major as u32;
    let minor = c.minor.unwrap_or(0) as u32;
    let patch = c.patch.unwrap_or(0) as u32;
    let target = SemanticVersion::new(major, minor, patch);

    match c.op {
        Op::Exact => Ranges::singleton(target),
        Op::Greater => Ranges::strictly_higher_than(target),
        Op::GreaterEq => Ranges::higher_than(target),
        Op::Less => Ranges::strictly_lower_than(target),
        Op::LessEq => Ranges::lower_than(target),
        Op::Tilde => {
            // ~1.2.3 := >=1.2.3, <1.3.0
            // ~1.2   := >=1.2.0, <1.3.0
            // ~1     := >=1.0.0, <2.0.0
            let upper = if c.minor.is_some() {
                SemanticVersion::new(major, minor + 1, 0)
            } else {
                SemanticVersion::new(major + 1, 0, 0)
            };
            Ranges::between(target, upper)
        }
        Op::Caret => {
            // ^1.2.3 := >=1.2.3, <2.0.0
            // ^0.2.3 := >=0.2.3, <0.3.0
            // ^0.0.3 := ==0.0.3
            if major > 0 {
                let upper = SemanticVersion::new(major + 1, 0, 0);
                Ranges::between(target, upper)
            } else if c.minor.unwrap_or(0) > 0 {
                let upper = SemanticVersion::new(0, minor + 1, 0);
                Ranges::between(target, upper)
            } else if c.patch.is_some() {
                Ranges::singleton(target)
            } else {
                let upper = SemanticVersion::new(0, 1, 0);
                Ranges::between(target, upper)
            }
        }
        Op::Wildcard => {
            if c.minor.is_some() {
                // 1.2.* := >=1.2.0, <1.3.0
                let upper = SemanticVersion::new(major, minor + 1, 0);
                Ranges::between(target, upper)
            } else if c.major > 0 {
                // 1.* := >=1.0.0, <2.0.0
                let upper = SemanticVersion::new(major + 1, 0, 0);
                Ranges::between(target, upper)
            } else {
                Ranges::full()
            }
        }
        _ => Ranges::full(),
    }
}

/// Maps a `semver::VersionReq` to a `pubgrub::Ranges<SemanticVersion>`.
pub fn version_req_to_ranges(req: &VersionReq) -> Ranges<SemanticVersion> {
    if req.comparators.is_empty() {
        return Ranges::full();
    }

    let mut result = Ranges::full();
    for comp in &req.comparators {
        result = result.intersection(&comparator_to_ranges(comp));
    }
    result
}

/// Pure in-memory catalog of available package releases and their manifests.
#[derive(Debug, Clone, Default)]
pub struct PackageCatalog {
    packages: BTreeMap<String, Vec<(Version, Manifest)>>,
}

impl PackageCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_package(&mut self, manifest: Manifest) {
        if let Ok(v) = Version::parse(&manifest.package.version) {
            let list = self
                .packages
                .entry(manifest.package.name.clone())
                .or_default();
            if !list.iter().any(|(existing_v, _)| existing_v == &v) {
                list.push((v, manifest));
            }
        }
    }

    pub fn add_packages(&mut self, manifests: impl IntoIterator<Item = Manifest>) {
        for m in manifests {
            self.add_package(m);
        }
    }

    pub fn get_manifest(&self, name: &str, version: &Version) -> Option<&Manifest> {
        self.packages
            .get(name)
            .and_then(|list| list.iter().find(|(v, _)| v == version).map(|(_, m)| m))
    }

    pub fn available_versions(&self, name: &str) -> Vec<Version> {
        self.packages
            .get(name)
            .map(|list| list.iter().map(|(v, _)| v.clone()).collect())
            .unwrap_or_default()
    }
}

/// Solves package versions for the given root dependencies against a `PackageCatalog` using PubGrub.
pub fn solve_dependencies(
    catalog: &PackageCatalog,
    root_dependencies: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Version>, ResolveError> {
    let mut provider = OfflineDependencyProvider::<String, Ranges<SemanticVersion>>::new();

    // 1. Register all catalog packages and their dependencies
    for (pkg_name, releases) in &catalog.packages {
        for (ver, manifest) in releases {
            let pubgrub_ver = to_pubgrub_version(ver);
            let mut deps = DependencyConstraints::<String, Ranges<SemanticVersion>>::default();

            for (dep_name, req_str) in &manifest.dependencies {
                let req = VersionReq::parse(req_str).unwrap_or(VersionReq::STAR);
                deps.insert(dep_name.clone(), version_req_to_ranges(&req));
            }

            provider.add_dependencies(pkg_name.clone(), pubgrub_ver, deps);
        }
    }

    // 2. Define virtual root package for the workspace
    let root_pkg = "__workspace_root__".to_string();
    let root_ver = SemanticVersion::new(1, 0, 0);
    let mut root_deps = DependencyConstraints::<String, Ranges<SemanticVersion>>::default();

    for (dep_name, req_str) in root_dependencies {
        let req = VersionReq::parse(req_str).unwrap_or(VersionReq::STAR);
        root_deps.insert(dep_name.clone(), version_req_to_ranges(&req));
    }
    provider.add_dependencies(root_pkg.clone(), root_ver, root_deps);

    // 3. Resolve using PubGrub CDCL algorithm
    match resolve(&provider, root_pkg, root_ver) {
        Ok(solution) => {
            let mut resolved = BTreeMap::new();
            for (pkg, ver) in solution {
                if pkg != "__workspace_root__" {
                    resolved.insert(pkg, to_semver_version(&ver));
                }
            }
            Ok(resolved)
        }
        Err(err) => match err {
            pubgrub::PubGrubError::NoSolution(derivation) => {
                let report = DefaultStringReporter::report(&derivation);
                Err(ResolveError::UnsolvableConstraints(report))
            }
            other => Err(ResolveError::UnsolvableConstraints(other.to_string())),
        },
    }
}

/// Solves package versions for a root `Manifest` against a `PackageCatalog`.
pub fn solve_workspace(
    catalog: &PackageCatalog,
    root_manifest: &Manifest,
) -> Result<BTreeMap<String, Version>, ResolveError> {
    solve_dependencies(catalog, &root_manifest.dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PackageMetadata, Priority, TargetGame};

    fn dummy_manifest(name: &str, version: &str, deps: &[(&str, &str)]) -> Manifest {
        let mut dep_map = BTreeMap::new();
        for (d_name, d_req) in deps {
            dep_map.insert(d_name.to_string(), d_req.to_string());
        }

        Manifest {
            package: PackageMetadata {
                name: name.to_string(),
                author: "author".to_string(),
                version: version.to_string(),
                priority: Priority::Default,
                description: None,
            },
            target: TargetGame {
                game_id: "test_game".to_string(),
                engine_version: None,
            },
            assets: None,
            dependencies: dep_map,
            overrides: BTreeMap::new(),
            load_order: None,
        }
    }

    #[test]
    fn test_semver_req_to_pubgrub_ranges() {
        let req = VersionReq::parse(">=1.2.0, <2.0.0").unwrap();
        let ranges = version_req_to_ranges(&req);

        assert!(ranges.contains(&SemanticVersion::new(1, 2, 0)));
        assert!(ranges.contains(&SemanticVersion::new(1, 5, 0)));
        assert!(!ranges.contains(&SemanticVersion::new(1, 1, 9)));
        assert!(!ranges.contains(&SemanticVersion::new(2, 0, 0)));
    }

    #[test]
    fn test_caret_and_tilde_ranges() {
        let req_caret = VersionReq::parse("^1.2.3").unwrap();
        let ranges_caret = version_req_to_ranges(&req_caret);
        assert!(ranges_caret.contains(&SemanticVersion::new(1, 2, 3)));
        assert!(ranges_caret.contains(&SemanticVersion::new(1, 9, 0)));
        assert!(!ranges_caret.contains(&SemanticVersion::new(2, 0, 0)));

        let req_tilde = VersionReq::parse("~1.2.3").unwrap();
        let ranges_tilde = version_req_to_ranges(&req_tilde);
        assert!(ranges_tilde.contains(&SemanticVersion::new(1, 2, 3)));
        assert!(ranges_tilde.contains(&SemanticVersion::new(1, 2, 9)));
        assert!(!ranges_tilde.contains(&SemanticVersion::new(1, 3, 0)));
    }

    #[test]
    fn test_pubgrub_solve_transitive_and_version_choice() {
        let mut catalog = PackageCatalog::new();

        // Framework versions: 1.0.0, 1.5.0, 2.0.0
        catalog.add_package(dummy_manifest("framework", "1.0.0", &[]));
        catalog.add_package(dummy_manifest("framework", "1.5.0", &[]));
        catalog.add_package(dummy_manifest("framework", "2.0.0", &[]));

        // Mod A requires Framework >= 1.0.0, < 2.0.0
        catalog.add_package(dummy_manifest(
            "mod_a",
            "1.0.0",
            &[("framework", ">=1.0.0, <2.0.0")],
        ));

        // Root depends on mod_a
        let mut root_deps = BTreeMap::new();
        root_deps.insert("mod_a".to_string(), ">=1.0.0".to_string());

        let solution =
            solve_dependencies(&catalog, &root_deps).expect("should find valid solution");

        assert_eq!(
            solution.get("mod_a"),
            Some(&Version::parse("1.0.0").unwrap())
        );
        // PubGrub chooses highest compatible matching version (1.5.0 over 1.0.0, but not incompatible 2.0.0)
        assert_eq!(
            solution.get("framework"),
            Some(&Version::parse("1.5.0").unwrap())
        );
    }

    #[test]
    fn test_pubgrub_conflict_derivation_reporting() {
        let mut catalog = PackageCatalog::new();

        catalog.add_package(dummy_manifest("skse", "2.1.0", &[]));
        catalog.add_package(dummy_manifest("skse", "2.3.0", &[]));

        // Mod A requires SKSE >= 2.2.0
        catalog.add_package(dummy_manifest("mod_a", "1.0.0", &[("skse", ">=2.2.0")]));
        // Mod B requires SKSE < 2.2.0
        catalog.add_package(dummy_manifest("mod_b", "1.0.0", &[("skse", "<2.2.0")]));

        // Root depends on both Mod A and Mod B
        let mut root_deps = BTreeMap::new();
        root_deps.insert("mod_a".to_string(), "*".to_string());
        root_deps.insert("mod_b".to_string(), "*".to_string());

        let err = solve_dependencies(&catalog, &root_deps).unwrap_err();
        match err {
            ResolveError::UnsolvableConstraints(explanation) => {
                assert!(
                    explanation.contains("mod_a")
                        || explanation.contains("mod_b")
                        || explanation.contains("skse"),
                    "Report must explain the conflict: {}",
                    explanation
                );
            }
            other => panic!("expected UnsolvableConstraints, got {:?}", other),
        }
    }
}
