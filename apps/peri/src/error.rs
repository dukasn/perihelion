use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CliError {
    #[error("Manifest error: {0}")]
    #[diagnostic(
        code(peri::manifest_error),
        help("Check your peri.toml file syntax and required fields.")
    )]
    Manifest(String),

    #[error("Dependency resolution failed: {0}")]
    #[diagnostic(
        code(peri::resolve_error),
        help(
            "Ensure all required dependencies are installed and there are no cyclic relationships."
        )
    )]
    Resolve(String),

    #[error("Archive error: {0}")]
    #[diagnostic(
        code(peri::archive_error),
        help("Verify that the target .peca archive contains a valid root peri.toml and uncorrupted zstd data.")
    )]
    Archive(String),

    #[error("VFS & Sandboxing error: {0}")]
    #[diagnostic(
        code(peri::vfs_error),
        help("Ensure Linux user namespaces and OverlayFS support are available in your kernel environment.")
    )]
    Vfs(String),

    #[error("Network error: {0}")]
    #[diagnostic(
        code(peri::net_error),
        help("Check your internet connection and verify asset URLs / magnet links.")
    )]
    Net(String),

    #[error("Workspace error: {0}")]
    #[diagnostic(
        code(peri::workspace_error),
        help("Run 'peri init <game_id>' to initialize a Perihelion workspace.")
    )]
    Workspace(String),

    #[allow(dead_code)]
    #[error("Command failed: {0}")]
    #[diagnostic(code(peri::command_error))]
    Execution(String),
}

impl From<peri_core::ManifestError> for CliError {
    fn from(err: peri_core::ManifestError) -> Self {
        CliError::Manifest(err.to_string())
    }
}

impl From<peri_core::ResolveError> for CliError {
    fn from(err: peri_core::ResolveError) -> Self {
        CliError::Resolve(err.to_string())
    }
}

impl From<peri_archive::ArchiveError> for CliError {
    fn from(err: peri_archive::ArchiveError) -> Self {
        CliError::Archive(err.to_string())
    }
}

impl From<peri_vfs::VfsError> for CliError {
    fn from(err: peri_vfs::VfsError) -> Self {
        CliError::Vfs(err.to_string())
    }
}

impl From<peri_net::NetError> for CliError {
    fn from(err: peri_net::NetError) -> Self {
        CliError::Net(err.to_string())
    }
}
