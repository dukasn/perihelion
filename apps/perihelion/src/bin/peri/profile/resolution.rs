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

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use peri_core::profile::GameProfile;

use crate::PROFILES_DIR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    Workspace,
    UserConfig,
    UserData,
    System,
    Generic,
}

impl fmt::Display for ProfileSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileSource::Workspace => write!(f, "Workspace"),
            ProfileSource::UserConfig => write!(f, "User Config"),
            ProfileSource::UserData => write!(f, "User Data"),
            ProfileSource::System => write!(f, "System"),
            ProfileSource::Generic => write!(f, "Builtin"),
        }
    }
}

pub struct DiscoveredProfile {
    pub profile: GameProfile,
    pub source: ProfileSource,
    #[allow(dead_code)]
    pub path: Option<PathBuf>,
    pub is_installed: bool,
}

/// Returns the ordered cascade of directory paths and their profile sources.
pub fn profile_search_dirs(workspace: &Path) -> Vec<(PathBuf, ProfileSource)> {
    let mut dirs = Vec::new();

    // 1. Workspace profiles: <workspace>/.perihelion/profiles
    dirs.push((workspace.join(PROFILES_DIR), ProfileSource::Workspace));

    // 2. User config directory: ~/.config/perihelion/profiles
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        dirs.push((
            PathBuf::from(xdg_config).join("perihelion/profiles"),
            ProfileSource::UserConfig,
        ));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push((
            PathBuf::from(home).join(".config/perihelion/profiles"),
            ProfileSource::UserConfig,
        ));
    }

    // 3. User data directory: ~/.local/share/perihelion/profiles
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        dirs.push((
            PathBuf::from(xdg_data).join("perihelion/profiles"),
            ProfileSource::UserData,
        ));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push((
            PathBuf::from(home).join(".local/share/perihelion/profiles"),
            ProfileSource::UserData,
        ));
    }

    // 4. System data directories: /usr/local/share and /usr/share
    dirs.push((
        PathBuf::from("/usr/local/share/perihelion/profiles"),
        ProfileSource::System,
    ));
    dirs.push((
        PathBuf::from("/usr/share/perihelion/profiles"),
        ProfileSource::System,
    ));

    dirs
}

/// Loads all valid `.toml` profile definitions from a directory into a buffer.
pub fn load_profiles_from_dir(dir: &Path, profiles: &mut Vec<GameProfile>) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(profile) = GameProfile::from_toml_str(&content) {
                        profiles.push(profile);
                    }
                }
            }
        }
    }
}

/// Resolves a `GameProfile` by identifier, evaluating cascading workspace, user,
/// and system profile directories before falling back to a generic baseline.
pub fn resolve_game_profile(workspace: &Path, game_id: &str) -> GameProfile {
    let mut custom_profiles = Vec::new();
    for (dir, _) in profile_search_dirs(workspace) {
        load_profiles_from_dir(&dir, &mut custom_profiles);
    }
    GameProfile::resolve(game_id, &custom_profiles)
}

/// Finds a game profile file and its provenance across cascading search directories.
pub fn find_profile_with_source(
    workspace: &Path,
    game_id: &str,
) -> (GameProfile, Option<PathBuf>, ProfileSource) {
    for (dir, source) in profile_search_dirs(workspace) {
        let file = dir.join(format!("{game_id}.toml"));
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(profile) = GameProfile::from_toml_str(&content) {
                    return (profile, Some(file), source);
                }
            }
        }
    }

    (GameProfile::generic(game_id), None, ProfileSource::Generic)
}

/// Scans all cascading profile directories, deduplicating profiles and checking installation status.
pub fn scan_all_profiles(workspace: &Path, steam_libs: &[PathBuf]) -> Vec<DiscoveredProfile> {
    let mut discovered = Vec::new();

    for (dir, source) in profile_search_dirs(workspace) {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(profile) = GameProfile::from_toml_str(&content) {
                            if !discovered.iter().any(|p: &DiscoveredProfile| {
                                p.profile.id.eq_ignore_ascii_case(&profile.id)
                            }) {
                                let is_installed = is_game_installed(&profile, steam_libs);
                                discovered.push(DiscoveredProfile {
                                    profile,
                                    source,
                                    path: Some(path),
                                    is_installed,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Append built-in generic fallback profile
    discovered.push(DiscoveredProfile {
        profile: GameProfile::generic("generic"),
        source: ProfileSource::Generic,
        path: None,
        is_installed: true,
    });

    discovered
}

/// Checks whether a game is installed in the given Steam library directories.
pub fn is_game_installed(profile: &GameProfile, steam_libs: &[PathBuf]) -> bool {
    if let Some(app_id) = profile.steam_app_id {
        for lib in steam_libs {
            if lib
                .join(format!("steamapps/appmanifest_{app_id}.acf"))
                .exists()
            {
                return true;
            }
        }
    }
    for lib in steam_libs {
        if lib.join("steamapps/common").join(&profile.name).exists() {
            return true;
        }
    }
    false
}
