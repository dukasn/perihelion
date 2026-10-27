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
use std::path::Path;

use peri_core::{EngineSignature, SignatureRegistry};

/// Case-insensitive ASCII wildcard matching supporting `*` and `?`.
pub fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let c_bytes = candidate.as_bytes();
    let mut p_idx = 0;
    let mut c_idx = 0;
    let mut star_idx = None;
    let mut match_idx = 0;

    while c_idx < c_bytes.len() {
        if p_idx < p_bytes.len()
            && (p_bytes[p_idx] == b'?' || p_bytes[p_idx].eq_ignore_ascii_case(&c_bytes[c_idx]))
        {
            p_idx += 1;
            c_idx += 1;
        } else if p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
            star_idx = Some(p_idx);
            match_idx = c_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            c_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
        p_idx += 1;
    }

    p_idx == p_bytes.len()
}

/// Probes a game root directory against an anchor or marker query pattern.
/// Supports relative paths, case-insensitivity, and wildcards (`*`).
pub fn probe_game_directory(game_dir: &Path, pattern: &str) -> bool {
    let normalized = pattern.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return false;
    }
    match_segments(game_dir, &segments)
}

fn match_segments(current_dir: &Path, remaining_segments: &[&str]) -> bool {
    if remaining_segments.is_empty() {
        return true;
    }

    let segment = remaining_segments[0];
    let next_remaining = &remaining_segments[1..];

    if !segment.contains('*') {
        let target = current_dir.join(segment);
        if target.exists() {
            if next_remaining.is_empty() {
                return true;
            }
            return match_segments(&target, next_remaining);
        }

        // Case-insensitive fallback for filesystem probe
        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().eq_ignore_ascii_case(segment) {
                    if next_remaining.is_empty() {
                        return true;
                    }
                    return match_segments(&entry.path(), next_remaining);
                }
            }
        }

        return false;
    }

    // Segment contains wildcards
    let Ok(entries) = fs::read_dir(current_dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if wildcard_match(segment, &name_str) {
            if next_remaining.is_empty() {
                return true;
            }
            if entry.path().is_dir() && match_segments(&entry.path(), next_remaining) {
                return true;
            }
        }
    }

    false
}

/// Detects the game engine layout from an installation directory by evaluating
/// structural anchors and markers against a `SignatureRegistry`.
pub fn detect_engine_layout(
    game_dir: &Path,
    registry: &SignatureRegistry,
) -> Option<EngineSignature> {
    if !game_dir.exists() {
        return None;
    }
    registry
        .detect(|query| probe_game_directory(game_dir, query))
        .cloned()
}

/// Locates candidate executable binaries tailored to a detected `EngineSignature`.
pub fn find_executable_for_engine(game_dir: &Path, signature: &EngineSignature) -> Option<String> {
    let is_ext_match = |path: &Path| -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            signature
                .binary_extensions
                .iter()
                .any(|e| ext.eq_ignore_ascii_case(e))
        } else {
            false
        }
    };

    for sub in &signature.binary_subdirs {
        let sub_path = if sub == "." || sub.is_empty() {
            game_dir.to_path_buf()
        } else {
            game_dir.join(sub)
        };

        if sub_path.exists() {
            if let Ok(entries) = fs::read_dir(&sub_path) {
                let mut candidates = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && is_ext_match(&path) {
                        if let Ok(rel) = path.strip_prefix(game_dir) {
                            candidates.push(rel.to_string_lossy().into_owned());
                        }
                    }
                }
                // Sort main game executable candidates above crash reporters/helpers
                candidates.sort_by_key(|c| {
                    let lower = c.to_lowercase();
                    if lower.contains("crash")
                        || lower.contains("report")
                        || lower.contains("helper")
                    {
                        1
                    } else {
                        0
                    }
                });
                if let Some(first) = candidates.into_iter().next() {
                    return Some(first);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wildcard_matching() {
        assert!(wildcard_match("*.esm", "Skyrim.esm"));
        assert!(wildcard_match("*.esm", "FALLOUT4.ESM"));
        assert!(wildcard_match("*_loader.exe", "f4se_loader.exe"));
        assert!(wildcard_match("*_Data", "Valheim_Data"));
        assert!(!wildcard_match("*.esm", "Skyrim.esp"));
        assert!(!wildcard_match("Data", "Other"));
    }

    #[test]
    fn test_probe_game_directory_wildcards() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("Data")).unwrap();
        fs::write(root.join("Data/Starfield.esm"), b"").unwrap();

        assert!(probe_game_directory(root, "Data"));
        assert!(probe_game_directory(root, "Data/*.esm"));
        assert!(!probe_game_directory(root, "Data/*.esp"));
        assert!(!probe_game_directory(root, "bin/x64"));
    }

    #[test]
    fn test_detect_engine_layout() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("bin/x64")).unwrap();
        fs::create_dir_all(root.join("r6/config")).unwrap();
        fs::write(root.join("bin/x64/Cyberpunk2077.exe"), b"").unwrap();

        let mut signatures = peri_core::SignatureRegistry::new();
        signatures.register(peri_core::EngineSignature {
            id: "redengine_4".to_string(),
            name: "REDengine 4".to_string(),
            priority: 150,
            anchors: vec!["bin/x64".to_string()],
            markers: vec!["r6/config".to_string()],
            binary_subdirs: vec!["bin/x64".to_string()],
            binary_extensions: vec!["exe".to_string()],
            environment: vec![],
            mappings: vec![],
            content_extensions: vec![],
        });
        let detected = detect_engine_layout(root, &signatures).expect("should detect redengine");

        assert_eq!(detected.id, "redengine_4");

        let exe = find_executable_for_engine(root, &detected);
        assert_eq!(exe.as_deref(), Some("bin/x64/Cyberpunk2077.exe"));
    }
}
