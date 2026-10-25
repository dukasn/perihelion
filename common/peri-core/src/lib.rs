//! Perihelion Core Domain (`peri-core`)
//!
//! Pure functional logic. No I/O, no async runtimes, no side effects.
//! Defines package manifests, Directed Acyclic Graph (DAG) dependency & load order solving,
//! game layout profiles, and strict domain errors.

pub mod config;
pub mod error;
pub mod graph;
pub mod manifest;
pub mod profile;
pub mod security;
pub mod vfi;

pub use config::{GlobalConfig, NetworkConfig, StorageConfig, CONFIG_FILENAME};
pub use error::{ConfigError, ManifestError, ProfileError, ResolveError, SecurityError};
pub use graph::{DependencyGraph, EdgeType};
pub use manifest::{
    Assets, LoadOrderRules, Manifest, OverrideRule, PackageMetadata, Priority, TargetGame,
    MANIFEST_FILENAME,
};
pub use profile::{GameProfile, LayoutMapping};
pub use security::validate_safe_path;
pub use vfi::{
    ConflictResolutionReport, FileConflict, FileOwner, PackageFileIndex, ResolutionSource,
    VirtualFileIndex, VFI_FILENAME,
};
