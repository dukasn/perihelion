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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, info_span};

use peri_core::SteamConfig;

use super::discovery::SteamDiscovery;
use crate::VfsError;

/// Discovered Proton / Wine compatibility tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonTool {
    pub name: String,
    pub path: PathBuf,
    pub runner: PathBuf,
}

/// Fully configured Proton launch environment ready to execute inside or outside a namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonLaunch {
    pub runner: PathBuf,
    pub compat_data_path: PathBuf,
    pub wine_prefix: PathBuf,
    pub steam_client_path: PathBuf,
    pub env_vars: Vec<(String, String)>,
}

impl ProtonLaunch {
    /// Configures a `std::process::Command` with the full Proton headless execution payload.
    pub fn configure_command(&self, command: &mut Command, game_executable: &str, args: &[String]) {
        command.arg(&self.runner).arg("run").arg(game_executable);
        for arg in args {
            command.arg(arg);
        }

        for (k, v) in &self.env_vars {
            command.env(k, v);
        }
    }
}

impl SteamDiscovery {
    /// Discovers all available Proton versions across Steam libraries and compatibility tool directories.
    pub fn discover_proton_versions(steam_root: &Path, libraries: &[PathBuf]) -> Vec<ProtonTool> {
        Self::discover_proton_versions_with_config(
            steam_root,
            libraries,
            &Self::default_steam_provider(),
        )
    }

    /// Discovers all available Proton versions using configured compatibility directories.
    pub fn discover_proton_versions_with_config(
        steam_root: &Path,
        libraries: &[PathBuf],
        config: &SteamConfig,
    ) -> Vec<ProtonTool> {
        let mut tools = Vec::new();

        // 1. Scan configured compatibility tool directories (Proton-GE, GE-Proton, custom Wine)
        for compat_dir in &config.compatibility_dirs {
            let resolved_dir = if compat_dir.is_relative() {
                steam_root.join(compat_dir)
            } else {
                Self::expand_path(compat_dir)
            };

            if let Ok(entries) = fs::read_dir(&resolved_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let runner = path.join("proton");
                    if runner.exists() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if !tools.iter().any(|t: &ProtonTool| t.path == path) {
                            tools.push(ProtonTool { name, path, runner });
                        }
                    }
                }
            }
        }

        // 2. Scan official Proton versions in steamapps/common across all libraries
        for lib in libraries {
            let common_dir = lib.join("steamapps/common");
            if let Ok(entries) = fs::read_dir(common_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with("Proton") {
                        let path = entry.path();
                        let runner = path.join("proton");
                        if runner.exists() && !tools.iter().any(|t: &ProtonTool| t.path == path) {
                            tools.push(ProtonTool { name, path, runner });
                        }
                    }
                }
            }
        }

        // Sort descending so newer/GE versions appear first
        tools.sort_by(|a, b| b.name.cmp(&a.name));
        tools
    }

    /// Builds a complete `ProtonLaunch` payload resolving Steam roots, compatdata prefixes, and Proton binaries.
    pub fn resolve_proton_launch(
        steam_app_id: Option<u64>,
        explicit_proton: Option<&Path>,
        explicit_prefix: Option<&Path>,
    ) -> Result<ProtonLaunch, VfsError> {
        Self::resolve_proton_launch_with_config(
            steam_app_id,
            explicit_proton,
            explicit_prefix,
            &Self::default_steam_provider(),
        )
    }

    /// Builds a complete `ProtonLaunch` payload using declarative `SteamConfig`.
    pub fn resolve_proton_launch_with_config(
        steam_app_id: Option<u64>,
        explicit_proton: Option<&Path>,
        explicit_prefix: Option<&Path>,
        config: &SteamConfig,
    ) -> Result<ProtonLaunch, VfsError> {
        let span = info_span!("resolve_proton_launch", ?steam_app_id);
        let _enter = span.enter();
        debug!("Resolving Proton launch configuration");

        let steam_root = Self::find_active_steam_root_with_config(config).unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".local/share/Steam")
        });

        let libraries = Self::discover_library_folders_with_config(&steam_root, config);

        // 1. Resolve Proton binary
        let runner = if let Some(explicit) = explicit_proton {
            if explicit.is_file() {
                explicit.to_path_buf()
            } else if explicit.join("proton").exists() {
                explicit.join("proton")
            } else {
                return Err(VfsError::ProtonDiscoveryFailed(format!(
                    "Explicit Proton path not found or missing 'proton' binary: {}",
                    explicit.display()
                )));
            }
        } else {
            let available =
                Self::discover_proton_versions_with_config(&steam_root, &libraries, config);
            if let Some(first) = available.into_iter().next() {
                info!(
                    proton = %first.name,
                    path = %first.runner.display(),
                    "Selected Proton runtime"
                );
                first.runner
            } else {
                return Err(VfsError::ProtonDiscoveryFailed(
                    "No Proton versions found in Steam installations or compatibilitytools.d. Please specify --proton <path>.".to_string(),
                ));
            }
        };

        // 2. Resolve CompatData / WinePrefix
        let (compat_data_path, wine_prefix) = if let Some(explicit) = explicit_prefix {
            let pfx = if explicit.ends_with("pfx") {
                explicit.to_path_buf()
            } else {
                explicit.join("pfx")
            };
            (explicit.to_path_buf(), pfx)
        } else if let Some(app_id) = steam_app_id {
            if let Some(found) = Self::find_compat_data(&libraries, app_id) {
                let pfx = found.join("pfx");
                (found, pfx)
            } else {
                // Default fallback inside primary steamapps
                let fallback = steam_root.join(format!("steamapps/compatdata/{app_id}"));
                let pfx = fallback.join("pfx");
                (fallback, pfx)
            }
        } else {
            let default_compat = steam_root.join("steamapps/compatdata/perihelion_default");
            let pfx = default_compat.join("pfx");
            (default_compat, pfx)
        };

        // 3. Assemble environment variables
        let mut env_vars = vec![
            (
                "STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(),
                steam_root.to_string_lossy().into_owned(),
            ),
            (
                "STEAM_COMPAT_DATA_PATH".to_string(),
                compat_data_path.to_string_lossy().into_owned(),
            ),
            (
                "WINEPREFIX".to_string(),
                wine_prefix.to_string_lossy().into_owned(),
            ),
        ];

        if let Some(app_id) = steam_app_id {
            env_vars.push(("SteamAppId".to_string(), app_id.to_string()));
            env_vars.push(("SteamGameId".to_string(), app_id.to_string()));
        }

        // Pass-through configured gaming graphics, layers & runtime variables if set
        for var in &config.environment {
            if let Ok(val) = std::env::var(var) {
                env_vars.push((var.clone(), val));
            }
        }

        Ok(ProtonLaunch {
            runner,
            compat_data_path,
            wine_prefix,
            steam_client_path: steam_root,
            env_vars,
        })
    }
}
