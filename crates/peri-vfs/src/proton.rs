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
use tracing::{debug, info};

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

/// Steam installation and library discovery engine.
pub struct SteamDiscovery;

impl SteamDiscovery {
    /// Probes standard Linux paths for active Steam root installations (both native and Flatpak).
    pub fn known_steam_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Ok(steam_dir) = std::env::var("STEAM_DIR") {
            roots.push(PathBuf::from(steam_dir));
        }

        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            roots.push(home_path.join(".local/share/Steam"));
            roots.push(home_path.join(".steam/root"));
            roots.push(home_path.join(".steam/steam"));
            roots.push(home_path.join(".var/app/com.valvesoftware.Steam/data/Steam"));
            roots.push(home_path.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        }

        roots
            .into_iter()
            .filter(|p| p.exists() && p.is_dir())
            .collect()
    }

    /// Finds the primary active Steam root installation.
    pub fn find_active_steam_root() -> Option<PathBuf> {
        Self::known_steam_roots()
            .into_iter()
            .find(|p| p.join("steamapps").exists() || p.join("ubuntu12_32").exists())
    }

    /// Parses Steam's `libraryfolders.vdf` to locate all configured Steam library folders across drives.
    pub fn discover_library_folders(steam_root: &Path) -> Vec<PathBuf> {
        let mut libraries = vec![steam_root.to_path_buf()];
        let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");

        if let Ok(content) = fs::read_to_string(&vdf_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 4 {
                        let lib_path = PathBuf::from(parts[3]);
                        if lib_path.exists() && !libraries.contains(&lib_path) {
                            debug!(path = %lib_path.display(), "Discovered Steam library folder");
                            libraries.push(lib_path);
                        }
                    }
                }
            }
        }

        libraries
    }

    /// Finds the `compatdata/{app_id}` directory across all Steam library folders.
    pub fn find_compat_data(libraries: &[PathBuf], app_id: u64) -> Option<PathBuf> {
        for lib in libraries {
            let candidate = lib.join(format!("steamapps/compatdata/{app_id}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Discovers all available Proton versions across Steam libraries and compatibility tool directories.
    pub fn discover_proton_versions(steam_root: &Path, libraries: &[PathBuf]) -> Vec<ProtonTool> {
        let mut tools = Vec::new();

        // 1. Scan compatibilitytools.d (Community builds: GE-Proton, Proton-GE, etc.)
        let compat_dirs = vec![
            steam_root.join("compatibilitytools.d"),
            steam_root.join("root/compatibilitytools.d"),
        ];

        for compat_dir in compat_dirs {
            if let Ok(entries) = fs::read_dir(compat_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let runner = path.join("proton");
                    if runner.exists() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        tools.push(ProtonTool { name, path, runner });
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
                        if runner.exists() {
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
        let steam_root = Self::find_active_steam_root().unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".local/share/Steam")
        });

        let libraries = Self::discover_library_folders(&steam_root);

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
            let available = Self::discover_proton_versions(&steam_root, &libraries);
            if let Some(first) = available.into_iter().next() {
                info!(proton = %first.name, path = %first.runner.display(), "Selected Proton runtime");
                first.runner
            } else {
                return Err(VfsError::ProtonDiscoveryFailed(
                    "No Proton versions found in Steam installations or compatibilitytools.d. Please specify --proton <path>.".to_string()
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

        // Pass-through standard Linux gaming graphics & layer variables if set
        let passthrough_vars = [
            "VK_ICD_FILENAMES",
            "VK_LAYER_PATH",
            "MANGOHUD",
            "MANGOHUD_CONFIG",
            "DXVK_HUD",
            "ENABLE_VKBASALT",
            "PROTON_ENABLE_NVAPI",
            "WINE_FULLSCREEN_FSR",
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "PULSE_SERVER",
        ];

        for var in passthrough_vars {
            if let Ok(val) = std::env::var(var) {
                env_vars.push((var.to_string(), val));
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_discover_library_folders_vdf() {
        let tmp = tempdir().unwrap();
        let steamapps = tmp.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();

        let vdf_content = r#"
"libraryfolders"
{
    "0"
    {
        "path"      "/mnt/games/FastSSD"
        "label"     ""
    }
    "1"
    {
        "path"      "/mnt/storage/Secondary"
        "label"     ""
    }
}
"#;
        fs::write(steamapps.join("libraryfolders.vdf"), vdf_content).unwrap();

        let libs = SteamDiscovery::discover_library_folders(tmp.path());
        assert_eq!(libs[0], tmp.path());
    }

    #[test]
    fn test_resolve_proton_launch_with_explicit_paths() {
        let tmp = tempdir().unwrap();
        let proton_dir = tmp.path().join("Proton-GE");
        fs::create_dir_all(&proton_dir).unwrap();
        let runner = proton_dir.join("proton");
        fs::write(&runner, "#!/bin/sh\necho running").unwrap();

        let prefix_dir = tmp.path().join("prefix");
        fs::create_dir_all(&prefix_dir).unwrap();

        let launch =
            SteamDiscovery::resolve_proton_launch(Some(489830), Some(&runner), Some(&prefix_dir))
                .expect("Explicit proton resolution should succeed");

        assert_eq!(launch.runner, runner);
        assert_eq!(launch.compat_data_path, prefix_dir);
        assert_eq!(launch.wine_prefix, prefix_dir.join("pfx"));

        let env_map: std::collections::HashMap<_, _> = launch.env_vars.into_iter().collect();
        assert_eq!(env_map.get("SteamAppId").unwrap(), "489830");
        assert!(env_map.contains_key("STEAM_COMPAT_DATA_PATH"));
        assert!(env_map.contains_key("WINEPREFIX"));
    }
}
