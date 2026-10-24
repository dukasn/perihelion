//! Perihelion Core Domain (`peri-core`)
//!
//! Pure functional logic. No I/O, no async runtimes, no side effects.
//! Defines package manifests, Directed Acyclic Graph (DAG) dependency & load order solving,
//! game layout profiles, and strict domain errors.

pub mod error;
pub mod graph;
pub mod manifest;
pub mod profile;
pub mod vfi;

pub use error::{ManifestError, ProfileError, ResolveError};
pub use graph::{DependencyGraph, EdgeType};
pub use manifest::{
    Assets, LoadOrderRules, Manifest, OverrideRule, PackageMetadata, Priority, TargetGame,
    MANIFEST_FILENAME,
};
pub use profile::{GameProfile, LayoutMapping};
pub use vfi::{
    ConflictResolutionReport, FileConflict, FileOwner, ResolutionSource, VirtualFileIndex,
    VFI_FILENAME,
};
