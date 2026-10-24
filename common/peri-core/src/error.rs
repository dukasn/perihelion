use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("Failed to parse manifest: {0}")]
    ParseError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid version '{version}': {reason}")]
    InvalidVersion { version: String, reason: String },

    #[error("Invalid checksum '{0}': expected 64-character hex SHA-256")]
    InvalidChecksum(String),

    #[error("Manifest validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResolveError {
    #[error("Cyclic dependency detected involving: {0}")]
    CyclicDependencyDetected(String),

    #[error("Unsatisfied dependency: package '{package}' requires '{dependency}' ({constraint})")]
    MissingDependency {
        package: String,
        dependency: String,
        constraint: String,
    },

    #[error("Incompatible packages: '{package_a}' is incompatible with '{package_b}'")]
    IncompatiblePackages {
        package_a: String,
        package_b: String,
    },

    #[error("Version mismatch: package '{package}' has version {found}, required constraint was {required}")]
    VersionMismatch {
        package: String,
        found: String,
        required: String,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("Unknown game profile ID: '{0}'")]
    UnknownGameProfile(String),

    #[error("Invalid profile configuration: {0}")]
    InvalidProfile(String),
}
