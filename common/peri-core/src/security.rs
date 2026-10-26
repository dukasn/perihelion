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

use std::path::{Component, Path, PathBuf};

use crate::error::SecurityError;

/// Normalizes a path lexically without touching the filesystem and enforces that
/// the path is strictly contained within `allowed_root`.
///
/// Disallows `..` (ParentDir) elements and absolute paths that escape the boundary.
pub fn validate_safe_path(
    target_path: &Path,
    allowed_root: &Path,
) -> Result<PathBuf, SecurityError> {
    // 1. Strictly forbid any parent traversal ('..')
    for comp in target_path.components() {
        if let Component::ParentDir = comp {
            return Err(SecurityError::PathTraversal(target_path.to_path_buf()));
        }
    }

    // 2. Resolve target against allowed_root if relative
    let normalized = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        allowed_root.join(target_path)
    };

    // 3. Enforce component-wise boundary containment
    if !normalized.starts_with(allowed_root) {
        return Err(SecurityError::SandboxViolation {
            path: normalized,
            allowed_root: allowed_root.to_path_buf(),
        });
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_contained_absolute_path() {
        let root = Path::new("/var/home/dukasn");
        let target = Path::new("/var/home/dukasn/.local/share/perihelion/cache");
        let validated = validate_safe_path(target, root).expect("should be valid");
        assert_eq!(validated, target);
    }

    #[test]
    fn test_valid_contained_relative_path() {
        let root = Path::new("/var/home/dukasn");
        let target = Path::new(".perihelion/cache/mods");
        let validated = validate_safe_path(target, root).expect("should resolve within root");
        assert_eq!(
            validated,
            PathBuf::from("/var/home/dukasn/.perihelion/cache/mods")
        );
    }

    #[test]
    fn test_reject_path_traversal() {
        let root = Path::new("/var/home/dukasn");
        let target = Path::new("/var/home/dukasn/../../etc/shadow");
        let err = validate_safe_path(target, root).unwrap_err();
        match err {
            SecurityError::PathTraversal(p) => assert_eq!(p, target),
            other => panic!("expected PathTraversal error, got {:?}", other),
        }
    }

    #[test]
    fn test_reject_system_escape() {
        let root = Path::new("/var/home/dukasn");
        let target = Path::new("/usr/bin/malicious");
        let err = validate_safe_path(target, root).unwrap_err();
        match err {
            SecurityError::SandboxViolation { path, allowed_root } => {
                assert_eq!(path, target);
                assert_eq!(allowed_root, root);
            }
            other => panic!("expected SandboxViolation error, got {:?}", other),
        }
    }

    #[test]
    fn test_reject_prefix_collision() {
        let root = Path::new("/var/home/dukasn");
        let target = Path::new("/var/home/dukasn_attacker/stolen");
        let err = validate_safe_path(target, root).unwrap_err();
        assert!(matches!(err, SecurityError::SandboxViolation { .. }));
    }
}
