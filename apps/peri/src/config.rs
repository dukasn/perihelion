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
use std::path::PathBuf;

use directories::ProjectDirs;
use peri_core::{GlobalConfig, CONFIG_FILENAME};
use tracing::{debug, warn};

pub fn resolve_config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "perihelion", "perihelion") {
        proj_dirs.config_dir().to_path_buf()
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/perihelion")
    } else {
        PathBuf::from(".config/perihelion")
    }
}

pub fn resolve_home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

pub fn load_global_config() -> GlobalConfig {
    let config_dir = resolve_config_dir();
    let config_file = config_dir.join(CONFIG_FILENAME);

    if config_file.exists() {
        if let Ok(content) = fs::read_to_string(&config_file) {
            match GlobalConfig::from_toml_str(&content) {
                Ok(cfg) => {
                    debug!(path = %config_file.display(), "Loaded global configuration");
                    return cfg;
                }
                Err(e) => {
                    warn!(
                        path = %config_file.display(),
                        error = %e,
                        "Failed to parse config.toml; falling back to defaults"
                    );
                }
            }
        }
    }

    if let Some(home) = resolve_home_dir() {
        GlobalConfig::default_for_home(&home)
    } else {
        GlobalConfig::default()
    }
}
