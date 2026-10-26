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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

pub const CONFIG_FILENAME: &str = "config.toml";

fn default_upload_limit() -> u32 {
    1024
}

fn default_true() -> bool {
    true
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from("~/.local/share/perihelion/cache")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_upload_limit")]
    pub daemon_upload_limit_kbs: u32,
    #[serde(default = "default_true")]
    pub enable_dht: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            daemon_upload_limit_kbs: default_upload_limit(),
            enable_dht: default_true(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_cache_dir")]
    pub global_cache_dir: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            global_cache_dir: default_cache_dir(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub storage: StorageConfig,
}

impl GlobalConfig {
    /// Constructs default configuration anchored to a specific home directory,
    /// following the XDG Base Directory standard (`~/.local/share/perihelion/cache`).
    pub fn default_for_home(home: &Path) -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig {
                global_cache_dir: home.join(".local/share/perihelion/cache"),
            },
        }
    }

    /// Parses global configuration from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Serializes global configuration to a formatted TOML string.
    pub fn to_toml_str(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_default() {
        let cfg = GlobalConfig::default();
        assert_eq!(cfg.network.daemon_upload_limit_kbs, 1024);
        assert!(cfg.network.enable_dht);
        assert_eq!(
            cfg.storage.global_cache_dir,
            PathBuf::from("~/.local/share/perihelion/cache")
        );
    }

    #[test]
    fn test_global_config_for_home() {
        let home = Path::new("/var/home/dukasn");
        let cfg = GlobalConfig::default_for_home(home);
        assert_eq!(
            cfg.storage.global_cache_dir,
            PathBuf::from("/var/home/dukasn/.local/share/perihelion/cache")
        );
    }

    #[test]
    fn test_global_config_serde_roundtrip() {
        let toml_data = r#"
[network]
daemon_upload_limit_kbs = 2048
enable_dht = false

[storage]
global_cache_dir = "/custom/cache/dir"
"#;
        let cfg = GlobalConfig::from_toml_str(toml_data).expect("should parse valid toml");
        assert_eq!(cfg.network.daemon_upload_limit_kbs, 2048);
        assert!(!cfg.network.enable_dht);
        assert_eq!(
            cfg.storage.global_cache_dir,
            PathBuf::from("/custom/cache/dir")
        );

        let output = cfg.to_toml_str().expect("should serialize");
        assert!(output.contains("daemon_upload_limit_kbs = 2048"));
        assert!(output.contains("enable_dht = false"));
    }
}
