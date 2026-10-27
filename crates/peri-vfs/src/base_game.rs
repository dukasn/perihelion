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

//! Dynamic Base Game Resolution Ladder
//!
//! Resolves the vanilla base game directory dynamically without requiring
//! a `./base_game` symlink in the workspace. Evaluates sources in strict
//! precedence order:
//! 1. CLI flag `--base-path <PATH>`
//! 2. Environment variable `PERI_BASE_GAME`
//! 3. Manifest target `game_path` (`peri.toml`)
//! 4. Game profile configured `game_path`
//! 5. Storefront provider auto-discovery (Steam)
//! 6. Workspace `./base_game` fallback directory

use std::fmt;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use peri_core::config::ProviderConfig;

use crate::proton::SteamDiscovery;
use crate::VfsError;

/// Provenance tier that successfully resolved the base game directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseGameSource {
    /// Explicit CLI flag (`--base-path`)
    Cli,
    /// Environment variable (`PERI_BASE_GAME`)
    Environment,
    /// Workspace manifest (`[target] game_path` in `peri.toml`)
    Manifest,
    /// Game profile definition (`game_path`)
    Profile,
    /// Storefront provider auto-discovery (e.g. Steam library scan)
    SteamDiscovery,
    /// Legacy fallback (`base_game` directory inside workspace)
    WorkspaceFallback,
}

impl fmt::Display for BaseGameSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli => write!(f, "CLI option --base-path"),
            Self::Environment => write!(f, "environment variable PERI_BASE_GAME"),
            Self::Manifest => write!(f, "manifest [target] game_path"),
            Self::Profile => write!(f, "game profile configuration"),
            Self::SteamDiscovery => write!(f, "Steam auto-discovery"),
            Self::WorkspaceFallback => write!(f, "workspace base_game directory"),
        }
    }
}

/// Resolved base game directory path and its resolution provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBaseGame {
    /// Absolute or canonical path to the base game folder
    pub path: PathBuf,
    /// Resolution tier that produced this path
    pub source: BaseGameSource,
}

/// Query parameters supplied to the dynamic base game resolution ladder.
#[derive(Debug, Clone, Default)]
pub struct BaseGameQuery<'a> {
    /// Explicit CLI override (`--base-path <PATH>`)
    pub cli_path: Option<&'a Path>,
    /// Environment variable value (e.g. read from `PERI_BASE_GAME`)
    pub env_var: Option<&'a str>,
    /// Target manifest specified path (`manifest.target.game_path`)
    pub manifest_path: Option<&'a Path>,
    /// Profile declared game path (`profile.game_path`)
    pub profile_path: Option<&'a Path>,
    /// Steam App ID for provider auto-discovery
    pub steam_app_id: Option<u64>,
    /// Platform provider config (for scanning Steam library folders)
    pub provider: Option<&'a ProviderConfig>,
    /// Workspace root directory (for resolving relative paths and `./base_game`)
    pub workspace: Option<&'a Path>,
}

/// Resolves the base game directory using the 6-tier precedence ladder.
pub fn resolve_base_game_path(query: &BaseGameQuery<'_>) -> Result<ResolvedBaseGame, VfsError> {
    // Tier 1: CLI override
    if let Some(path) = query.cli_path {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !canon.exists() {
            return Err(VfsError::BaseGameNotFound(format!(
                "Base game path specified via CLI '--base-path' does not exist: {}",
                path.display()
            )));
        }
        return Ok(ResolvedBaseGame {
            path: canon,
            source: BaseGameSource::Cli,
        });
    }

    // Tier 2: Environment variable PERI_BASE_GAME
    let env_val = query
        .env_var
        .map(String::from)
        .or_else(|| std::env::var("PERI_BASE_GAME").ok());

    if let Some(val) = env_val {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !canon.exists() {
                return Err(VfsError::BaseGameNotFound(format!(
                    "Base game path specified in environment variable PERI_BASE_GAME does not exist: {}",
                    path.display()
                )));
            }
            return Ok(ResolvedBaseGame {
                path: canon,
                source: BaseGameSource::Environment,
            });
        }
    }

    // Tier 3: Manifest target game_path
    if let Some(path) = query.manifest_path {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ws) = query.workspace {
            ws.join(path)
        } else {
            path.to_path_buf()
        };
        let canon = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if canon.exists() {
            return Ok(ResolvedBaseGame {
                path: canon,
                source: BaseGameSource::Manifest,
            });
        }
        debug!(
            path = %canon.display(),
            "Manifest target game_path does not exist; continuing search"
        );
    }

    // Tier 4: Profile declared game_path
    if let Some(path) = query.profile_path {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ws) = query.workspace {
            ws.join(path)
        } else {
            path.to_path_buf()
        };
        let canon = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if canon.exists() {
            return Ok(ResolvedBaseGame {
                path: canon,
                source: BaseGameSource::Profile,
            });
        }
        debug!(
            path = %canon.display(),
            "Profile declared game_path does not exist; continuing search"
        );
    }

    // Tier 5: Storefront provider auto-discovery (Steam)
    if let (Some(app_id), Some(provider)) = (query.steam_app_id, query.provider) {
        let roots = SteamDiscovery::known_steam_roots_with_config(provider);
        let mut libraries = Vec::new();
        for root in &roots {
            libraries.extend(SteamDiscovery::discover_library_folders_with_config(
                root, provider,
            ));
        }
        if let Some(app) =
            SteamDiscovery::find_installed_app_with_config(&libraries, app_id, provider)
        {
            if app.install_path.exists() {
                let canon = app.install_path.canonicalize().unwrap_or(app.install_path);
                info!(
                    app_id = app_id,
                    path = %canon.display(),
                    "Auto-discovered base game directory via Steam"
                );
                return Ok(ResolvedBaseGame {
                    path: canon,
                    source: BaseGameSource::SteamDiscovery,
                });
            }
        }
    }

    // Tier 6: Legacy workspace ./base_game directory
    if let Some(ws) = query.workspace {
        let base_game = ws.join("base_game");
        if base_game.exists() {
            let canon = base_game.canonicalize().unwrap_or(base_game);
            return Ok(ResolvedBaseGame {
                path: canon,
                source: BaseGameSource::WorkspaceFallback,
            });
        }
    }

    // Tier 7: Resolution failure
    let ws_display = query
        .workspace
        .map(|w| w.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let steam_info = match query.steam_app_id {
        Some(id) => format!(" (Steam App ID: {id})"),
        None => String::new(),
    };

    Err(VfsError::BaseGameNotFound(format!(
        "Could not resolve base game directory{steam_info}.\n\
         Checked resolution sources in order:\n  \
         1. CLI option '--base-path <DIR>'\n  \
         2. Environment variable PERI_BASE_GAME\n  \
         3. Manifest '[target] game_path' in peri.toml\n  \
         4. Profile 'game_path' in profile definition\n  \
         5. Storefront auto-discovery\n  \
         6. Legacy workspace folder: {ws_display}/base_game\n\n\
         Provide the installation path via '--base-path <DIR>' or set PERI_BASE_GAME=<DIR>."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_tier1_cli_precedence() {
        let temp = tempdir().unwrap();
        let cli_dir = temp.path().join("cli_game");
        let env_dir = temp.path().join("env_game");
        fs::create_dir_all(&cli_dir).unwrap();
        fs::create_dir_all(&env_dir).unwrap();

        let query = BaseGameQuery {
            cli_path: Some(&cli_dir),
            env_var: Some(env_dir.to_str().unwrap()),
            ..Default::default()
        };

        let resolved = resolve_base_game_path(&query).unwrap();
        assert_eq!(resolved.source, BaseGameSource::Cli);
        assert_eq!(resolved.path, cli_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_tier1_cli_missing_fails_fast() {
        let temp = tempdir().unwrap();
        let missing = temp.path().join("missing_game");

        let query = BaseGameQuery {
            cli_path: Some(&missing),
            ..Default::default()
        };

        let err = resolve_base_game_path(&query).unwrap_err();
        assert!(matches!(err, VfsError::BaseGameNotFound(_)));
        assert!(err.to_string().contains("--base-path"));
    }

    #[test]
    fn test_tier2_env_precedence() {
        let temp = tempdir().unwrap();
        let env_dir = temp.path().join("env_game");
        let ws_dir = temp.path().join("ws");
        let ws_base = ws_dir.join("base_game");
        fs::create_dir_all(&env_dir).unwrap();
        fs::create_dir_all(&ws_base).unwrap();

        let query = BaseGameQuery {
            env_var: Some(env_dir.to_str().unwrap()),
            workspace: Some(&ws_dir),
            ..Default::default()
        };

        let resolved = resolve_base_game_path(&query).unwrap();
        assert_eq!(resolved.source, BaseGameSource::Environment);
        assert_eq!(resolved.path, env_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_tier3_manifest_precedence() {
        let temp = tempdir().unwrap();
        let manifest_dir = temp.path().join("manifest_game");
        let profile_dir = temp.path().join("profile_game");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(&profile_dir).unwrap();

        let query = BaseGameQuery {
            manifest_path: Some(&manifest_dir),
            profile_path: Some(&profile_dir),
            ..Default::default()
        };

        let resolved = resolve_base_game_path(&query).unwrap();
        assert_eq!(resolved.source, BaseGameSource::Manifest);
        assert_eq!(resolved.path, manifest_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_tier4_profile_precedence() {
        let temp = tempdir().unwrap();
        let profile_dir = temp.path().join("profile_game");
        let ws_dir = temp.path().join("ws");
        let ws_base = ws_dir.join("base_game");
        fs::create_dir_all(&profile_dir).unwrap();
        fs::create_dir_all(&ws_base).unwrap();

        let query = BaseGameQuery {
            profile_path: Some(&profile_dir),
            workspace: Some(&ws_dir),
            ..Default::default()
        };

        let resolved = resolve_base_game_path(&query).unwrap();
        assert_eq!(resolved.source, BaseGameSource::Profile);
        assert_eq!(resolved.path, profile_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_tier6_workspace_fallback() {
        let temp = tempdir().unwrap();
        let ws_dir = temp.path().join("ws");
        let ws_base = ws_dir.join("base_game");
        fs::create_dir_all(&ws_base).unwrap();

        let query = BaseGameQuery {
            workspace: Some(&ws_dir),
            ..Default::default()
        };

        let resolved = resolve_base_game_path(&query).unwrap();
        assert_eq!(resolved.source, BaseGameSource::WorkspaceFallback);
        assert_eq!(resolved.path, ws_base.canonicalize().unwrap());
    }

    #[test]
    fn test_tier7_resolution_exhaustion_error() {
        let temp = tempdir().unwrap();
        let empty_ws = temp.path().join("empty_ws");
        fs::create_dir_all(&empty_ws).unwrap();

        let query = BaseGameQuery {
            workspace: Some(&empty_ws),
            steam_app_id: Some(489830),
            ..Default::default()
        };

        let err = resolve_base_game_path(&query).unwrap_err();
        assert!(matches!(err, VfsError::BaseGameNotFound(_)));
        assert!(err.to_string().contains("489830"));
        assert!(err.to_string().contains("PERI_BASE_GAME"));
    }
}
