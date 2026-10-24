use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic, Clone, PartialEq, Eq)]
pub enum ManifestError {
    #[error("Failed to parse peri.toml: {message}")]
    #[diagnostic(
        code(peri::manifest::syntax_error),
        help("Ensure your TOML syntax is valid.")
    )]
    ParseError {
        message: String,
        #[source_code]
        src: String,
        #[label("Syntax error found here")]
        span: SourceSpan,
    },

    #[error("Missing required field: {field}")]
    #[diagnostic(
        code(peri::manifest::missing_field),
        help("peri.toml manifests must specify [package] name, version, and [target] game_id.")
    )]
    MissingField {
        field: String,
        #[source_code]
        src: String,
        #[label("Missing required field '{field}'")]
        span: SourceSpan,
    },

    #[error("Invalid version '{version}': {reason}")]
    #[diagnostic(
        code(peri::manifest::invalid_version),
        help("Package version must follow semantic versioning (e.g. '1.2.0')")
    )]
    InvalidVersion {
        version: String,
        reason: String,
        #[source_code]
        src: String,
        #[label("Cannot parse version: {reason}")]
        span: SourceSpan,
    },

    #[error("Invalid priority level '{bad_value}'")]
    #[diagnostic(
        code(peri::manifest::invalid_priority),
        help("Valid priorities are: lowest, low, default, high, highest, absolute")
    )]
    InvalidPriority {
        bad_value: String,
        #[source_code]
        src: String,
        #[label("Unknown priority level '{bad_value}'")]
        span: SourceSpan,
    },

    #[error("Invalid SemVer constraint for dependency '{dependency}'")]
    #[diagnostic(
        code(peri::manifest::invalid_dependency_constraint),
        help("Dependencies must use valid SemVer constraints (e.g. '>=1.2.0', '~2.0')")
    )]
    InvalidDependencyConstraint {
        dependency: String,
        constraint: String,
        reason: String,
        #[source_code]
        src: String,
        #[label("Invalid constraint: {reason}")]
        span: SourceSpan,
    },

    #[error("Invalid checksum '{checksum}': expected 64-character hex SHA-256")]
    #[diagnostic(
        code(peri::manifest::invalid_checksum),
        help("Expected a 64-character lowercase hexadecimal SHA-256 hash")
    )]
    InvalidChecksum {
        checksum: String,
        #[source_code]
        src: String,
        #[label("Invalid SHA-256 hash")]
        span: SourceSpan,
    },

    #[error("Manifest validation error: {0}")]
    #[diagnostic(code(peri::manifest::validation))]
    ValidationError(String),
}

#[derive(Debug, Error, Diagnostic, Clone, PartialEq, Eq)]
pub enum ResolveError {
    #[error("Cyclic dependency detected involving: {0}")]
    #[diagnostic(
        code(peri::resolve::cyclic_dependency),
        help(
            "Break the circular dependency by removing an explicit requires or loads_after rule."
        )
    )]
    CyclicDependencyDetected(String),

    #[error("Unsatisfied dependency: package '{package}' requires '{dependency}' ({constraint})")]
    #[diagnostic(
        code(peri::resolve::missing_dependency),
        help("Install the missing package or update the dependency version requirement.")
    )]
    MissingDependency {
        package: String,
        dependency: String,
        constraint: String,
    },

    #[error("Incompatible packages: '{package_a}' is incompatible with '{package_b}'")]
    #[diagnostic(
        code(peri::resolve::incompatible_packages),
        help("One of these packages declared an incompatible_with rule against the other.")
    )]
    IncompatiblePackages {
        package_a: String,
        package_b: String,
    },

    #[error("Version mismatch: package '{package}' has version {found}, required constraint was {required}")]
    #[diagnostic(
        code(peri::resolve::version_mismatch),
        help("Update the package or adjust the version requirement constraint.")
    )]
    VersionMismatch {
        package: String,
        found: String,
        required: String,
    },

    #[error("Hard conflict detected on file '{file}': mods '{mod_a}' and '{mod_b}' both have priority '{priority}'. Add an explicit rule to your [overrides] block.")]
    #[diagnostic(
        code(peri::resolve::hard_conflict),
        help("Add an explicit override rule to your peri.toml [overrides] table to decide which mod wins.")
    )]
    HardConflict {
        file: String,
        mod_a: String,
        mod_b: String,
        priority: String,
    },
}

#[derive(Debug, Error, Diagnostic, Clone, PartialEq, Eq)]
pub enum ProfileError {
    #[error("Unknown game profile ID: '{0}'")]
    #[diagnostic(code(peri::profile::unknown_game))]
    UnknownGameProfile(String),

    #[error("Failed to parse game profile TOML: {0}")]
    #[diagnostic(code(peri::profile::parse_error))]
    ParseError(String),

    #[error("Invalid profile configuration: {0}")]
    #[diagnostic(code(peri::profile::invalid))]
    InvalidProfile(String),
}
