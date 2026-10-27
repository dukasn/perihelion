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
use tracing::debug;

use peri_core::{GlobalConfig, ProviderConfig, SteamConfig};

/// Discovered installed Steam game from `appmanifest_*.acf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSteamApp {
    pub app_id: u64,
    pub name: String,
    pub installdir: String,
    pub install_path: PathBuf,
    pub executable: Option<String>,
    pub detected_engine: Option<String>,
}

/// Steam installation and library discovery engine.
pub struct SteamDiscovery;

impl SteamDiscovery {
    /// Expands `~` prefix to the current user home directory.
    pub fn expand_path(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str == "~" {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home);
            }
        } else if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(stripped);
            }
        }
        path.to_path_buf()
    }

    /// Default Steam platform provider configuration loaded from builtin discovery baseline.
    pub fn default_steam_provider() -> ProviderConfig {
        GlobalConfig::default().provider("steam")
    }

    /// Probes standard Linux paths for active Steam root installations with default configuration.
    pub fn known_steam_roots() -> Vec<PathBuf> {
        Self::known_steam_roots_with_config(&Self::default_steam_provider())
    }

    /// Probes paths for active Steam root installations based on declarative `SteamConfig`.
    pub fn known_steam_roots_with_config(config: &SteamConfig) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Ok(steam_dir) = std::env::var("STEAM_DIR") {
            let p = PathBuf::from(steam_dir);
            if p.exists() && p.is_dir() {
                roots.push(p);
            }
        }

        for candidate in &config.roots {
            let expanded = Self::expand_path(candidate);
            if expanded.exists() && expanded.is_dir() && !roots.contains(&expanded) {
                roots.push(expanded);
            }
        }

        roots
    }

    /// Finds the primary active Steam root installation with default configuration.
    pub fn find_active_steam_root() -> Option<PathBuf> {
        Self::find_active_steam_root_with_config(&Self::default_steam_provider())
    }

    /// Finds the primary active Steam root installation matching `SteamConfig`.
    pub fn find_active_steam_root_with_config(config: &SteamConfig) -> Option<PathBuf> {
        Self::known_steam_roots_with_config(config)
            .into_iter()
            .find(|p| p.join("steamapps").exists() || p.join("ubuntu12_32").exists())
    }

    /// Parses Steam's `libraryfolders.vdf` with default configuration.
    pub fn discover_library_folders(steam_root: &Path) -> Vec<PathBuf> {
        Self::discover_library_folders_with_config(steam_root, &Self::default_steam_provider())
    }

    /// Parses Steam's `libraryfolders.vdf` and appends any extra libraries configured declaratively.
    pub fn discover_library_folders_with_config(
        steam_root: &Path,
        config: &SteamConfig,
    ) -> Vec<PathBuf> {
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

        for extra in &config.libraries {
            let expanded = Self::expand_path(extra);
            if expanded.exists() && !libraries.contains(&expanded) {
                debug!(
                    path = %expanded.display(),
                    "Discovered configured extra Steam library folder"
                );
                libraries.push(expanded);
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

    /// Discovers all installed Steam applications across configured library folders.
    pub fn scan_installed_apps(libraries: &[PathBuf]) -> Vec<DiscoveredSteamApp> {
        Self::scan_installed_apps_with_config(libraries, &Self::default_steam_provider())
    }

    /// Discovers all installed Steam applications using declarative `SteamConfig`.
    pub fn scan_installed_apps_with_config(
        libraries: &[PathBuf],
        config: &SteamConfig,
    ) -> Vec<DiscoveredSteamApp> {
        let mut apps = Vec::new();

        for lib in libraries {
            let steamapps_dir = lib.join("steamapps");
            if let Ok(entries) = fs::read_dir(&steamapps_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                        if let Some(app) = Self::parse_manifest_file_with_config(&path, lib, config)
                        {
                            if !apps
                                .iter()
                                .any(|a: &DiscoveredSteamApp| a.app_id == app.app_id)
                            {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }

        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    /// Finds a specific installed Steam app by App ID across library folders.
    pub fn find_installed_app(libraries: &[PathBuf], app_id: u64) -> Option<DiscoveredSteamApp> {
        Self::find_installed_app_with_config(libraries, app_id, &Self::default_steam_provider())
    }

    /// Finds a specific installed Steam app by App ID using declarative `SteamConfig`.
    pub fn find_installed_app_with_config(
        libraries: &[PathBuf],
        app_id: u64,
        config: &SteamConfig,
    ) -> Option<DiscoveredSteamApp> {
        for lib in libraries {
            let manifest_path = lib.join(format!("steamapps/appmanifest_{app_id}.acf"));
            if manifest_path.exists() {
                if let Some(app) =
                    Self::parse_manifest_file_with_config(&manifest_path, lib, config)
                {
                    return Some(app);
                }
            }
        }
        None
    }

    /// Parses an individual `appmanifest_{app_id}.acf` file with default config.
    pub fn parse_manifest_file(
        manifest_path: &Path,
        library_root: &Path,
    ) -> Option<DiscoveredSteamApp> {
        Self::parse_manifest_file_with_config(
            manifest_path,
            library_root,
            &Self::default_steam_provider(),
        )
    }

    /// Parses an individual `appmanifest_{app_id}.acf` file using declarative `SteamConfig`.
    pub fn parse_manifest_file_with_config(
        manifest_path: &Path,
        library_root: &Path,
        config: &SteamConfig,
    ) -> Option<DiscoveredSteamApp> {
        let content = fs::read_to_string(manifest_path).ok()?;
        let mut app_id = None;
        let mut name = None;
        let mut installdir = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"appid\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    app_id = parts[3].parse::<u64>().ok();
                }
            } else if trimmed.starts_with("\"name\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    name = Some(parts[3].to_string());
                }
            } else if trimmed.starts_with("\"installdir\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    installdir = Some(parts[3].to_string());
                }
            }
        }

        if app_id.is_none() {
            if let Some(file_name) = manifest_path.file_name().and_then(|s| s.to_str()) {
                if let Some(stripped) = file_name
                    .strip_prefix("appmanifest_")
                    .and_then(|s| s.strip_suffix(".acf"))
                {
                    app_id = stripped.parse::<u64>().ok();
                }
            }
        }

        let app_id = app_id?;
        let name = name.unwrap_or_else(|| format!("Steam App {app_id}"));
        let installdir = installdir.unwrap_or_default();
        let install_path = library_root.join("steamapps/common").join(&installdir);

        let (executable, detected_engine) = if install_path.exists() {
            let global_cfg = GlobalConfig::default();
            let engine =
                crate::engine::detect_engine_layout(&install_path, global_cfg.signatures());
            let exe = if let Some(ref eng) = engine {
                crate::engine::find_executable_for_engine(&install_path, eng)
                    .or_else(|| Self::find_executable_candidate_with_config(&install_path, config))
            } else {
                Self::find_executable_candidate_with_config(&install_path, config)
            };
            (exe, engine.map(|e| e.id))
        } else {
            (None, None)
        };

        Some(DiscoveredSteamApp {
            app_id,
            name,
            installdir,
            install_path,
            executable,
            detected_engine,
        })
    }

    /// Probes a game's installation directory for potential executable binaries with default config.
    pub fn find_executable_candidate(game_dir: &Path) -> Option<String> {
        Self::find_executable_candidate_with_config(game_dir, &Self::default_steam_provider())
    }

    /// Probes a game's installation directory for potential executable binaries using declarative `SteamConfig`.
    pub fn find_executable_candidate_with_config(
        game_dir: &Path,
        config: &SteamConfig,
    ) -> Option<String> {
        let is_match = |path: &Path| -> bool {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                config
                    .binary_extensions
                    .iter()
                    .any(|e| ext.eq_ignore_ascii_case(e))
            } else {
                false
            }
        };

        // 1. Root direct search
        if let Ok(entries) = fs::read_dir(game_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_match(&path) {
                    if let Ok(rel) = path.strip_prefix(game_dir) {
                        return Some(rel.to_string_lossy().into_owned());
                    }
                }
            }
        }

        // 2. Scan configured nested binary subdirectories
        for sub in &config.binary_subdirs {
            let sub_path = game_dir.join(sub);
            if sub_path.exists() {
                if let Ok(entries) = fs::read_dir(&sub_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() && is_match(&path) {
                            if let Ok(rel) = path.strip_prefix(game_dir) {
                                return Some(rel.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }

        None
    }
}
