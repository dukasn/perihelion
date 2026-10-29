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

//! Format-preserving edits of a `peri.toml` source string.
//!
//! Both functions are pure: they take the current text and return the new text,
//! keeping comments, ordering, and the user's formatting intact.

use semver::VersionReq;
use toml_edit::{value, DocumentMut, Item, Table};

use crate::error::ManifestError;

fn invalid(reason: impl Into<String>) -> ManifestError {
    ManifestError::ValidationError(reason.into())
}

fn parse(source: &str) -> Result<DocumentMut, ManifestError> {
    source
        .parse::<DocumentMut>()
        .map_err(|e| invalid(format!("cannot edit peri.toml: {e}")))
}

/// Sets `name = "req"` under `[dependencies]`, replacing any existing requirement.
pub fn add_dependency(source: &str, name: &str, req: &str) -> Result<String, ManifestError> {
    VersionReq::parse(req)
        .map_err(|e| invalid(format!("invalid version requirement '{req}': {e}")))?;

    let mut doc = parse(source)?;
    let deps = doc
        .entry("dependencies")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| invalid("'dependencies' in peri.toml is not a table"))?;
    deps.insert(name, value(req));
    Ok(doc.to_string())
}

/// Removes `name` from `[dependencies]`; returns `None` when it was not listed.
pub fn remove_dependency(source: &str, name: &str) -> Result<Option<String>, ManifestError> {
    let mut doc = parse(source)?;
    let removed = doc
        .get_mut("dependencies")
        .and_then(Item::as_table_mut)
        .and_then(|deps| deps.remove(name))
        .is_some();
    Ok(removed.then(|| doc.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# my pack\n[package]\nname = \"Pack\"\n\n[dependencies]\n# loader first\nskse64 = \"^2.2.6\"\n\n[overrides]\n";

    #[test]
    fn test_add_dependency_keeps_comments_and_other_tables() {
        let out = add_dependency(SAMPLE, "skyui", "^5.2.0").unwrap();
        assert!(out.contains("# my pack"));
        assert!(out.contains("# loader first"));
        assert!(out.contains("skyui = \"^5.2.0\""));
        assert!(out.contains("[overrides]"));
    }

    #[test]
    fn test_add_dependency_replaces_existing_requirement() {
        let out = add_dependency(SAMPLE, "skse64", "^2.3.0").unwrap();
        assert!(out.contains("skse64 = \"^2.3.0\""));
        assert!(!out.contains("^2.2.6"));
    }

    #[test]
    fn test_add_dependency_creates_missing_table() {
        let out = add_dependency("[package]\nname = \"P\"\n", "skse64", "^2").unwrap();
        assert!(out.contains("[dependencies]"));
        assert!(out.contains("skse64 = \"^2\""));
    }

    #[test]
    fn test_add_dependency_rejects_invalid_requirement() {
        assert!(add_dependency(SAMPLE, "skyui", "not-a-version").is_err());
    }

    #[test]
    fn test_remove_dependency() {
        let out = remove_dependency(SAMPLE, "skse64").unwrap().unwrap();
        assert!(!out.contains("skse64"));
        assert!(out.contains("# my pack"));
        assert!(remove_dependency(SAMPLE, "missing").unwrap().is_none());
    }
}
