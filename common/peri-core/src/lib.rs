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

//! Perihelion Core Domain (`peri-core`)
//!
//! Pure functional logic. No I/O, no async runtimes, no side effects.
//! Defines package manifests, Directed Acyclic Graph (DAG) dependency & load order solving,
//! game layout profiles, and strict domain errors.

pub mod config;
pub mod draft;
pub mod error;
pub mod graph;
pub mod lockfile;
pub mod manifest;
pub mod path;
pub mod profile;
pub mod report;
pub mod security;
pub mod signature;
pub mod solver;
pub mod vfi;

pub use config::{
    GlobalConfig, MappingRegistry, NetworkConfig, ProviderConfig, SteamConfig, StorageConfig,
    UiConfig, CONFIG_FILENAME,
};
pub use draft::{
    ExecutionSessionMetadata, SessionDraft, StagedChanges, UiContext, DRAFT_FILENAME,
    DRAFT_SCHEMA_VERSION, DRAFT_TMP_FILENAME,
};
pub use error::{
    ConfigError, LockfileError, ManifestError, ProfileError, ResolveError, SecurityError,
};
pub use graph::{DependencyGraph, EdgeType};
pub use lockfile::{verify_lockfile_sync, LockedPackage, Lockfile, LOCKFILE_FILENAME};
pub use manifest::{
    Assets, LoadOrderRules, Manifest, OverrideRule, PackageMetadata, Priority, TargetGame,
    MANIFEST_FILENAME,
};
pub use path::{hash_normalized_path, CompiledMappingRule, FastPathRouter, NormalizedPath};
pub use profile::{GameProfile, LayoutMapping};
pub use report::{
    CheckReport, DryRunPlanReport, FileConflictSummary, OutputFormat, ProfileSummaryReport,
    ResolvedPackageSummary,
};
pub use security::validate_safe_path;
pub use signature::{EngineSignature, SignatureRegistry};
pub use solver::{
    comparator_to_ranges, solve_dependencies, solve_workspace, to_pubgrub_version,
    to_semver_version, version_req_to_ranges, PackageCatalog,
};
pub use vfi::{
    ConflictResolutionReport, FileConflict, FileOwner, PackageFileIndex, ResolutionSource,
    VirtualFileIndex, VFI_FILENAME,
};
