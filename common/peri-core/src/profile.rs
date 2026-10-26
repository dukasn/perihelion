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

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::ProfileError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutMapping {
    pub from_prefix: String,
    pub to_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steam_app_id: Option<u64>,
    #[serde(default)]
    pub default_executable: Option<String>,
    #[serde(default)]
    pub mappings: Vec<LayoutMapping>,
}

impl GameProfile {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            steam_app_id: None,
            default_executable: None,
            mappings: Vec::new(),
        }
    }

    pub fn from_toml_str(input: &str) -> Result<Self, ProfileError> {
        toml::from_str(input).map_err(|e| ProfileError::ParseError(e.to_string()))
    }

    pub fn to_toml_str(&self) -> Result<String, ProfileError> {
        toml::to_string_pretty(self).map_err(|e| ProfileError::ParseError(e.to_string()))
    }

    pub fn cyberpunk_2077() -> Self {
        Self {
            id: "cyberpunk_2077".to_string(),
            name: "Cyberpunk 2077".to_string(),
            steam_app_id: Some(1091500),
            default_executable: Some("bin/x64/Cyberpunk2077.exe".to_string()),
            mappings: vec![
                LayoutMapping {
                    from_prefix: "archive".to_string(),
                    to_prefix: "archive".to_string(),
                },
                LayoutMapping {
                    from_prefix: "bin".to_string(),
                    to_prefix: "bin".to_string(),
                },
                LayoutMapping {
                    from_prefix: "r6".to_string(),
                    to_prefix: "r6".to_string(),
                },
                LayoutMapping {
                    from_prefix: "mods".to_string(),
                    to_prefix: "archive/pc/mod".to_string(),
                },
            ],
        }
    }

    pub fn skyrim_se() -> Self {
        Self {
            id: "skyrim_se".to_string(),
            name: "The Elder Scrolls V: Skyrim Special Edition".to_string(),
            steam_app_id: Some(489830),
            default_executable: Some("SkyrimSE.exe".to_string()),
            mappings: vec![
                LayoutMapping {
                    from_prefix: "Data".to_string(),
                    to_prefix: "Data".to_string(),
                },
                LayoutMapping {
                    from_prefix: "data".to_string(),
                    to_prefix: "Data".to_string(),
                },
                LayoutMapping {
                    from_prefix: "SKSE".to_string(),
                    to_prefix: "Data/SKSE".to_string(),
                },
                LayoutMapping {
                    from_prefix: "skse".to_string(),
                    to_prefix: "Data/SKSE".to_string(),
                },
            ],
        }
    }

    pub fn starfield() -> Self {
        Self {
            id: "starfield".to_string(),
            name: "Starfield".to_string(),
            steam_app_id: Some(1716740),
            default_executable: Some("Starfield.exe".to_string()),
            mappings: vec![
                LayoutMapping {
                    from_prefix: "Data".to_string(),
                    to_prefix: "Data".to_string(),
                },
                LayoutMapping {
                    from_prefix: "data".to_string(),
                    to_prefix: "Data".to_string(),
                },
            ],
        }
    }

    pub fn generic(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            steam_app_id: None,
            default_executable: None,
            mappings: Vec::new(),
        }
    }

    pub fn builtin_profiles() -> Vec<Self> {
        vec![Self::cyberpunk_2077(), Self::skyrim_se(), Self::starfield()]
    }

    pub fn resolve(id: &str, custom_profiles: &[Self]) -> Self {
        for profile in custom_profiles {
            if profile.id.eq_ignore_ascii_case(id) {
                return profile.clone();
            }
        }

        for builtin in Self::builtin_profiles() {
            if builtin.id.eq_ignore_ascii_case(id) {
                return builtin;
            }
        }

        Self::generic(id)
    }

    pub fn map_path(&self, relative_path: &Path) -> PathBuf {
        let path_str = relative_path.to_string_lossy();
        for mapping in &self.mappings {
            if path_str.starts_with(&mapping.from_prefix) {
                let remainder = path_str.strip_prefix(&mapping.from_prefix).unwrap_or("");
                let remainder = remainder.trim_start_matches('/');
                return PathBuf::from(&mapping.to_prefix).join(remainder);
            }
        }
        relative_path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_profile_mapping() {
        let profile = GameProfile::cyberpunk_2077();
        let mod_path = Path::new("mods/my_mod.archive");
        let mapped = profile.map_path(mod_path);
        assert_eq!(mapped, PathBuf::from("archive/pc/mod/my_mod.archive"));
    }

    #[test]
    fn test_skyrim_se_mapping() {
        let profile = GameProfile::skyrim_se();
        let mod_path = Path::new("SKSE/Plugins/engine_fixes.dll");
        let mapped = profile.map_path(mod_path);
        assert_eq!(mapped, PathBuf::from("Data/SKSE/Plugins/engine_fixes.dll"));
    }

    #[test]
    fn test_profile_toml_serde() {
        let sample = r#"
id = "fallout4"
name = "Fallout 4"
steam_app_id = 377160
default_executable = "Fallout4.exe"

[[mappings]]
from_prefix = "Data"
to_prefix = "Data"
"#;
        let profile = GameProfile::from_toml_str(sample).expect("Should parse TOML profile");
        assert_eq!(profile.id, "fallout4");
        assert_eq!(profile.steam_app_id, Some(377160));
        assert_eq!(profile.default_executable, Some("Fallout4.exe".to_string()));

        let serialized = profile.to_toml_str().expect("Should serialize TOML");
        assert!(serialized.contains("fallout4"));
    }

    #[test]
    fn test_resolve_precedence() {
        let custom = GameProfile {
            id: "cyberpunk_2077".to_string(),
            name: "Custom Cyberpunk".to_string(),
            steam_app_id: None,
            default_executable: Some("custom.exe".to_string()),
            mappings: Vec::new(),
        };

        let resolved = GameProfile::resolve("cyberpunk_2077", &[custom]);
        assert_eq!(resolved.name, "Custom Cyberpunk");
        assert_eq!(resolved.default_executable, Some("custom.exe".to_string()));
    }
}
