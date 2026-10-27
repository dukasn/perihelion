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
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::profile::LayoutMapping;
use crate::signature::{EngineSignature, SignatureRegistry};

pub const CONFIG_FILENAME: &str = "config.toml";

/// Default embedded TOML configuration for platform discovery and layout mapping LUT.
/// Compiled directly into the binary from `data/discovery.toml`.
pub const BUILTIN_DISCOVERY_TOML: &str = include_str!("../data/discovery.toml");

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

fn default_theme() -> String {
    "terminal".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

/// Generic platform or discovery provider configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    /// Candidate root installation directories (supports ~ home expansion).
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    /// Additional library / storage folders.
    #[serde(default)]
    pub libraries: Vec<PathBuf>,
    /// Directories scanned for compatibility tools (Proton, Wine, runners).
    #[serde(default)]
    pub compatibility_dirs: Vec<PathBuf>,
    /// Relative subdirectories inside install folders scanned for executables.
    #[serde(default)]
    pub binary_subdirs: Vec<String>,
    /// File extensions considered executable candidates.
    #[serde(default)]
    pub binary_extensions: Vec<String>,
    /// Environment variables passed through to runner execution.
    #[serde(default)]
    pub environment: Vec<String>,
}

/// Lookup table (LUT) registry for VFS layout prefix mapping rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MappingRegistry {
    /// Default fallback layout prefix mappings.
    #[serde(default)]
    pub default: Vec<LayoutMapping>,
    /// Registered mapping types (templates / layout styles, e.g. "redengine", "creation", "unreal").
    #[serde(default)]
    pub types: BTreeMap<String, Vec<LayoutMapping>>,
    /// Registered target mapping rules keyed by App ID, slug, or target name.
    #[serde(default)]
    pub targets: BTreeMap<String, Vec<LayoutMapping>>,
}

impl MappingRegistry {
    /// Registers a reusable mapping type in the LUT.
    pub fn register_type(&mut self, name: impl Into<String>, rules: Vec<LayoutMapping>) {
        self.types.insert(name.into(), rules);
    }

    /// Registers target mapping rules in the LUT.
    pub fn register_target(&mut self, target: impl Into<String>, rules: Vec<LayoutMapping>) {
        self.targets.insert(target.into(), rules);
    }

    /// Resolves mappings from the LUT for an optional App ID and target/game slug,
    /// falling back to matching registered types or default mappings.
    pub fn resolve(&self, app_id: Option<u64>, game_id: Option<&str>) -> Vec<LayoutMapping> {
        if let Some(id) = app_id {
            let key = id.to_string();
            if let Some(mappings) = self.targets.get(&key) {
                return mappings.clone();
            }
        }
        if let Some(slug) = game_id {
            if let Some(mappings) = self.targets.get(slug) {
                return mappings.clone();
            }
            if let Some(mappings) = self.types.get(slug) {
                return mappings.clone();
            }
        }
        self.default.clone()
    }
}

/// Type alias maintaining backward-compatibility for Steam discovery lookups.
pub type SteamConfig = ProviderConfig;

#[derive(Debug, Clone, Deserialize)]
struct BuiltinDiscoveryToml {
    #[serde(default)]
    providers: BTreeMap<String, ProviderConfig>,
    #[serde(default)]
    signatures: Vec<EngineSignature>,
    #[serde(default)]
    mappings: MappingRegistry,
}

fn parse_builtin_discovery() -> BuiltinDiscoveryToml {
    toml::from_str(BUILTIN_DISCOVERY_TOML).unwrap_or(BuiltinDiscoveryToml {
        providers: BTreeMap::new(),
        signatures: Vec::new(),
        mappings: MappingRegistry::default(),
    })
}

fn default_providers() -> BTreeMap<String, ProviderConfig> {
    parse_builtin_discovery().providers
}

fn default_signatures() -> SignatureRegistry {
    let mut reg = SignatureRegistry::new();
    for sig in parse_builtin_discovery().signatures {
        reg.register(sig);
    }
    reg
}

fn default_mappings() -> MappingRegistry {
    parse_builtin_discovery().mappings
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default = "default_providers")]
    pub providers: BTreeMap<String, ProviderConfig>,
    #[serde(default = "default_signatures")]
    pub signatures: SignatureRegistry,
    #[serde(default = "default_mappings")]
    pub mappings: MappingRegistry,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            ui: UiConfig::default(),
            providers: default_providers(),
            signatures: default_signatures(),
            mappings: default_mappings(),
        }
    }
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
            ui: UiConfig::default(),
            providers: default_providers(),
            signatures: default_signatures(),
            mappings: default_mappings(),
        }
    }

    /// Retrieves provider configuration by key from the providers LUT (e.g. "steam", "lutris").
    pub fn provider(&self, name: &str) -> ProviderConfig {
        self.providers.get(name).cloned().unwrap_or_default()
    }

    /// Accesses the registry of declarative engine signatures.
    pub fn signatures(&self) -> &SignatureRegistry {
        &self.signatures
    }

    /// Probes registered engine signatures to detect which game engine layout applies.
    pub fn detect_engine<F: FnMut(&str) -> bool>(&self, probe: F) -> Option<&EngineSignature> {
        self.signatures.detect(probe)
    }

    /// Resolves path mappings using the mappings LUT and registered engine signatures.
    pub fn resolve_mappings(
        &self,
        app_id: Option<u64>,
        game_id: Option<&str>,
    ) -> Vec<LayoutMapping> {
        let from_lut = self.mappings.resolve(app_id, game_id);
        if !from_lut.is_empty() {
            return from_lut;
        }

        if let Some(slug) = game_id {
            if let Some(sig) = self.signatures.get(slug) {
                if !sig.mappings.is_empty() {
                    return sig.mappings.clone();
                }
            }
        }

        self.mappings.default.clone()
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
        assert_eq!(cfg.ui.theme, "terminal");
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
        assert!(output.contains("[providers.steam]"));
    }

    #[test]
    fn test_provider_and_mapping_lut_deserialization() {
        let toml_data = r#"
[providers.custom_runner]
roots = ["/custom/runner/root"]
libraries = ["/mnt/games/Library"]
binary_extensions = ["exe", "bin"]

[mappings]
default = [{ from = "Data", to = "Data" }]

[mappings.types]
redengine = [
    { from = "mods", to = "archive/pc/mod" },
    { from = "r6", to = "r6" }
]

[mappings.targets]
"1091500" = [
    { from = "mods", to = "archive/pc/mod" }
]
"#;
        let cfg = GlobalConfig::from_toml_str(toml_data)
            .expect("should parse valid toml with provider and mappings LUT");

        let provider = cfg.provider("custom_runner");
        assert_eq!(provider.roots, vec![PathBuf::from("/custom/runner/root")]);
        assert_eq!(
            provider.libraries,
            vec![PathBuf::from("/mnt/games/Library")]
        );
        assert_eq!(
            provider.binary_extensions,
            vec!["exe".to_string(), "bin".to_string()]
        );

        // Resolves from LUT targets by App ID
        let app_mapped = cfg.resolve_mappings(Some(1091500), None);
        assert_eq!(app_mapped.len(), 1);
        assert_eq!(app_mapped[0].from_prefix, "mods");
        assert_eq!(app_mapped[0].to_prefix, "archive/pc/mod");

        // Resolves from LUT types by type slug
        let type_mapped = cfg.resolve_mappings(None, Some("redengine"));
        assert_eq!(type_mapped.len(), 2);
        assert_eq!(type_mapped[0].from_prefix, "mods");

        // Falls back to default LUT mappings
        let fallback_mapped = cfg.resolve_mappings(Some(489830), Some("other"));
        assert_eq!(fallback_mapped.len(), 1);
        assert_eq!(fallback_mapped[0].from_prefix, "Data");
    }

    #[test]
    fn test_builtin_discovery_config_loads_from_asset_toml() {
        let cfg = GlobalConfig::default();
        let steam = cfg.provider("steam");
        assert!(!steam.roots.is_empty());
        assert!(steam.roots.contains(&PathBuf::from("~/.local/share/Steam")));
        assert!(steam
            .roots
            .contains(&PathBuf::from("~/snap/steam/common/.local/share/Steam")));
        assert!(steam.binary_extensions.contains(&"exe".to_string()));
        assert!(steam.environment.contains(&"MANGOHUD".to_string()));

        // Signatures loaded from embedded asset baseline
        let sigs = cfg.signatures();
        assert!(!sigs.is_empty());
        let bethesda = sigs
            .get("bethesda_creation")
            .expect("bethesda creation engine present");
        assert_eq!(bethesda.anchors, vec!["Data"]);
        assert!(bethesda.mappings.iter().any(|m| m.from_prefix == "SKSE"));

        let redengine = sigs.get("redengine_4").expect("redengine 4 present");
        assert_eq!(redengine.binary_subdirs, vec!["bin/x64"]);

        let unreal = sigs.get("unreal_engine").expect("unreal engine present");
        assert_eq!(
            unreal.binary_subdirs,
            vec!["Binaries/Win64", "Binaries/Linux"]
        );

        // Engine detection via abstract probe
        let detected = cfg.detect_engine(|path| path == "Data" || path == "Data/*.esm");
        assert_eq!(detected.map(|s| s.id.as_str()), Some("bethesda_creation"));

        // Signatures loaded from embedded asset baseline
        let sigs = cfg.signatures();
        assert!(!sigs.is_empty());
        let bethesda = sigs
            .get("bethesda_creation")
            .expect("bethesda creation engine present");
        assert_eq!(bethesda.anchors, vec!["Data"]);
        assert!(bethesda.mappings.iter().any(|m| m.from_prefix == "SKSE"));

        let redengine = sigs.get("redengine_4").expect("redengine 4 present");
        assert_eq!(redengine.binary_subdirs, vec!["bin/x64"]);

        let unreal = sigs.get("unreal_engine").expect("unreal engine present");
        assert_eq!(
            unreal.binary_subdirs,
            vec!["Binaries/Win64", "Binaries/Linux"]
        );

        // Engine detection via abstract probe
        let detected = cfg.detect_engine(|path| path == "Data" || path == "Data/*.esm");
        assert_eq!(detected.map(|s| s.id.as_str()), Some("bethesda_creation"));
    }
}
