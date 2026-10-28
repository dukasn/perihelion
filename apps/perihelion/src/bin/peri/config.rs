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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use peri_core::{
    ConfigError, EngineSignature, GlobalConfig, ProviderConfig, SignatureRegistry, CONFIG_FILENAME,
};
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize)]
struct SignaturesDocument {
    #[serde(default)]
    signatures: Vec<EngineSignature>,
}

#[derive(Deserialize)]
struct ProvidersDocument {
    #[serde(default)]
    providers: BTreeMap<String, ProviderConfig>,
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
                        if !doc.signatures.is_empty() {
                            for sig in doc.signatures {
                                registry.register(sig);
                            }
                            continue;
                        }
                    }
                    if let Ok(sig) = toml::from_str::<EngineSignature>(&content) {
                        registry.register(sig);
                    }
                }
            }
        }
    }
}

pub fn load_providers_from_dir(dir: &Path, providers: &mut BTreeMap<String, ProviderConfig>) {
    if !dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(doc) = toml::from_str::<ProvidersDocument>(&content) {
                        if !doc.providers.is_empty() {
                            for (k, v) in doc.providers {
                                providers.insert(k, v);
                            }
                            continue;
                        }
                    }
                    if let Ok(provider) = toml::from_str::<ProviderConfig>(&content) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            providers.insert(stem.to_string(), provider);
                        }
                    }
                }
            }
        }
    }
}

pub fn load_cascading_signatures(config_dir: &Path, workspace: Option<&Path>) -> SignatureRegistry {
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

    // 4. Workspace tier
    if let Some(ws) = workspace {
        load_signatures_from_dir(&ws.join(".perihelion/signatures"), &mut registry);
    }
    load_signatures_from_dir(Path::new(".perihelion/signatures"), &mut registry);

    registry
}

pub fn load_cascading_providers(
    config_dir: &Path,
    workspace: Option<&Path>,
) -> BTreeMap<String, ProviderConfig> {
    let mut providers = BTreeMap::new();

    // 1. System tiers
    for sys_dir in &[
        Path::new("/usr/share/perihelion/providers"),
        Path::new("/usr/local/share/perihelion/providers"),
    ] {
        load_providers_from_dir(sys_dir, &mut providers);
    }

    // 2. User data tier
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        load_providers_from_dir(
            &PathBuf::from(xdg_data).join("perihelion/providers"),
            &mut providers,
        );
    } else if let Ok(home) = std::env::var("HOME") {
        load_providers_from_dir(
            &PathBuf::from(home).join(".local/share/perihelion/providers"),
            &mut providers,
        );
    }

    // 3. User config tier
    load_providers_from_dir(&config_dir.join("providers"), &mut providers);

    // 4. Workspace tier
    if let Some(ws) = workspace {
        load_providers_from_dir(&ws.join(".perihelion/providers"), &mut providers);
    }
    load_providers_from_dir(Path::new(".perihelion/providers"), &mut providers);

    providers
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

/// Reads `config_file`; `Ok(None)` means it doesn't exist. Invalid or unreadable files are errors.
fn read_config_file(config_file: &Path) -> Result<Option<GlobalConfig>, ConfigError> {
    if !config_file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_file)
        .map_err(|e| ConfigError::Unreadable(format!("{}: {e}", config_file.display())))?;
    let cfg = GlobalConfig::from_toml_str(&content).map_err(|e| match e {
        ConfigError::ParseError(msg) => {
            ConfigError::ParseError(format!("{}: {msg}", config_file.display()))
        }
        other => other,
    })?;
    debug!(path = %config_file.display(), "Loaded global configuration");
    Ok(Some(cfg))
}

pub fn load_global_config_with_workspace(
    workspace: Option<&Path>,
) -> Result<GlobalConfig, ConfigError> {
    let config_dir = resolve_config_dir();
    let config_file = config_dir.join(CONFIG_FILENAME);

    let mut cfg = match read_config_file(&config_file)? {
        Some(loaded) => loaded,
        None => match resolve_home_dir() {
            Some(home) => GlobalConfig::default_for_home(&home),
            None => GlobalConfig::default(),
        },
    };

    let cascading_signatures = load_cascading_signatures(&config_dir, workspace);
    cfg.signatures.merge(cascading_signatures);

    let cascading_providers = load_cascading_providers(&config_dir, workspace);
    for (k, v) in cascading_providers {
        cfg.providers.entry(k).or_insert(v);
    }

    Ok(cfg)
}

pub fn load_global_config() -> Result<GlobalConfig, ConfigError> {
    load_global_config_with_workspace(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config_file_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_config_file(&dir.path().join("config.toml"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_read_config_file_invalid_is_error_with_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[network\nbroken").unwrap();

        match read_config_file(&path) {
            Err(ConfigError::ParseError(msg)) => {
                assert!(msg.contains(&path.display().to_string()))
            }
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn test_read_config_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, GlobalConfig::default().to_toml_str().unwrap()).unwrap();
        assert!(read_config_file(&path).unwrap().is_some());
    }
}
