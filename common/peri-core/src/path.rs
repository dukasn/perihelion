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

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::profile::LayoutMapping;

/// A normalized, POSIX-style relative path guaranteed to be free of
/// parent-traversal components (`..`) and redundant current-directory markers (`./`).
///
/// Borrows from the input slice whenever the source is already clean.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedPath<'a> {
    inner: Cow<'a, str>,
}

impl<'a> NormalizedPath<'a> {
    /// Creates a new `NormalizedPath` from a `std::path::Path`.
    /// Rejects parent-traversal components (`..`) by returning `None`.
    pub fn new(path: &'a Path) -> Option<Self> {
        let raw = path.to_str()?;
        Self::from_str(raw)
    }

    /// Creates a new `NormalizedPath` from a string slice.
    /// Rejects parent-traversal components (`..`) by returning `None`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(raw: &'a str) -> Option<Self> {
        // Fast path: clean UTF-8 string with no Windows backslashes, redundant relative tokens, or parent escapes
        if !raw.contains('\\') && !raw.contains("./") && !raw.contains("..") {
            let trimmed = raw.trim_start_matches('/');
            return Some(Self {
                inner: Cow::Borrowed(trimmed),
            });
        }

        // Slow path: normalize into a single allocated buffer without intermediate allocations
        // Handles cross-platform Windows backslashes and POSIX forward slashes identically
        let mut normalized = String::with_capacity(raw.len());
        for segment in raw.split(['/', '\\']) {
            match segment {
                "" | "." => continue,
                ".." => return None, // Security boundary: reject path traversal
                part => {
                    if !normalized.is_empty() {
                        normalized.push('/');
                    }
                    normalized.push_str(part);
                }
            }
        }

        Some(Self {
            inner: Cow::Owned(normalized),
        })
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    #[inline(always)]
    pub fn as_path(&self) -> &Path {
        Path::new(self.as_str())
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.as_str())
    }

    /// Case-insensitive prefix test avoiding intermediate String allocations.
    /// Returns the remainder of the path if the prefix matches as a directory boundary.
    #[inline(always)]
    pub fn strip_prefix<'p>(&'p self, prefix: &str) -> Option<&'p str> {
        let src = self.as_str();
        let p_len = prefix.len();

        if src.len() == p_len && src.eq_ignore_ascii_case(prefix) {
            return Some("");
        }

        if src.len() > p_len
            && src[..p_len].eq_ignore_ascii_case(prefix)
            && src.as_bytes()[p_len] == b'/'
        {
            return Some(&src[p_len + 1..]);
        }

        None
    }
}

/// A pre-compiled layout mapping rule with normalized prefixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMappingRule {
    pub from_prefix: String,
    pub to_prefix: String,
}

/// Pre-compiled radix prefix router for game layout transformations.
///
/// Rules are sorted descending by `from_prefix.len()`, guaranteeing
/// that the longest matching prefix evaluates first without ambiguities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FastPathRouter {
    rules: Vec<CompiledMappingRule>,
}

impl FastPathRouter {
    /// Compiles a slice of `LayoutMapping` into a deterministic prefix routing table.
    pub fn compile(mappings: &[LayoutMapping]) -> Self {
        let mut rules: Vec<CompiledMappingRule> = mappings
            .iter()
            .map(|m| CompiledMappingRule {
                from_prefix: m.from_prefix.trim_matches('/').to_string(),
                to_prefix: m.to_prefix.trim_matches('/').to_string(),
            })
            .collect();

        // Sort descending by prefix length: longest match wins unambiguously
        rules.sort_by_key(|a| std::cmp::Reverse(a.from_prefix.len()));

        Self { rules }
    }

    /// Checks whether the router has any registered rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns the number of compiled rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns a slice of the compiled mapping rules.
    pub fn rules(&self) -> &[CompiledMappingRule] {
        &self.rules
    }

    /// Transforms a normalized mod path to its target VFS destination path.
    pub fn route(&self, path: &NormalizedPath) -> PathBuf {
        for rule in &self.rules {
            if let Some(remainder) = path.strip_prefix(&rule.from_prefix) {
                if remainder.is_empty() {
                    return PathBuf::from(&rule.to_prefix);
                }

                let mut out = String::with_capacity(rule.to_prefix.len() + 1 + remainder.len());
                if !rule.to_prefix.is_empty() {
                    out.push_str(&rule.to_prefix);
                    out.push('/');
                }
                out.push_str(remainder);
                return PathBuf::from(out);
            }
        }

        // Default: 1:1 direct overlay without translation
        path.to_path_buf()
    }

    /// Convenience helper that normalizes a string slice and routes it.
    /// If normalization fails (due to directory traversal), returns None.
    pub fn route_str(&self, raw: &str) -> Option<PathBuf> {
        let norm = NormalizedPath::from_str(raw)?;
        Some(self.route(&norm))
    }
}

/// Computes a deterministic 64-bit case-folded FNV-1a hash for normalized paths.
///
/// Enables constant-time bitwise set operations across virtual file index entries.
#[inline(always)]
pub fn hash_normalized_path(path: &str) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in path.bytes() {
        hash ^= byte.to_ascii_lowercase() as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_path_clean() {
        let path = Path::new("Data/Textures/sky.dds");
        let norm = NormalizedPath::new(path).expect("should normalize");
        assert_eq!(norm.as_str(), "Data/Textures/sky.dds");
        assert!(matches!(norm.inner, Cow::Borrowed(_)));
    }

    #[test]
    fn test_normalized_path_windows_slashes() {
        let raw = r"Data\Meshes\Weapons\sword.nif";
        let norm = NormalizedPath::from_str(raw).expect("should normalize");
        assert_eq!(norm.as_str(), "Data/Meshes/Weapons/sword.nif");
        assert!(matches!(norm.inner, Cow::Owned(_)));
    }

    #[test]
    fn test_normalized_path_curdir_and_leading_slash() {
        let raw = "./mods/tweaks/settings.ini";
        let norm = NormalizedPath::from_str(raw).expect("should normalize");
        assert_eq!(norm.as_str(), "mods/tweaks/settings.ini");

        let leading = "/mods/tweaks/settings.ini";
        let norm_leading = NormalizedPath::from_str(leading).expect("should normalize");
        assert_eq!(norm_leading.as_str(), "mods/tweaks/settings.ini");
    }

    #[test]
    fn test_normalized_path_rejects_parent_traversal() {
        let escape = "../etc/shadow";
        assert!(NormalizedPath::from_str(escape).is_none());

        let nested_escape = "Data/../../root/file";
        assert!(NormalizedPath::from_str(nested_escape).is_none());
    }

    #[test]
    fn test_normalized_path_strip_prefix() {
        let norm = NormalizedPath::from_str("mods/archive/cyber.archive").unwrap();
        assert_eq!(norm.strip_prefix("mods"), Some("archive/cyber.archive"));
        assert_eq!(norm.strip_prefix("MODS"), Some("archive/cyber.archive"));
        assert_eq!(norm.strip_prefix("other"), None);

        let exact = NormalizedPath::from_str("mods").unwrap();
        assert_eq!(exact.strip_prefix("mods"), Some(""));
        assert_eq!(exact.strip_prefix("MODS"), Some(""));
    }

    #[test]
    fn test_fast_path_router_longest_prefix_priority() {
        let mappings = vec![
            LayoutMapping::new("Data", "BaseData"),
            LayoutMapping::new("Data/SKSE", "Plugins/SKSE"),
            LayoutMapping::new("mods", "archive/pc/mod"),
        ];

        let router = FastPathRouter::compile(&mappings);

        // Longest prefix Data/SKSE must win over Data
        let skse_path = NormalizedPath::from_str("data/skse/plugin.dll").unwrap();
        let routed = router.route(&skse_path);
        assert_eq!(routed, PathBuf::from("Plugins/SKSE/plugin.dll"));

        let base_path = NormalizedPath::from_str("Data/textures/wood.dds").unwrap();
        let routed_base = router.route(&base_path);
        assert_eq!(routed_base, PathBuf::from("BaseData/textures/wood.dds"));

        let mod_path = NormalizedPath::from_str("mods/cyber.archive").unwrap();
        let routed_mod = router.route(&mod_path);
        assert_eq!(routed_mod, PathBuf::from("archive/pc/mod/cyber.archive"));

        // Unmatched fallback
        let untouched = NormalizedPath::from_str("unrelated/file.txt").unwrap();
        assert_eq!(
            router.route(&untouched),
            PathBuf::from("unrelated/file.txt")
        );
    }

    #[test]
    fn test_hash_normalized_path_case_insensitivity() {
        let h1 = hash_normalized_path("Data/Textures/Grass.DDS");
        let h2 = hash_normalized_path("data/textures/grass.dds");
        let h3 = hash_normalized_path("DATA/TEXTURES/GRASS.DDS");
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);

        let different = hash_normalized_path("Data/Textures/Stone.dds");
        assert_ne!(h1, different);
    }
}
