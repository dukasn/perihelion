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

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CliError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Manifest(#[from] peri_core::ManifestError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Resolve(#[from] peri_core::ResolveError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Profile(#[from] peri_core::ProfileError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Config(#[from] peri_core::ConfigError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Security(#[from] peri_core::SecurityError),

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

impl From<peri_archive::ArchiveError> for CliError {
    fn from(err: peri_archive::ArchiveError) -> Self {
        match err {
            peri_archive::ArchiveError::Manifest(m) => CliError::Manifest(m),
            other => CliError::Archive(other.to_string()),
        }
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
