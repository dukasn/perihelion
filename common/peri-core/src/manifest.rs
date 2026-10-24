use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_spanned::Spanned;
use std::collections::BTreeMap;

use crate::error::ManifestError;

pub const MANIFEST_FILENAME: &str = "peri.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Lowest = 1,
    Low = 2,
    #[default]
    Default = 3,
    High = 4,
    Highest = 5,
    Absolute = 6,
}

impl Priority {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "lowest" => Some(Priority::Lowest),
            "low" => Some(Priority::Low),
            "default" => Some(Priority::Default),
            "high" => Some(Priority::High),
            "highest" => Some(Priority::Highest),
            "absolute" => Some(Priority::Absolute),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OverrideRule {
    Priority { priority: Priority },
    Target(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageMetadata,
    pub target: TargetGame,
    #[serde(default)]
    pub assets: Option<Assets>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, OverrideRule>,
    #[serde(default)]
    pub load_order: Option<LoadOrderRules>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    #[serde(default)]
    pub priority: Priority,
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

// -----------------------------------------------------------------------------
// Intermediate Spanned AST for compiler-style diagnostics
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct RawManifest {
    package: Option<Spanned<RawPackageMetadata>>,
    target: Option<Spanned<RawTargetGame>>,
    assets: Option<Spanned<RawAssets>>,
    #[serde(default)]
    dependencies: BTreeMap<String, Spanned<String>>,
    #[serde(default)]
    overrides: BTreeMap<String, Spanned<RawOverrideRule>>,
    #[serde(default)]
    load_order: Option<RawLoadOrderRules>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPackageMetadata {
    name: Option<Spanned<String>>,
    author: Option<Spanned<String>>,
    version: Option<Spanned<String>>,
    priority: Option<Spanned<String>>,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTargetGame {
    game_id: Option<Spanned<String>>,
    engine_version: Option<Spanned<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawAssets {
    magnet: Option<String>,
    url: Option<String>,
    sha256: Option<Spanned<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawOverrideRule {
    Priority { priority: String },
    Target(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawLoadOrderRules {
    #[serde(default)]
    loads_after: Vec<String>,
    #[serde(default)]
    incompatible_with: Vec<String>,
}

impl Manifest {
    /// Parses a raw TOML string and runs domain validations, capturing exact byte spans
    /// for compiler-grade diagnostic outputs with `miette`.
    pub fn parse_and_validate(source: &str) -> Result<Self, ManifestError> {
        let raw: RawManifest = toml::from_str(source).map_err(|e| {
            let span = match e.span() {
                Some(r) => miette::SourceSpan::from(r.start..r.end),
                None => miette::SourceSpan::from(0..0),
            };
            ManifestError::ParseError {
                message: e.message().to_string(),
                src: source.to_string(),
                span,
            }
        })?;

        // 1. Validate [package]
        let raw_pkg_spanned = raw.package.ok_or_else(|| ManifestError::MissingField {
            field: "package".to_string(),
            src: source.to_string(),
            span: (0..0).into(),
        })?;
        let raw_pkg = raw_pkg_spanned.into_inner();

        let name_spanned = raw_pkg.name.ok_or_else(|| ManifestError::MissingField {
            field: "package.name".to_string(),
            src: source.to_string(),
            span: (0..0).into(),
        })?;
        let name_span = name_spanned.span();
        let name = name_spanned.into_inner();
        if name.trim().is_empty() {
            return Err(ManifestError::MissingField {
                field: "package.name".to_string(),
                src: source.to_string(),
                span: miette::SourceSpan::from(name_span.start..name_span.end),
            });
        }

        let ver_spanned = raw_pkg.version.ok_or_else(|| ManifestError::MissingField {
            field: "package.version".to_string(),
            src: source.to_string(),
            span: (0..0).into(),
        })?;
        let ver_span = ver_spanned.span();
        let version_str = ver_spanned.into_inner();
        Version::parse(&version_str).map_err(|e| ManifestError::InvalidVersion {
            version: version_str.clone(),
            reason: e.to_string(),
            src: source.to_string(),
            span: miette::SourceSpan::from(ver_span.start..ver_span.end),
        })?;

        let priority = if let Some(prio_spanned) = raw_pkg.priority {
            let prio_span = prio_spanned.span();
            let prio_str = prio_spanned.into_inner();
            Priority::from_name(&prio_str).ok_or_else(|| ManifestError::InvalidPriority {
                bad_value: prio_str.clone(),
                src: source.to_string(),
                span: miette::SourceSpan::from(prio_span.start..prio_span.end),
            })?
        } else {
            Priority::Default
        };

        let author = raw_pkg
            .author
            .map(|s| s.into_inner())
            .unwrap_or_else(|| "unknown".to_string());

        let package = PackageMetadata {
            name,
            author,
            version: version_str,
            priority,
            description: raw_pkg.description,
        };

        // 2. Validate [target]
        let raw_target_spanned = raw.target.ok_or_else(|| ManifestError::MissingField {
            field: "target".to_string(),
            src: source.to_string(),
            span: (0..0).into(),
        })?;
        let raw_target = raw_target_spanned.into_inner();

        let game_id_spanned = raw_target
            .game_id
            .ok_or_else(|| ManifestError::MissingField {
                field: "target.game_id".to_string(),
                src: source.to_string(),
                span: (0..0).into(),
            })?;
        let game_id_span = game_id_spanned.span();
        let game_id = game_id_spanned.into_inner();
        if game_id.trim().is_empty() {
            return Err(ManifestError::MissingField {
                field: "target.game_id".to_string(),
                src: source.to_string(),
                span: miette::SourceSpan::from(game_id_span.start..game_id_span.end),
            });
        }

        let engine_version = if let Some(engine_spanned) = raw_target.engine_version {
            let eng_span = engine_spanned.span();
            let eng_str = engine_spanned.into_inner();
            VersionReq::parse(&eng_str).map_err(|e| ManifestError::InvalidVersion {
                version: eng_str.clone(),
                reason: format!("target engine_version constraint invalid: {e}"),
                src: source.to_string(),
                span: miette::SourceSpan::from(eng_span.start..eng_span.end),
            })?;
            Some(eng_str)
        } else {
            None
        };

        let target = TargetGame {
            game_id,
            engine_version,
        };

        // 3. Validate [dependencies]
        let mut dependencies = BTreeMap::new();
        for (dep_name, dep_req_spanned) in raw.dependencies {
            let req_span = dep_req_spanned.span();
            let req_str = dep_req_spanned.into_inner();
            VersionReq::parse(&req_str).map_err(|e| {
                ManifestError::InvalidDependencyConstraint {
                    dependency: dep_name.clone(),
                    constraint: req_str.clone(),
                    reason: e.to_string(),
                    src: source.to_string(),
                    span: miette::SourceSpan::from(req_span.start..req_span.end),
                }
            })?;
            dependencies.insert(dep_name, req_str);
        }

        // 4. Validate [assets]
        let assets = if let Some(raw_assets_spanned) = raw.assets {
            let raw_assets = raw_assets_spanned.into_inner();
            let sha256 = if let Some(sha_spanned) = raw_assets.sha256 {
                let sha_span = sha_spanned.span();
                let sha_str = sha_spanned.into_inner();
                if sha_str.len() != 64 || !sha_str.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(ManifestError::InvalidChecksum {
                        checksum: sha_str,
                        src: source.to_string(),
                        span: miette::SourceSpan::from(sha_span.start..sha_span.end),
                    });
                }
                Some(sha_str)
            } else {
                None
            };
            Some(Assets {
                magnet: raw_assets.magnet,
                url: raw_assets.url,
                sha256,
            })
        } else {
            None
        };

        // 5. Validate [overrides]
        let mut overrides = BTreeMap::new();
        for (ov_key, ov_rule_spanned) in raw.overrides {
            let rule_span = ov_rule_spanned.span();
            let rule = match ov_rule_spanned.into_inner() {
                RawOverrideRule::Priority { priority: prio_str } => {
                    let p = Priority::from_name(&prio_str).ok_or_else(|| {
                        ManifestError::InvalidPriority {
                            bad_value: prio_str,
                            src: source.to_string(),
                            span: miette::SourceSpan::from(rule_span.start..rule_span.end),
                        }
                    })?;
                    OverrideRule::Priority { priority: p }
                }
                RawOverrideRule::Target(t) => OverrideRule::Target(t),
            };
            overrides.insert(ov_key, rule);
        }

        // 6. [load_order]
        let load_order = raw.load_order.map(|lo| LoadOrderRules {
            loads_after: lo.loads_after,
            incompatible_with: lo.incompatible_with,
        });

        Ok(Manifest {
            package,
            target,
            assets,
            dependencies,
            overrides,
            load_order,
        })
    }

    pub fn from_toml_str(input: &str) -> Result<Self, ManifestError> {
        Self::parse_and_validate(input)
    }

    pub fn to_toml_str(&self) -> Result<String, ManifestError> {
        toml::to_string_pretty(self).map_err(|e| ManifestError::ValidationError(e.to_string()))
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
            &OverrideRule::Target("self".to_string())
        );
    }

    #[test]
    fn test_parse_v1_2_manifest_with_priority() {
        let sample = r#"
[package]
name = "Cyber Engine Tweaks"
author = "yamashi"
version = "1.30.0"
priority = "high"

[target]
game_id = "cyberpunk_2077"

[overrides]
"github.com/author/conflicting-mod" = { priority = "highest" }
"#;
        let manifest = Manifest::from_toml_str(sample).expect("v1.2 manifest should parse");
        assert_eq!(manifest.package.priority, Priority::High);
        assert_eq!(
            manifest
                .overrides
                .get("github.com/author/conflicting-mod")
                .unwrap(),
            &OverrideRule::Priority {
                priority: Priority::Highest
            }
        );
    }

    #[test]
    fn test_invalid_priority_span_detection() {
        let sample = r#"
[package]
name = "Broken Priority Mod"
version = "1.0.0"
priority = "ultra"

[target]
game_id = "skyrim"
"#;
        let err = Manifest::from_toml_str(sample).unwrap_err();
        match err {
            ManifestError::InvalidPriority {
                bad_value,
                span,
                src,
            } => {
                assert_eq!(bad_value, "ultra");
                let highlighted = &src[span.offset()..(span.offset() + span.len())];
                assert_eq!(highlighted, "\"ultra\"");
            }
            _ => panic!("Expected InvalidPriority error with span, got: {err:?}"),
        }
    }

    #[test]
    fn test_invalid_dependency_constraint_span() {
        let sample = r#"
[package]
name = "Bad Dep Mod"
version = "1.0.0"

[target]
game_id = "skyrim"

[dependencies]
"broken-lib" = ">>1.0.0"
"#;
        let err = Manifest::from_toml_str(sample).unwrap_err();
        match err {
            ManifestError::InvalidDependencyConstraint {
                dependency,
                constraint,
                span,
                src,
                ..
            } => {
                assert_eq!(dependency, "broken-lib");
                assert_eq!(constraint, ">>1.0.0");
                let highlighted = &src[span.offset()..(span.offset() + span.len())];
                assert_eq!(highlighted, "\">>1.0.0\"");
            }
            _ => panic!("Expected InvalidDependencyConstraint with span, got: {err:?}"),
        }
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
            ManifestError::InvalidChecksum {
                checksum,
                span,
                src,
            } => {
                assert_eq!(checksum, "invalid_hash");
                let highlighted = &src[span.offset()..(span.offset() + span.len())];
                assert_eq!(highlighted, "\"invalid_hash\"");
            }
            _ => panic!("Expected InvalidChecksum error with span, got: {err:?}"),
        }
    }
}
