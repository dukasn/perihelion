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

use directories::ProjectDirs;
use peri_core::{EngineSignature, GlobalConfig, SignatureRegistry, CONFIG_FILENAME};
use serde::Deserialize;
use tracing::{debug, warn};

#[derive(Deserialize)]
struct SignaturesDocument {
    #[serde(default)]
    signatures: Vec<EngineSignature>,
}

pub fn load_signatures_from_dir(dir: &Path, registry: &mut SignatureRegistry) {
    if !dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(doc) = toml::from_str::<SignaturesDocument>(&content) {
                        for sig in doc.signatures {
                            registry.register(sig);
                        }
                    } else if let Ok(sig) = toml::from_str::<EngineSignature>(&content) {
                        registry.register(sig);
                    }
                }
            }
        }
    }
}

pub fn load_cascading_signatures(config_dir: &Path) -> SignatureRegistry {
    let mut registry = SignatureRegistry::new();

    // 1. System tiers
    for sys_dir in &[
        Path::new("/usr/share/perihelion/signatures"),
        Path::new("/usr/local/share/perihelion/signatures"),
    ] {
        load_signatures_from_dir(sys_dir, &mut registry);
    }

    // 2. User data tier
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        load_signatures_from_dir(
            &PathBuf::from(xdg_data).join("perihelion/signatures"),
            &mut registry,
        );
    } else if let Ok(home) = std::env::var("HOME") {
        load_signatures_from_dir(
            &PathBuf::from(home).join(".local/share/perihelion/signatures"),
            &mut registry,
        );
    }

    // 3. User config tier
    load_signatures_from_dir(&config_dir.join("signatures"), &mut registry);

    registry
}

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

    let mut cfg = if config_file.exists() {
        if let Ok(content) = fs::read_to_string(&config_file) {
            match GlobalConfig::from_toml_str(&content) {
                Ok(loaded) => {
                    debug!(path = %config_file.display(), "Loaded global configuration");
                    loaded
                }
                Err(e) => {
                    warn!(
                        path = %config_file.display(),
                        error = %e,
                        "Failed to parse config.toml; falling back to defaults"
                    );
                    if let Some(home) = resolve_home_dir() {
                        GlobalConfig::default_for_home(&home)
                    } else {
                        GlobalConfig::default()
                    }
                }
            }
        } else if let Some(home) = resolve_home_dir() {
            GlobalConfig::default_for_home(&home)
        } else {
            GlobalConfig::default()
        }
    } else if let Some(home) = resolve_home_dir() {
        GlobalConfig::default_for_home(&home)
    } else {
        GlobalConfig::default()
    };

    let cascading_signatures = load_cascading_signatures(&config_dir);
    cfg.signatures.merge(cascading_signatures);

    cfg
}
