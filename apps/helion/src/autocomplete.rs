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

//! Smart, reusable autocomplete and autosuggestion engine for Helion input fields.
//!
//! Provides inline ghost-text suggestions and command/slug completions across:
//! - Overview Quick-Action Command Palette
//! - Mod Load Order search filter
//! - Downloads URI / Magnet / Package Slug input
//! - Multi-Game Profile creation name input

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteSuggestion {
    /// Full replacement string when the suggestion is accepted (via Tab or Right Arrow).
    pub completed_text: String,
    /// The ghost text suffix displayed inline after the cursor in dimmed styling.
    pub ghost_suffix: String,
    /// Optional short semantic explanation of what this completion is.
    pub description: Option<String>,
}

pub struct Autocomplete;

impl Autocomplete {
    /// Reusable helper to match a query against a slice of candidate strings.
    /// If candidate starts with query (case-insensitive) and has remaining characters,
    /// returns the suggestion with exact casing from the candidate.
    pub fn match_candidate(query: &str, candidates: &[&str]) -> Option<AutocompleteSuggestion> {
        let trimmed_query = query.trim_start();
        if trimmed_query.is_empty() {
            return None;
        }

        let query_lower = trimmed_query.to_ascii_lowercase();

        for &candidate in candidates {
            let cand_lower = candidate.to_ascii_lowercase();
            if cand_lower.starts_with(&query_lower) && cand_lower.len() > query_lower.len() {
                let suffix = &candidate[trimmed_query.len()..];
                return Some(AutocompleteSuggestion {
                    completed_text: candidate.to_string(),
                    ghost_suffix: suffix.to_string(),
                    description: None,
                });
            }
        }
        None
    }

    /// Smart contextual autocomplete for Overview command palette.
    pub fn suggest_overview(query: &str, app: &crate::app::App) -> Option<AutocompleteSuggestion> {
        let trimmed = query.trim_start();
        if trimmed.is_empty() {
            return None;
        }

        let lower = trimmed.to_ascii_lowercase();

        // 1. Root command verbs
        const ROOT_COMMANDS: &[&str] = &[
            "clone ",
            "download ",
            "find ",
            "search ",
            "switch ",
            "profile ",
            "launch",
            "run",
            "conflicts",
            "mods",
            "downloads",
            "profiles",
            "help",
        ];

        if !lower.contains(' ') {
            if let Some(s) = Self::match_candidate(trimmed, ROOT_COMMANDS) {
                return Some(s);
            }
        }

        // 2. Sub-command arguments
        if lower.starts_with("clone ") || lower.starts_with("download ") {
            let prefix = if lower.starts_with("clone ") {
                "clone "
            } else {
                "download "
            };
            let arg = &trimmed[prefix.len()..];

            let mut candidates: Vec<String> =
                vec!["https://".to_string(), "magnet:?xt=urn:btih:".to_string()];
            for p in &app.curated_packages {
                candidates.push(p.title.to_lowercase().replace(' ', "_"));
            }
            for m in &app.mods {
                candidates.push(m.id.clone());
            }
            if let Ok(entries) = std::fs::read_dir(".perihelion/cache/mods") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".peca") {
                        candidates.push(name.trim_end_matches(".peca").to_string());
                    }
                }
            }

            let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
            if let Some(mut s) = Self::match_candidate(arg, &candidate_refs) {
                s.completed_text = format!("{}{}", prefix, s.completed_text);
                return Some(s);
            }
        } else if lower.starts_with("switch ") || lower.starts_with("profile ") {
            let prefix = if lower.starts_with("switch ") {
                "switch "
            } else {
                "profile "
            };
            let arg = &trimmed[prefix.len()..];

            let mut candidates: Vec<String> = Vec::new();
            for p in &app.profiles {
                candidates.push(p.name.clone());
                candidates.push(p.game_short.clone());
            }
            for g in &app.game_catalog {
                candidates.push(g.short_name.clone());
            }

            let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
            if let Some(mut s) = Self::match_candidate(arg, &candidate_refs) {
                s.completed_text = format!("{}{}", prefix, s.completed_text);
                return Some(s);
            }
        } else if lower.starts_with("search ") || lower.starts_with("find ") {
            let prefix = if lower.starts_with("search ") {
                "search "
            } else {
                "find "
            };
            let arg = &trimmed[prefix.len()..];

            let mut candidates: Vec<String> = Vec::new();
            for m in &app.mods {
                candidates.push(m.name.clone());
                candidates.push(m.id.clone());
                candidates.push(m.category.clone());
            }

            let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
            if let Some(mut s) = Self::match_candidate(arg, &candidate_refs) {
                s.completed_text = format!("{}{}", prefix, s.completed_text);
                return Some(s);
            }
        } else if lower.starts_with("theme ") {
            let prefix = "theme ";
            let arg = &trimmed[prefix.len()..];

            let candidates: Vec<String> = crate::theme::Theme::all_available()
                .into_iter()
                .map(|t| t.id)
                .collect();

            let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
            if let Some(mut s) = Self::match_candidate(arg, &candidate_refs) {
                s.completed_text = format!("{}{}", prefix, s.completed_text);
                return Some(s);
            }
        }

        None
    }

    /// Smart autocomplete for Mod Search filter.
    pub fn suggest_mod_search(
        query: &str,
        app: &crate::app::App,
    ) -> Option<AutocompleteSuggestion> {
        let trimmed = query.trim_start();
        if trimmed.is_empty() {
            return None;
        }

        let mut candidates: Vec<String> = Vec::new();
        for m in &app.mods {
            candidates.push(m.name.clone());
            candidates.push(m.id.clone());
            candidates.push(m.category.clone());
            candidates.push(m.author.clone());
        }

        let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
        Self::match_candidate(trimmed, &candidate_refs)
    }

    /// Smart autocomplete for Downloads URI input.
    pub fn suggest_download_uri(
        query: &str,
        app: &crate::app::App,
    ) -> Option<AutocompleteSuggestion> {
        let trimmed = query.trim_start();
        if trimmed.is_empty() {
            return None;
        }

        let mut candidates: Vec<String> =
            vec!["https://".to_string(), "magnet:?xt=urn:btih:".to_string()];
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".peca") {
                    candidates.push(name);
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(".perihelion/cache/mods") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".peca") {
                    candidates.push(name);
                }
            }
        }
        for d in &app.downloads {
            candidates.push(format!("{}.peca", d.name.to_lowercase().replace(' ', "_")));
        }
        for p in &app.curated_packages {
            candidates.push(format!("{}.peca", p.title.to_lowercase().replace(' ', "_")));
        }
        for m in &app.mods {
            candidates.push(format!("{}.peca", m.id));
        }

        let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
        Self::match_candidate(trimmed, &candidate_refs)
    }

    /// Smart autocomplete for New Profile name input.
    pub fn suggest_profile_name(
        query: &str,
        app: &crate::app::App,
    ) -> Option<AutocompleteSuggestion> {
        let trimmed = query.trim_start();
        if trimmed.is_empty() {
            return None;
        }

        let mut candidates: Vec<String> = vec![
            "Default".to_string(),
            "Vanilla+".to_string(),
            "Modded".to_string(),
            "Testing".to_string(),
            "Hardcore".to_string(),
        ];
        if let Some(game) = app.game_catalog.get(app.new_profile_game_idx) {
            candidates.push(format!("{} Modded", game.short_name));
            candidates.push(format!("{} Vanilla+", game.short_name));
        }

        for p in &app.profiles {
            candidates.push(p.name.clone());
        }

        let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
        Self::match_candidate(trimmed, &candidate_refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_candidate_prefix() {
        let candidates = ["clone ", "download ", "switch ", "launch"];
        let res = Autocomplete::match_candidate("cl", &candidates);
        assert!(res.is_some());
        let s = res.unwrap();
        assert_eq!(s.completed_text, "clone ");
        assert_eq!(s.ghost_suffix, "one ");

        // Exact match with no suffix returns None
        assert!(Autocomplete::match_candidate("clone ", &candidates).is_none());

        // Case-insensitive match canonicalizes to candidate casing
        let res2 = Autocomplete::match_candidate("SW", &candidates);
        assert!(res2.is_some());
        assert_eq!(res2.unwrap().completed_text, "switch ");
    }
}
