use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    pub default_executable: Option<String>,
    #[serde(default)]
    pub mappings: Vec<LayoutMapping>,
}

impl GameProfile {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            default_executable: None,
            mappings: Vec::new(),
        }
    }

    pub fn cyberpunk_2077() -> Self {
        Self {
            id: "cyberpunk_2077".to_string(),
            name: "Cyberpunk 2077".to_string(),
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

    pub fn generic(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            default_executable: None,
            mappings: Vec::new(),
        }
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
    fn test_default_passthrough() {
        let profile = GameProfile::generic("custom");
        let path = Path::new("some/file.txt");
        assert_eq!(profile.map_path(path), PathBuf::from("some/file.txt"));
    }
}
