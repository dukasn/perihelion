use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ManifestError;

pub const MANIFEST_FILENAME: &str = "peri.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageMetadata,
    pub target: TargetGame,
    #[serde(default)]
    pub assets: Option<Assets>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub load_order: Option<LoadOrderRules>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetGame {
    pub game_id: String,
    #[serde(default)]
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assets {
    #[serde(default)]
    pub magnet: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadOrderRules {
    #[serde(default)]
    pub loads_after: Vec<String>,
    #[serde(default)]
    pub incompatible_with: Vec<String>,
}

impl Manifest {
    pub fn from_toml_str(input: &str) -> Result<Self, ManifestError> {
        let manifest: Self =
            toml::from_str(input).map_err(|e| ManifestError::ParseError(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_toml_str(&self) -> Result<String, ManifestError> {
        toml::to_string_pretty(self).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.package.name.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "Package name cannot be empty".to_string(),
            ));
        }

        if self.target.game_id.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "Target game_id cannot be empty".to_string(),
            ));
        }

        Version::parse(&self.package.version).map_err(|e| ManifestError::InvalidVersion {
            version: self.package.version.clone(),
            reason: e.to_string(),
        })?;

        if let Some(ref engine_req) = self.target.engine_version {
            VersionReq::parse(engine_req).map_err(|e| ManifestError::InvalidVersion {
                version: engine_req.clone(),
                reason: e.to_string(),
            })?;
        }

        for (dep_name, constraint) in &self.dependencies {
            VersionReq::parse(constraint).map_err(|e| ManifestError::InvalidVersion {
                version: constraint.clone(),
                reason: format!("Dependency '{dep_name}' has invalid version constraint: {e}"),
            })?;
        }

        if let Some(assets) = &self.assets {
            if let Some(sha) = &assets.sha256 {
                if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(ManifestError::InvalidChecksum(sha.clone()));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let sample = r#"
[package]
name = "Cyber Engine Tweaks"
author = "yamashi"
version = "1.30.0"

[target]
game_id = "cyberpunk_2077"
engine_version = ">=2.12"

[assets]
magnet = "magnet:?xt=urn:btih:123456789abcdef&dn=cet_1.30.0.peca"
url = "https://example.com/cet.peca"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[dependencies]
"some-framework" = ">=2.0"

[overrides]
"bin/x64/plugins/cyber_engine_tweaks.asi" = "self"
"#;
        let manifest = Manifest::from_toml_str(sample).expect("valid manifest should parse");
        assert_eq!(manifest.package.name, "Cyber Engine Tweaks");
        assert_eq!(manifest.package.version, "1.30.0");
        assert_eq!(manifest.target.game_id, "cyberpunk_2077");
        assert_eq!(
            manifest.dependencies.get("some-framework").unwrap(),
            ">=2.0"
        );
        assert_eq!(
            manifest
                .overrides
                .get("bin/x64/plugins/cyber_engine_tweaks.asi")
                .unwrap(),
            "self"
        );
    }

    #[test]
    fn test_invalid_sha256() {
        let sample = r#"
[package]
name = "Bad Checksum Mod"
author = "tester"
version = "1.0.0"

[target]
game_id = "skyrim"

[assets]
sha256 = "invalid_hash"
"#;
        let err = Manifest::from_toml_str(sample).unwrap_err();
        match err {
            ManifestError::InvalidChecksum(_) => (),
            _ => panic!("Expected InvalidChecksum error, got: {err:?}"),
        }
    }
}
