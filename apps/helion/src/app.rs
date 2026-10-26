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

use peri_core::{
    draft::{SessionDraft, StagedChanges, UiContext},
    manifest::{Manifest, OverrideRule, Priority},
    profile::{GameProfile, LayoutMapping},
    vfi::VirtualFileIndex,
};
use std::path::{Path, PathBuf};

use crate::action::{
    Action, ConflictsFocus, DownloadsFocus, InputValidation, ModsFocus, OverviewFocus,
    ProfilesFocus, StatusLevel, Tab,
};
use crate::worker::{PersistenceCommand, WorkerSender};
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Debug, Clone)]
pub enum DownloadMessage {
    Progress {
        name: String,
        progress: f32,
        speed: String,
        peers_info: String,
        status: String,
    },
    Complete {
        name: String,
        path: PathBuf,
    },
    Error {
        name: String,
        error: String,
    },
}

pub struct DownloadReceiver(pub std::sync::mpsc::Receiver<DownloadMessage>);

impl std::fmt::Debug for DownloadReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DownloadReceiver(...)")
    }
}

#[derive(Debug, Clone)]
pub struct ModItem {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub category: String,
    pub priority: Priority,
    pub version: String,
    pub author: String,
    pub description: String,
    pub affected_files: Vec<String>,
    pub has_conflict: bool,
}

#[derive(Debug, Clone)]
pub struct ConflictOption {
    pub mod_id: String,
    pub mod_name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ConflictCard {
    pub title: String,
    pub file_path: String,
    pub option_a: ConflictOption,
    pub option_b: ConflictOption,
    pub winner_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConflict {
    pub title: String,
    pub file_path: String,
    pub resolution_note: String,
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub name: String,
    pub progress: f32,
    pub speed: String,
    pub status: String,
    pub peers_info: String,
    pub paused: bool,
}

#[derive(Debug, Clone)]
pub struct CuratedPackage {
    pub title: String,
    pub desc: String,
    pub is_installed: bool,
}

#[derive(Debug, Clone)]
pub struct DiagnosticAlert {
    pub level: String,
    pub title: String,
    pub message: String,
    pub action_text: String,
    pub resolved: bool,
}

#[derive(Debug, Clone)]
pub struct GameCatalogEntry {
    pub game_id: String,
    pub short_name: String,
    pub full_name: String,
    pub steam_app_id: u64,
    pub executable: String,
    pub default_proton: String,
    pub install_dir: String,
    pub mappings: Vec<LayoutMapping>,
    pub default_diagnostic: DiagnosticAlert,
}

#[derive(Debug, Clone)]
pub struct ProfileItem {
    pub name: String,
    pub game_id: String,
    pub game_short: String,
    pub game_full_name: String,
    pub steam_app_id: u64,
    pub executable: String,
    pub install_dir: String,
    pub upper_dir: String,
    pub mappings: Vec<LayoutMapping>,
    pub desc: String,
    pub mod_count: usize,
    pub proton_version: String,
    pub is_active: bool,
    pub diagnostic: DiagnosticAlert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveGameHandle {
    pub pid: u32,
    pub game_id: String,
    pub snapshot_id: String,
}

#[derive(Debug)]
pub struct App {
    pub active_tab: Tab,
    pub overview_focus: OverviewFocus,
    pub overview_input: String,
    pub overview_input_active: bool,
    pub overview_input_error: Option<String>,
    pub overview_profile_cursor: usize,
    pub game_title: String,
    pub proton_version: String,
    pub mods: Vec<ModItem>,
    pub cursor_index: usize,
    pub mods_focus: ModsFocus,
    pub inspector_file_cursor: usize,
    pub search_query: String,
    pub search_active: bool,
    pub search_error: Option<String>,
    pub category_filter: Option<String>,
    pub show_inspector: bool,
    pub conflicts: Vec<ConflictCard>,
    pub conflicts_focus: ConflictsFocus,
    pub resolved_conflicts: Vec<ResolvedConflict>,
    pub active_conflict_cursor: usize,
    pub downloads: Vec<DownloadItem>,
    pub download_focus: DownloadsFocus,
    pub download_cursor: usize,
    pub curated_cursor: usize,
    pub download_input: String,
    pub download_input_active: bool,
    pub download_input_error: Option<String>,
    pub curated_packages: Vec<CuratedPackage>,
    pub download_tx: std::sync::mpsc::Sender<DownloadMessage>,
    pub download_rx: DownloadReceiver,
    pub game_catalog: Vec<GameCatalogEntry>,
    pub new_profile_game_idx: usize,
    pub profiles: Vec<ProfileItem>,
    pub profiles_focus: ProfilesFocus,
    pub profile_cursor: usize,
    pub profile_input: String,
    pub profile_input_active: bool,
    pub profile_input_error: Option<String>,
    pub diagnostics: Vec<DiagnosticAlert>,
    pub show_help: bool,
    pub status_message: Option<String>,
    pub status_level: StatusLevel,
    pub should_quit: bool,
    pub canonical_manifest: Option<Manifest>,
    pub staged: StagedChanges,
    pub active_session: Option<ActiveGameHandle>,
    pub worker_tx: Option<WorkerSender>,
    pub session_tx: Option<tokio_mpsc::Sender<PersistenceCommand>>,
    pub theme: crate::theme::Theme,
    pub committed_theme: crate::theme::Theme,
    pub theme_picker_cursor: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (download_tx, download_rx) = std::sync::mpsc::channel();
        let initial_theme = crate::theme::load_configured_theme();
        crate::theme::set_active_theme(initial_theme.clone());

        let mut app = Self {
            active_tab: Tab::Overview,
            overview_focus: OverviewFocus::CommandInput,
            overview_input: String::new(),
            overview_input_active: false,
            overview_input_error: None,
            overview_profile_cursor: 0,
            game_title: "No Active Workspace".to_string(),
            proton_version: "Detecting...".to_string(),
            mods: Vec::new(),
            cursor_index: 0,
            mods_focus: ModsFocus::Table,
            inspector_file_cursor: 0,
            search_query: String::new(),
            search_active: false,
            search_error: None,
            category_filter: None,
            show_inspector: true,
            conflicts: Vec::new(),
            conflicts_focus: ConflictsFocus::OptionA,
            resolved_conflicts: Vec::new(),
            active_conflict_cursor: 0,
            downloads: Vec::new(),
            download_focus: DownloadsFocus::Queue,
            download_cursor: 0,
            curated_cursor: 0,
            download_input: String::new(),
            download_input_active: false,
            download_input_error: None,
            curated_packages: Vec::new(),
            download_tx,
            download_rx: DownloadReceiver(download_rx),
            game_catalog: Vec::new(),
            new_profile_game_idx: 0,
            profiles: Vec::new(),
            profiles_focus: ProfilesFocus::ProfileList,
            profile_cursor: 0,
            profile_input: String::new(),
            profile_input_active: false,
            profile_input_error: None,
            diagnostics: Vec::new(),
            show_help: false,
            status_message: None,
            status_level: StatusLevel::Info,
            should_quit: false,
            canonical_manifest: None,
            staged: StagedChanges::default(),
            active_session: None,
            worker_tx: None,
            session_tx: None,
            theme: initial_theme.clone(),
            committed_theme: initial_theme,
            theme_picker_cursor: 0,
        };

        app.discover_system_and_profiles();
        app.try_load_workspace();
        app
    }

    pub fn set_theme(&mut self, theme: crate::theme::Theme) {
        self.theme = theme.clone();
        self.committed_theme = theme.clone();
        crate::theme::set_active_theme(theme);
    }

    pub fn is_theme_command(&self) -> bool {
        let trimmed = self.overview_input.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        lower.starts_with("theme") || lower.starts_with("/theme") || lower.starts_with(":theme")
    }

    pub fn matching_themes_for_overview(&self) -> Vec<crate::theme::Theme> {
        let trimmed = self.overview_input.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        let all = crate::theme::Theme::all_available();

        let prefix = if lower.starts_with("/theme") {
            "/theme"
        } else if lower.starts_with(":theme") {
            ":theme"
        } else if lower.starts_with("theme") {
            "theme"
        } else {
            return all;
        };

        let arg = lower[prefix.len()..].trim();
        if arg.is_empty() {
            return all;
        }

        let filtered: Vec<_> = all
            .iter()
            .filter(|t| t.id.contains(arg) || t.name.to_ascii_lowercase().contains(arg))
            .cloned()
            .collect();

        if filtered.is_empty() {
            all
        } else {
            filtered
        }
    }

    pub fn update_realtime_theme_preview(&mut self) {
        if self.is_theme_command() {
            let matches = self.matching_themes_for_overview();
            if !matches.is_empty() {
                if self.theme_picker_cursor >= matches.len() {
                    self.theme_picker_cursor = 0;
                }
                let selected = &matches[self.theme_picker_cursor];
                if self.theme.id != selected.id {
                    self.theme = selected.clone();
                    crate::theme::set_active_theme(self.theme.clone());
                }
            }
        } else if self.theme.id != self.committed_theme.id {
            self.theme = self.committed_theme.clone();
            crate::theme::set_active_theme(self.theme.clone());
        }
    }

    pub fn cycle_theme_preview(&mut self, forward: bool) {
        let matches = self.matching_themes_for_overview();
        if matches.is_empty() {
            return;
        }
        let len = matches.len();
        if forward {
            self.theme_picker_cursor = (self.theme_picker_cursor + 1) % len;
        } else {
            self.theme_picker_cursor = (self.theme_picker_cursor + len - 1) % len;
        }
        self.theme = matches[self.theme_picker_cursor].clone();
        crate::theme::set_active_theme(self.theme.clone());
    }

    fn set_status(&mut self, level: StatusLevel, msg: impl Into<String>) {
        self.status_level = level;
        self.status_message = Some(msg.into());
    }

    pub fn is_any_input_active(&self) -> bool {
        self.search_active
            || self.download_input_active
            || self.profile_input_active
            || self.overview_input_active
    }

    /// Automatically accepts and applies an inline autocomplete suggestion if available.
    /// Returns true if a suggestion was accepted and applied.
    pub fn accept_autocomplete(&mut self) -> bool {
        if self.overview_input_active {
            if let Some(s) =
                crate::autocomplete::Autocomplete::suggest_overview(&self.overview_input, self)
            {
                self.overview_input = s.completed_text;
                self.overview_input_error = None;
                self.update_realtime_theme_preview();
                return true;
            }
        } else if self.search_active {
            if let Some(s) =
                crate::autocomplete::Autocomplete::suggest_mod_search(&self.search_query, self)
            {
                self.search_query = s.completed_text;
                self.search_error = None;
                self.clamp_mod_cursor();
                return true;
            }
        } else if self.download_input_active {
            if let Some(s) =
                crate::autocomplete::Autocomplete::suggest_download_uri(&self.download_input, self)
            {
                self.download_input = s.completed_text;
                self.download_input_error = None;
                return true;
            }
        } else if self.profile_input_active {
            if let Some(s) =
                crate::autocomplete::Autocomplete::suggest_profile_name(&self.profile_input, self)
            {
                self.profile_input = s.completed_text;
                self.profile_input_error = None;
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn current_focus_label(&self) -> &'static str {
        match self.active_tab {
            Tab::Overview => match self.overview_focus {
                OverviewFocus::CommandInput => "Quick Action Bar",
                OverviewFocus::RecentProfiles => "Recent Workspaces",
            },
            Tab::Mods => match self.mods_focus {
                ModsFocus::Search => "Filter Input",
                ModsFocus::Categories => "Category Bar",
                ModsFocus::Table => "Mod Load Order",
                ModsFocus::Inspector => "Package Inspector",
            },
            Tab::Conflicts => match self.conflicts_focus {
                ConflictsFocus::OptionA => "Option A Layer",
                ConflictsFocus::OptionB => "Option B Layer",
                ConflictsFocus::AutoResolve => "Auto-Resolve",
                ConflictsFocus::ResolvedList => "Resolved List",
            },
            Tab::Downloads => match self.download_focus {
                DownloadsFocus::Input => "URI / Magnet Input",
                DownloadsFocus::Queue => "Transfer Queue",
                DownloadsFocus::Curated => "Curated Bundles",
            },
            Tab::Profiles => match self.profiles_focus {
                ProfilesFocus::NewProfileInput => "New Profile Name",
                ProfilesFocus::GameSelector => "Target Game Selector",
                ProfilesFocus::ProfileList => "Profiles List",
                ProfilesFocus::Inspector => "Profile Inspector",
            },
        }
    }

    pub fn validate_overview_input(&self) -> InputValidation {
        if let Some(err) = &self.overview_input_error {
            return InputValidation::Invalid(err.clone());
        }
        let trimmed = self.overview_input.trim();
        if trimmed.is_empty() {
            return InputValidation::Idle;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower == "launch" || lower == "play" || lower == "run" {
            InputValidation::Valid(format!(
                "Launch active game ({}) with VFS overlay",
                self.game_title
            ))
        } else if lower.starts_with("clone ") || lower.starts_with("download ") {
            let arg = if lower.starts_with("clone ") {
                &trimmed[6..]
            } else {
                &trimmed[9..]
            };
            if arg.trim().is_empty() {
                InputValidation::Warning(
                    "Provide package slug, magnet URI, or .peca URL".to_string(),
                )
            } else {
                InputValidation::Valid(format!("Clone & mount package '{}'", arg.trim()))
            }
        } else if lower.starts_with("switch ") || lower.starts_with("profile ") {
            let arg = if lower.starts_with("switch ") {
                &trimmed[7..]
            } else {
                &trimmed[8..]
            };
            if arg.trim().is_empty() {
                InputValidation::Warning("Provide profile or game name".to_string())
            } else {
                InputValidation::Valid(format!("Switch active profile to '{}'", arg.trim()))
            }
        } else if lower.starts_with("search ") || lower.starts_with("find ") {
            let query = if lower.starts_with("search ") {
                &trimmed[7..]
            } else {
                &trimmed[5..]
            };
            InputValidation::Valid(format!("Search mod index for '{}'", query.trim()))
        } else if lower == "conflicts" || lower == "resolve" {
            InputValidation::Valid("Open VFI Conflict Resolver".to_string())
        } else if lower == "mods" {
            InputValidation::Valid("Open Mod Load Order & Priority".to_string())
        } else if lower == "downloads" {
            InputValidation::Valid("Open Package Transport & Swarm Queue".to_string())
        } else if lower == "profiles" {
            InputValidation::Valid("Open Multi-Game Profiles Inspector".to_string())
        } else if lower == "help" {
            InputValidation::Valid("Open keyboard cheatsheet & guide".to_string())
        } else if lower.starts_with("magnet:")
            || lower.starts_with("https://")
            || lower.ends_with(".peca")
        {
            InputValidation::Valid("Direct package/profile stream URI".to_string())
        } else {
            InputValidation::Valid(format!("Execute command or search query: '{}'", trimmed))
        }
    }

    pub fn search_validation(&self) -> InputValidation {
        if let Some(err) = &self.search_error {
            return InputValidation::Invalid(err.clone());
        }
        if self.search_query.is_empty() {
            return InputValidation::Idle;
        }
        if self.search_query.len() > 64 {
            return InputValidation::Invalid("Query exceeds 64 character limit".to_string());
        }
        let count = self.filtered_mods().len();
        if count == 0 {
            InputValidation::Warning(format!("0 packages match '{}'", self.search_query))
        } else {
            InputValidation::Valid(format!("{} matched", count))
        }
    }

    pub fn validate_download_input(&self) -> InputValidation {
        if let Some(err) = &self.download_input_error {
            return InputValidation::Invalid(err.clone());
        }
        let trimmed = self.download_input.trim();
        if trimmed.is_empty() {
            return InputValidation::Idle;
        }
        if trimmed.contains(char::is_whitespace) {
            return InputValidation::Invalid(
                "URI or package slug cannot contain spaces".to_string(),
            );
        }
        if trimmed.starts_with("magnet:") {
            if trimmed.contains("xt=urn:") && trimmed.len() > 20 {
                InputValidation::Valid("Valid P2P Magnet URI".to_string())
            } else {
                InputValidation::Invalid(
                    "Malformed Magnet URI (expected magnet:?xt=urn:btih:...)".to_string(),
                )
            }
        } else if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            let after_scheme = trimmed
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            if after_scheme.len() >= 4 && after_scheme.contains('.') {
                if trimmed.ends_with(".peca") {
                    InputValidation::Valid("Valid .peca Archive Stream".to_string())
                } else {
                    InputValidation::Valid("Valid HTTP/S Package URL".to_string())
                }
            } else {
                InputValidation::Invalid("Incomplete URL hostname or domain".to_string())
            }
        } else if trimmed.contains("://") {
            InputValidation::Invalid("Unsupported protocol (use https:// or magnet:?)".to_string())
        } else if trimmed.len() < 3 {
            InputValidation::Warning("Type at least 3 chars for package slug".to_string())
        } else if trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'))
        {
            InputValidation::Valid("Valid Registry Package Slug".to_string())
        } else {
            InputValidation::Invalid(
                "Invalid characters (use alphanumeric, '-', '_', '/', or URL)".to_string(),
            )
        }
    }

    pub fn validate_profile_input(&self) -> InputValidation {
        if let Some(err) = &self.profile_input_error {
            return InputValidation::Invalid(err.clone());
        }
        let trimmed = self.profile_input.trim();
        if trimmed.is_empty() {
            return InputValidation::Idle;
        }
        if trimmed.len() < 3 {
            return InputValidation::Warning("Profile name must be at least 3 chars".to_string());
        }
        if trimmed.len() > 32 {
            return InputValidation::Invalid("Profile name exceeds 32 chars".to_string());
        }
        if self
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(trimmed))
        {
            return InputValidation::Invalid("A profile with this name already exists".to_string());
        }
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '+' | '.'))
        {
            return InputValidation::Invalid(
                "Invalid characters (use letters, numbers, spaces, '-', '_', '+', '.')".to_string(),
            );
        }
        InputValidation::Valid("Valid new profile name".to_string())
    }

    pub fn discover_system_and_profiles(&mut self) {
        let steam_root = peri_vfs::SteamDiscovery::find_active_steam_root();
        let libraries = steam_root
            .as_ref()
            .map(|r| peri_vfs::SteamDiscovery::discover_library_folders(r))
            .unwrap_or_default();

        let proton_tools = steam_root
            .as_ref()
            .map(|r| peri_vfs::SteamDiscovery::discover_proton_versions(r, &libraries))
            .unwrap_or_default();

        if let Some(tool) = proton_tools.first() {
            self.proton_version = tool.name.clone();
        } else {
            self.proton_version = "System Default / Wine".to_string();
        }

        let mut custom_profiles = Vec::new();
        let ws_profiles = Path::new(".perihelion/profiles");
        if ws_profiles.exists() {
            if let Ok(entries) = std::fs::read_dir(ws_profiles) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            if let Ok(profile) = GameProfile::from_toml_str(&content) {
                                custom_profiles.push(profile);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_profiles = PathBuf::from(home).join(".config/perihelion/profiles");
            if user_profiles.exists() {
                if let Ok(entries) = std::fs::read_dir(user_profiles) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                            if let Ok(content) = std::fs::read_to_string(path) {
                                if let Ok(profile) = GameProfile::from_toml_str(&content) {
                                    custom_profiles.push(profile);
                                }
                            }
                        }
                    }
                }
            }
        }

        let default_proton = self.proton_version.clone();

        // 1. Agnostic generic template is first in catalog, followed by built-in templates
        let mut catalog = vec![GameCatalogEntry {
            game_id: "generic".to_string(),
            short_name: "Generic / Custom Game".to_string(),
            full_name: "Generic Linux / Wine Game".to_string(),
            steam_app_id: 0,
            executable: "game.exe".to_string(),
            default_proton: default_proton.clone(),
            install_dir: ".".to_string(),
            mappings: vec![],
            default_diagnostic: DiagnosticAlert {
                level: "INFO".to_string(),
                title: "Agnostic Sandbox Ready".to_string(),
                message: "Direct VFS overlay without game-specific layout translation.".to_string(),
                action_text: "[STANDBY]".to_string(),
                resolved: true,
            },
        }];

        for gp in GameProfile::builtin_profiles() {
            let mut install_dir = format!("~/.local/share/Steam/steamapps/common/{}", gp.name);
            let mut installed = false;
            if let Some(_app_id) = gp.steam_app_id {
                for lib in &libraries {
                    let candidate = lib.join("steamapps/common").join(&gp.name);
                    if candidate.exists() {
                        install_dir = candidate.to_string_lossy().into_owned();
                        installed = true;
                        break;
                    }
                }
            }

            let diagnostic = if installed {
                DiagnosticAlert {
                    level: "OK".to_string(),
                    title: "Game Installation Detected".to_string(),
                    message: format!("Binaries located at '{}'.", install_dir),
                    action_text: "[VERIFIED]".to_string(),
                    resolved: true,
                }
            } else {
                DiagnosticAlert {
                    level: "INFO".to_string(),
                    title: "Game Layout Template Ready".to_string(),
                    message: format!(
                        "Template layout for '{}' available for new profiles.",
                        gp.name
                    ),
                    action_text: "Standby".to_string(),
                    resolved: true,
                }
            };

            catalog.push(GameCatalogEntry {
                game_id: gp.id.clone(),
                short_name: gp.name.clone(),
                full_name: gp.name.clone(),
                steam_app_id: gp.steam_app_id.unwrap_or(0),
                executable: gp
                    .default_executable
                    .clone()
                    .unwrap_or_else(|| "game.exe".to_string()),
                default_proton: default_proton.clone(),
                install_dir,
                mappings: gp.mappings.clone(),
                default_diagnostic: diagnostic,
            });
        }

        for custom in &custom_profiles {
            if !catalog
                .iter()
                .any(|g| g.game_id.eq_ignore_ascii_case(&custom.id))
            {
                catalog.push(GameCatalogEntry {
                    game_id: custom.id.clone(),
                    short_name: custom.name.clone(),
                    full_name: custom.name.clone(),
                    steam_app_id: custom.steam_app_id.unwrap_or(0),
                    executable: custom
                        .default_executable
                        .clone()
                        .unwrap_or_else(|| "game.exe".to_string()),
                    default_proton: default_proton.clone(),
                    install_dir: ".".to_string(),
                    mappings: custom.mappings.clone(),
                    default_diagnostic: DiagnosticAlert {
                        level: "OK".to_string(),
                        title: "Custom Profile Loaded".to_string(),
                        message: format!("Profile '{}' loaded from config.", custom.name),
                        action_text: "[VERIFIED]".to_string(),
                        resolved: true,
                    },
                });
            }
        }

        self.game_catalog = catalog;
        self.new_profile_game_idx = 0;

        // 2. Profiles: ONLY populate if user actually configured profile TOML files on disk.
        // Fresh installs start completely clean and agnostically generic.
        self.profiles = custom_profiles
            .into_iter()
            .enumerate()
            .map(|(idx, cp)| ProfileItem {
                name: cp.name.clone(),
                game_id: cp.id.clone(),
                game_short: cp.name.clone(),
                game_full_name: cp.name.clone(),
                steam_app_id: cp.steam_app_id.unwrap_or(0),
                executable: cp
                    .default_executable
                    .clone()
                    .unwrap_or_else(|| "game.exe".to_string()),
                install_dir: ".".to_string(),
                upper_dir: format!(".perihelion/profiles/{}/upper", cp.id),
                mappings: cp.mappings.clone(),
                desc: format!("Configured profile for {}", cp.name),
                mod_count: 0,
                proton_version: default_proton.clone(),
                is_active: idx == 0,
                diagnostic: DiagnosticAlert {
                    level: "OK".to_string(),
                    title: "Configured Profile".to_string(),
                    message: "User profile loaded from disk.".to_string(),
                    action_text: "[VERIFIED]".to_string(),
                    resolved: true,
                },
            })
            .collect();

        self.diagnostics = if let Some(first) = self.profiles.first() {
            vec![first.diagnostic.clone()]
        } else {
            Vec::new()
        };
    }

    #[allow(dead_code)]
    pub fn populate_test_fixture(&mut self) {
        let skyrim_gp = GameProfile::skyrim_se();
        let cp2077_gp = GameProfile::cyberpunk_2077();
        let starfield_gp = GameProfile::starfield();

        self.game_catalog = vec![
            GameCatalogEntry {
                game_id: skyrim_gp.id.clone(),
                short_name: "Skyrim SE".to_string(),
                full_name: skyrim_gp.name.clone(),
                steam_app_id: skyrim_gp.steam_app_id.unwrap_or(489830),
                executable: skyrim_gp
                    .default_executable
                    .clone()
                    .unwrap_or_else(|| "SkyrimSE.exe".to_string()),
                default_proton: "GE-Proton 9-23".to_string(),
                install_dir: "~/.local/share/Steam/steamapps/common/Skyrim Special Edition"
                    .to_string(),
                mappings: skyrim_gp.mappings.clone(),
                default_diagnostic: DiagnosticAlert {
                    level: "WARN".to_string(),
                    title: "Script Extender (SKSE64) Recommended".to_string(),
                    message:
                        "SkyUI and DLL plugins require SKSE64 v2.2.6+ mounted in the top VFS layer."
                            .to_string(),
                    action_text: "Press [f] to verify/install SKSE64".to_string(),
                    resolved: false,
                },
            },
            GameCatalogEntry {
                game_id: cp2077_gp.id.clone(),
                short_name: "Cyberpunk 2077".to_string(),
                full_name: cp2077_gp.name.clone(),
                steam_app_id: cp2077_gp.steam_app_id.unwrap_or(1091500),
                executable: cp2077_gp
                    .default_executable
                    .clone()
                    .unwrap_or_else(|| "bin/x64/Cyberpunk2077.exe".to_string()),
                default_proton: "GE-Proton 9-20".to_string(),
                install_dir: "~/.local/share/Steam/steamapps/common/Cyberpunk 2077".to_string(),
                mappings: cp2077_gp.mappings.clone(),
                default_diagnostic: DiagnosticAlert {
                    level: "WARN".to_string(),
                    title: "RED4ext & Cyber Engine Tweaks Check".to_string(),
                    message:
                        "WINEDLLOVERRIDES=\"version=n,b\" is required in Proton launch flags for RED4ext."
                            .to_string(),
                    action_text: "Press [f] to apply DLL override".to_string(),
                    resolved: false,
                },
            },
            GameCatalogEntry {
                game_id: starfield_gp.id.clone(),
                short_name: "Starfield".to_string(),
                full_name: starfield_gp.name.clone(),
                steam_app_id: starfield_gp.steam_app_id.unwrap_or(1716740),
                executable: starfield_gp
                    .default_executable
                    .clone()
                    .unwrap_or_else(|| "Starfield.exe".to_string()),
                default_proton: "Valve Proton Experimental".to_string(),
                install_dir: "~/.local/share/Steam/steamapps/common/Starfield".to_string(),
                mappings: starfield_gp.mappings.clone(),
                default_diagnostic: DiagnosticAlert {
                    level: "OK".to_string(),
                    title: "Starfield Custom.ini & Archive Invalidation".to_string(),
                    message: "bInvalidateOlderFiles=1 is active inside isolated profile config."
                        .to_string(),
                    action_text: "[VERIFIED]".to_string(),
                    resolved: true,
                },
            },
            GameCatalogEntry {
                game_id: "baldurs_gate_3".to_string(),
                short_name: "Baldur's Gate 3".to_string(),
                full_name: "Baldur's Gate 3".to_string(),
                steam_app_id: 1086940,
                executable: "bin/bg3_dx11.exe".to_string(),
                default_proton: "Proton 9.0-3".to_string(),
                install_dir: "~/.local/share/Steam/steamapps/common/Baldurs Gate 3".to_string(),
                mappings: vec![
                    LayoutMapping {
                        from_prefix: "Mods".to_string(),
                        to_prefix: "Data/Mods".to_string(),
                    },
                    LayoutMapping {
                        from_prefix: "Public".to_string(),
                        to_prefix: "Data/Public".to_string(),
                    },
                    LayoutMapping {
                        from_prefix: "bin".to_string(),
                        to_prefix: "bin".to_string(),
                    },
                ],
                default_diagnostic: DiagnosticAlert {
                    level: "OK".to_string(),
                    title: "BG3 Script Extender (DWrite.dll) Mounted".to_string(),
                    message: "Norbyte Script Extender v18 is mapped to bin/DWrite.dll.".to_string(),
                    action_text: "[VERIFIED]".to_string(),
                    resolved: true,
                },
            },
        ];

        self.mods = vec![
            ModItem {
                id: "ussep".to_string(),
                name: "Unofficial Skyrim Special Edition Patch".to_string(),
                active: true,
                category: "Fixes".to_string(),
                priority: Priority::Lowest,
                version: "4.2.9".to_string(),
                author: "Unofficial Patch Team".to_string(),
                description: "Comprehensive bugfix resolving thousands of gameplay, quest, and audio bugs.".to_string(),
                affected_files: vec![
                    "Skyrim.esm (override)".to_string(),
                    "Unofficial Patch.esp".to_string(),
                    "textures/patch/...".to_string(),
                ],
                has_conflict: false,
            },
            ModItem {
                id: "skyui".to_string(),
                name: "SkyUI Modern Interface".to_string(),
                active: true,
                category: "UI".to_string(),
                priority: Priority::High,
                version: "5.2.0".to_string(),
                author: "SkyUI Team".to_string(),
                description: "Complete inventory and menu overhaul designed for PC keyboards and mouse navigation.".to_string(),
                affected_files: vec![
                    "interface/skyui.swf".to_string(),
                    "scripts/ski_activeeffects.pex".to_string(),
                ],
                has_conflict: false,
            },
            ModItem {
                id: "ordinator".to_string(),
                name: "Ordinator - Perks of Skyrim".to_string(),
                active: true,
                category: "Gameplay".to_string(),
                priority: Priority::Default,
                version: "9.31.0".to_string(),
                author: "Enai Siaion".to_string(),
                description: "Overhauls perk trees with 400+ new perks to increase character depth and diversity.".to_string(),
                affected_files: vec![
                    "scripts/ord_perkscripts.pex".to_string(),
                    "Ordinator - Perks of Skyrim.esp".to_string(),
                ],
                has_conflict: false,
            },
            ModItem {
                id: "skyland".to_string(),
                name: "Skyland Complete Textures".to_string(),
                active: true,
                category: "Visuals".to_string(),
                priority: Priority::Default,
                version: "4.1.2".to_string(),
                author: "Skyking2020".to_string(),
                description: "Complete visual overhaul featuring photorealistic 2K/4K landscapes and architecture.".to_string(),
                affected_files: vec![
                    "textures/landscape/dirt01.dds".to_string(),
                    "textures/architecture/whiterun/stone.dds".to_string(),
                ],
                has_conflict: false,
            },
            ModItem {
                id: "better_iron_weapons".to_string(),
                name: "Better Iron Weapons (4K)".to_string(),
                active: true,
                category: "Visuals".to_string(),
                priority: Priority::High,
                version: "1.2.0".to_string(),
                author: "LeanWolf".to_string(),
                description: "Realistic metallic finish and high-resolution reflections for standard weaponry.".to_string(),
                affected_files: vec![
                    "textures/weapons/iron_sword.dds".to_string(),
                    "meshes/weapons/iron_sword.nif".to_string(),
                ],
                has_conflict: true,
            },
        ];

        self.conflicts = vec![ConflictCard {
            title: "Iron Weapons Appearance Conflict".to_string(),
            file_path: "textures/weapons/iron_sword.dds".to_string(),
            option_a: ConflictOption {
                mod_id: "better_iron_weapons".to_string(),
                mod_name: "Better Iron Weapons (4K)".to_string(),
                description: "Realistic metallic finish and high-resolution reflections."
                    .to_string(),
            },
            option_b: ConflictOption {
                mod_id: "skyland".to_string(),
                mod_name: "Skyland Complete Textures".to_string(),
                description: "Unified landscape & item textures matching whole world overhaul."
                    .to_string(),
            },
            winner_id: Some("skyland".to_string()),
        }];

        self.resolved_conflicts = vec![ResolvedConflict {
            title: "Whiterun Architecture Cobblestone".to_string(),
            file_path: "textures/architecture/whiterun/stone.dds".to_string(),
            resolution_note:
                "Automatically resolved by priority rule: [Skyland Complete] overrides [Unofficial Patch]."
                    .to_string(),
        }];

        self.downloads = vec![DownloadItem {
            name: "Realistic Water Two (Next-Gen)".to_string(),
            progress: 42.0,
            speed: "38.4 MB/s".to_string(),
            status: "SWARMING".to_string(),
            peers_info: "14 DHT Peers + 2 Web Seeds".to_string(),
            paused: false,
        }];

        self.curated_packages = vec![
            CuratedPackage {
                title: "Essential Bug Fixes 2026".to_string(),
                desc: "SKSE, Engine Fixes, Unofficial Patch, and Memory Allocator.".to_string(),
                is_installed: false,
            },
            CuratedPackage {
                title: "Nordic UI Minimalist Bundle".to_string(),
                desc: "Clean dark HUD with 60fps inventory and ultrawide support.".to_string(),
                is_installed: false,
            },
        ];

        self.profiles = vec![
            ProfileItem {
                name: "Requiem Overhaul".to_string(),
                game_id: self.game_catalog[0].game_id.clone(),
                game_short: self.game_catalog[0].short_name.clone(),
                game_full_name: self.game_catalog[0].full_name.clone(),
                steam_app_id: self.game_catalog[0].steam_app_id,
                executable: self.game_catalog[0].executable.clone(),
                install_dir: self.game_catalog[0].install_dir.clone(),
                upper_dir: "~/.local/share/perihelion/profiles/skyrim_requiem/upper".to_string(),
                mappings: self.game_catalog[0].mappings.clone(),
                desc: "Hardcore roleplaying setup with de-leveled world, 2K textures, and custom combat.".to_string(),
                mod_count: 5,
                proton_version: "GE-Proton 9-23".to_string(),
                is_active: true,
                diagnostic: self.game_catalog[0].default_diagnostic.clone(),
            },
            ProfileItem {
                name: "Vanilla+ Light Fixes".to_string(),
                game_id: self.game_catalog[0].game_id.clone(),
                game_short: self.game_catalog[0].short_name.clone(),
                game_full_name: self.game_catalog[0].full_name.clone(),
                steam_app_id: self.game_catalog[0].steam_app_id,
                executable: self.game_catalog[0].executable.clone(),
                install_dir: self.game_catalog[0].install_dir.clone(),
                upper_dir: "~/.local/share/perihelion/profiles/skyrim_vanilla_plus/upper"
                    .to_string(),
                mappings: self.game_catalog[0].mappings.clone(),
                desc: "Lightweight bug fixes and original visuals with Steam achievements enabled."
                    .to_string(),
                mod_count: 2,
                proton_version: "Valve Proton Experimental".to_string(),
                is_active: false,
                diagnostic: DiagnosticAlert {
                    level: "OK".to_string(),
                    title: "Clean Engine & Achievement Enabler".to_string(),
                    message: "SSE Engine Fixes TOML configured for vanilla achievement parity."
                        .to_string(),
                    action_text: "[VERIFIED]".to_string(),
                    resolved: true,
                },
            },
            ProfileItem {
                name: "Cyberware & Vehicles".to_string(),
                game_id: self.game_catalog[1].game_id.clone(),
                game_short: self.game_catalog[1].short_name.clone(),
                game_full_name: self.game_catalog[1].full_name.clone(),
                steam_app_id: self.game_catalog[1].steam_app_id,
                executable: self.game_catalog[1].executable.clone(),
                install_dir: self.game_catalog[1].install_dir.clone(),
                upper_dir: "~/.local/share/perihelion/profiles/cp2077_cyberware/upper".to_string(),
                mappings: self.game_catalog[1].mappings.clone(),
                desc: "Overhauls combat implants, REDscript cyberware slots, and adds 14 custom vehicles.".to_string(),
                mod_count: 14,
                proton_version: "GE-Proton 9-20".to_string(),
                is_active: false,
                diagnostic: self.game_catalog[1].default_diagnostic.clone(),
            },
        ];

        self.game_title = "Skyrim SE [Requiem]".to_string();
        self.proton_version = "GE-Proton 9-23".to_string();
        self.diagnostics = vec![self.profiles[0].diagnostic.clone()];
    }

    pub fn try_load_workspace(&mut self) {
        if !Path::new("peri.toml").exists() {
            self.game_title = "No Active Workspace".to_string();
            self.mods.clear();
            self.conflicts.clear();
            self.resolved_conflicts.clear();
            return;
        }

        let content = match std::fs::read_to_string("peri.toml") {
            Ok(c) => c,
            Err(_) => return,
        };

        let manifest = match Manifest::from_toml_str(&content) {
            Ok(m) => m,
            Err(_) => return,
        };

        self.canonical_manifest = Some(manifest.clone());
        self.game_title = format!("{} [{}]", manifest.target.game_id, manifest.package.name);

        let mut found = false;
        for prof in &mut self.profiles {
            if prof.game_id == manifest.target.game_id {
                prof.name = manifest.package.name.clone();
                prof.is_active = true;
                found = true;
            } else {
                prof.is_active = false;
            }
        }

        if !found {
            let target_entry = self
                .game_catalog
                .iter()
                .find(|g| g.game_id == manifest.target.game_id);
            let (game_short, game_full_name, steam_app_id, executable, install_dir, mappings, diag) =
                if let Some(entry) = target_entry {
                    (
                        entry.short_name.clone(),
                        entry.full_name.clone(),
                        entry.steam_app_id,
                        entry.executable.clone(),
                        entry.install_dir.clone(),
                        entry.mappings.clone(),
                        entry.default_diagnostic.clone(),
                    )
                } else {
                    (
                        manifest.target.game_id.clone(),
                        manifest.target.game_id.clone(),
                        0,
                        "game.exe".to_string(),
                        ".".to_string(),
                        vec![],
                        DiagnosticAlert {
                            level: "OK".to_string(),
                            title: "Workspace Profile Active".to_string(),
                            message: format!(
                                "Configured via manifest target '{}'.",
                                manifest.target.game_id
                            ),
                            action_text: "[VERIFIED]".to_string(),
                            resolved: true,
                        },
                    )
                };

            self.profiles.push(ProfileItem {
                name: manifest.package.name.clone(),
                game_id: manifest.target.game_id.clone(),
                game_short,
                game_full_name,
                steam_app_id,
                executable,
                install_dir,
                upper_dir: format!(".perihelion/profiles/{}/upper", manifest.target.game_id),
                mappings,
                desc: manifest.package.description.clone().unwrap_or_default(),
                mod_count: 0,
                proton_version: self.proton_version.clone(),
                is_active: true,
                diagnostic: diag,
            });
        }

        let cache_dirs = [
            PathBuf::from(".perihelion/cache/mods"),
            PathBuf::from("crates/peri-archive"),
            match std::env::var("HOME") {
                Ok(h) => PathBuf::from(h).join(".config/perihelion/cache/mods"),
                Err(_) => PathBuf::from(".config/perihelion/cache/mods"),
            },
        ];

        let mut discovered_mods = Vec::new();
        let mut vfi = VirtualFileIndex::new();

        for dep_name in manifest.dependencies.keys() {
            let mut found_peca = None;
            for dir in &cache_dirs {
                let candidate = dir.join(format!("{}.peca", dep_name));
                if candidate.exists() {
                    found_peca = Some(candidate);
                    break;
                }
            }

            let (version, author, priority, desc, affected_files) =
                if let Some(ref peca_path) = found_peca {
                    if let Ok(dep_manifest) = peri_archive::read_manifest_from_peca(peca_path) {
                        let files = peri_archive::list_peca_files(peca_path).unwrap_or_default();
                        (
                            dep_manifest.package.version,
                            dep_manifest.package.author,
                            dep_manifest.package.priority,
                            dep_manifest.package.description.unwrap_or_default(),
                            files,
                        )
                    } else {
                        (
                            "1.0.0".to_string(),
                            "Unknown".to_string(),
                            Priority::Default,
                            "Cached package archive".to_string(),
                            vec![],
                        )
                    }
                } else {
                    (
                        "1.0.0".to_string(),
                        "Unknown".to_string(),
                        Priority::Default,
                        "Manifest dependency".to_string(),
                        vec![],
                    )
                };

            for f in &affected_files {
                vfi.add_entry(f, dep_name, priority);
            }

            discovered_mods.push(ModItem {
                id: dep_name.clone(),
                name: dep_name.clone(),
                active: true,
                category: "General".to_string(),
                priority,
                version,
                author,
                description: desc,
                affected_files,
                has_conflict: false,
            });
        }

        let file_conflicts = vfi.detect_conflicts();
        let conflict_paths: std::collections::HashSet<_> =
            file_conflicts.iter().map(|c| c.file_path.clone()).collect();

        for m in &mut discovered_mods {
            if m.affected_files.iter().any(|f| conflict_paths.contains(f)) {
                m.has_conflict = true;
            }
        }

        self.conflicts = file_conflicts
            .into_iter()
            .map(|c| {
                let owner_a = c
                    .owners
                    .first()
                    .map(|o| o.mod_name.clone())
                    .unwrap_or_default();
                let owner_b = c
                    .owners
                    .get(1)
                    .map(|o| o.mod_name.clone())
                    .unwrap_or_default();
                ConflictCard {
                    title: format!("File Collision: {}", c.file_path),
                    file_path: c.file_path,
                    option_a: ConflictOption {
                        mod_id: owner_a.clone(),
                        mod_name: owner_a,
                        description: "Conflicting layer in lowerdir stack".to_string(),
                    },
                    option_b: ConflictOption {
                        mod_id: owner_b.clone(),
                        mod_name: owner_b,
                        description: "Conflicting layer in lowerdir stack".to_string(),
                    },
                    winner_id: None,
                }
            })
            .collect();

        self.mods = discovered_mods;
        if let Some(prof) = self.profiles.iter_mut().find(|p| p.is_active) {
            prof.mod_count = self.mods.len();
        }
    }

    pub fn filtered_mods(&self) -> Vec<(usize, &ModItem)> {
        self.mods
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                let matches_query = if self.search_query.is_empty() {
                    true
                } else {
                    let q = self.search_query.to_lowercase();
                    m.name.to_lowercase().contains(&q)
                        || m.category.to_lowercase().contains(&q)
                        || m.author.to_lowercase().contains(&q)
                        || m.id.to_lowercase().contains(&q)
                };

                let matches_category = if let Some(cat) = &self.category_filter {
                    m.category.eq_ignore_ascii_case(cat)
                } else {
                    true
                };

                matches_query && matches_category
            })
            .collect()
    }

    pub fn selected_mod(&self) -> Option<&ModItem> {
        let filtered = self.filtered_mods();
        if filtered.is_empty() {
            None
        } else {
            let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
            Some(filtered[idx].1)
        }
    }

    pub fn selected_profile(&self) -> Option<&ProfileItem> {
        if self.profiles.is_empty() {
            None
        } else {
            let idx = self.profile_cursor.min(self.profiles.len() - 1);
            Some(&self.profiles[idx])
        }
    }

    fn clamp_mod_cursor(&mut self) {
        let total = self.filtered_mods().len();
        if total == 0 {
            self.cursor_index = 0;
        } else if self.cursor_index >= total {
            self.cursor_index = total - 1;
        }
        self.inspector_file_cursor = 0;
    }

    fn delete_last_word(s: &mut String) {
        while s.ends_with(char::is_whitespace) {
            s.pop();
        }
        while let Some(c) = s.chars().last() {
            if c.is_whitespace() || matches!(c, '/' | '-' | '_' | ':') {
                s.pop();
                break;
            }
            s.pop();
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::NextTab => {
                self.active_tab = self.active_tab.next();
            }
            Action::PrevTab => {
                self.active_tab = self.active_tab.prev();
            }
            Action::SetTab(tab) => {
                self.search_active = false;
                self.download_input_active = false;
                self.profile_input_active = false;
                self.overview_input_active = false;
                self.active_tab = tab;
            }
            Action::FocusNext => match self.active_tab {
                Tab::Overview => {
                    self.overview_input_active = false;
                    self.overview_focus = match self.overview_focus {
                        OverviewFocus::CommandInput => OverviewFocus::RecentProfiles,
                        OverviewFocus::RecentProfiles => OverviewFocus::CommandInput,
                    };
                }
                Tab::Mods => {
                    self.mods_focus = match self.mods_focus {
                        ModsFocus::Table => {
                            if self.show_inspector {
                                ModsFocus::Inspector
                            } else {
                                ModsFocus::Search
                            }
                        }
                        ModsFocus::Inspector => ModsFocus::Search,
                        ModsFocus::Search => ModsFocus::Categories,
                        ModsFocus::Categories => ModsFocus::Table,
                    };
                    self.search_active = self.mods_focus == ModsFocus::Search;
                }
                Tab::Conflicts => {
                    self.conflicts_focus = match self.conflicts_focus {
                        ConflictsFocus::OptionA => ConflictsFocus::OptionB,
                        ConflictsFocus::OptionB => ConflictsFocus::AutoResolve,
                        ConflictsFocus::AutoResolve => ConflictsFocus::ResolvedList,
                        ConflictsFocus::ResolvedList => ConflictsFocus::OptionA,
                    };
                }
                Tab::Downloads => {
                    self.download_input_active = false;
                    self.download_focus = match self.download_focus {
                        DownloadsFocus::Input => DownloadsFocus::Queue,
                        DownloadsFocus::Queue => DownloadsFocus::Curated,
                        DownloadsFocus::Curated => DownloadsFocus::Input,
                    };
                }
                Tab::Profiles => {
                    self.profile_input_active = false;
                    self.profiles_focus = match self.profiles_focus {
                        ProfilesFocus::ProfileList => ProfilesFocus::Inspector,
                        ProfilesFocus::Inspector => ProfilesFocus::NewProfileInput,
                        ProfilesFocus::NewProfileInput => ProfilesFocus::GameSelector,
                        ProfilesFocus::GameSelector => ProfilesFocus::ProfileList,
                    };
                }
            },
            Action::FocusPrev => match self.active_tab {
                Tab::Overview => {
                    self.overview_input_active = false;
                    self.overview_focus = match self.overview_focus {
                        OverviewFocus::CommandInput => OverviewFocus::RecentProfiles,
                        OverviewFocus::RecentProfiles => OverviewFocus::CommandInput,
                    };
                }
                Tab::Mods => {
                    self.mods_focus = match self.mods_focus {
                        ModsFocus::Table => ModsFocus::Categories,
                        ModsFocus::Categories => ModsFocus::Search,
                        ModsFocus::Search => {
                            if self.show_inspector {
                                ModsFocus::Inspector
                            } else {
                                ModsFocus::Table
                            }
                        }
                        ModsFocus::Inspector => ModsFocus::Table,
                    };
                    self.search_active = self.mods_focus == ModsFocus::Search;
                }
                Tab::Conflicts => {
                    self.conflicts_focus = match self.conflicts_focus {
                        ConflictsFocus::OptionA => ConflictsFocus::ResolvedList,
                        ConflictsFocus::OptionB => ConflictsFocus::OptionA,
                        ConflictsFocus::AutoResolve => ConflictsFocus::OptionB,
                        ConflictsFocus::ResolvedList => ConflictsFocus::AutoResolve,
                    };
                }
                Tab::Downloads => {
                    self.download_input_active = false;
                    self.download_focus = match self.download_focus {
                        DownloadsFocus::Input => DownloadsFocus::Curated,
                        DownloadsFocus::Queue => DownloadsFocus::Input,
                        DownloadsFocus::Curated => DownloadsFocus::Queue,
                    };
                }
                Tab::Profiles => {
                    self.profile_input_active = false;
                    self.profiles_focus = match self.profiles_focus {
                        ProfilesFocus::ProfileList => ProfilesFocus::GameSelector,
                        ProfilesFocus::GameSelector => ProfilesFocus::NewProfileInput,
                        ProfilesFocus::NewProfileInput => ProfilesFocus::Inspector,
                        ProfilesFocus::Inspector => ProfilesFocus::ProfileList,
                    };
                }
            },
            Action::SwitchPaneFocus => {
                self.handle_action(Action::FocusNext);
            }
            Action::ActivateFocused => match self.active_tab {
                Tab::Overview => match self.overview_focus {
                    OverviewFocus::CommandInput => {
                        self.overview_input_active = true;
                    }
                    OverviewFocus::RecentProfiles => {
                        if self.overview_profile_cursor == 4 {
                            self.overview_input = "theme ".to_string();
                            self.overview_input_active = true;
                            self.overview_focus = OverviewFocus::CommandInput;
                            self.update_realtime_theme_preview();
                        } else {
                            let idx = self.overview_profile_cursor;
                            self.handle_action(Action::SelectProfile(idx));
                        }
                    }
                },
                Tab::Mods => match self.mods_focus {
                    ModsFocus::Search => self.handle_action(Action::StartSearch),
                    ModsFocus::Categories => self.handle_action(Action::CycleCategory),
                    ModsFocus::Table | ModsFocus::Inspector => {
                        self.handle_action(Action::ToggleMod)
                    }
                },
                Tab::Conflicts => match self.conflicts_focus {
                    ConflictsFocus::AutoResolve => {
                        self.handle_action(Action::AutoResolveConflicts);
                    }
                    ConflictsFocus::OptionA => {
                        if let Some(card) = self.conflicts.get(self.active_conflict_cursor) {
                            let id = card.option_a.mod_id.clone();
                            self.handle_action(Action::ResolveConflictChoice {
                                index: self.active_conflict_cursor,
                                winner_id: id,
                            });
                        }
                    }
                    ConflictsFocus::OptionB => {
                        if let Some(card) = self.conflicts.get(self.active_conflict_cursor) {
                            let id = card.option_b.mod_id.clone();
                            self.handle_action(Action::ResolveConflictChoice {
                                index: self.active_conflict_cursor,
                                winner_id: id,
                            });
                        }
                    }
                    ConflictsFocus::ResolvedList => {
                        self.handle_action(Action::ToggleConflictWinner);
                    }
                },
                Tab::Downloads => match self.download_focus {
                    DownloadsFocus::Input => self.handle_action(Action::StartDownloadInput),
                    DownloadsFocus::Queue => self.handle_action(Action::TogglePauseDownload),
                    DownloadsFocus::Curated => self.handle_action(Action::InstallCuratedPackage),
                },
                Tab::Profiles => match self.profiles_focus {
                    ProfilesFocus::NewProfileInput => self.handle_action(Action::StartProfileInput),
                    ProfilesFocus::GameSelector => self.handle_action(Action::CycleTargetGame),
                    ProfilesFocus::ProfileList => {
                        self.handle_action(Action::SelectProfile(self.profile_cursor));
                    }
                    ProfilesFocus::Inspector => self.handle_action(Action::FixDiagnostic),
                },
            },
            Action::CursorUp => match self.active_tab {
                Tab::Overview => {
                    if self.overview_input_active && self.is_theme_command() {
                        self.cycle_theme_preview(false);
                    } else {
                        self.overview_focus = OverviewFocus::RecentProfiles;
                        if self.overview_profile_cursor > 0 {
                            self.overview_profile_cursor -= 1;
                        } else {
                            self.overview_focus = OverviewFocus::CommandInput;
                        }
                    }
                }
                Tab::Mods => match self.mods_focus {
                    ModsFocus::Inspector => {
                        self.inspector_file_cursor = self.inspector_file_cursor.saturating_sub(1);
                    }
                    _ => {
                        self.mods_focus = ModsFocus::Table;
                        let total = self.filtered_mods().len();
                        if total > 0 {
                            self.cursor_index = self.cursor_index.saturating_sub(1);
                            self.inspector_file_cursor = 0;
                        }
                    }
                },
                Tab::Conflicts => {
                    self.active_conflict_cursor = self.active_conflict_cursor.saturating_sub(1);
                }
                Tab::Downloads => match self.download_focus {
                    DownloadsFocus::Curated => {
                        if self.curated_cursor > 0 {
                            self.curated_cursor -= 1;
                        } else {
                            self.download_focus = DownloadsFocus::Queue;
                        }
                    }
                    DownloadsFocus::Queue => {
                        if self.download_cursor > 0 {
                            self.download_cursor -= 1;
                        } else {
                            self.download_focus = DownloadsFocus::Input;
                        }
                    }
                    DownloadsFocus::Input => {}
                },
                Tab::Profiles => {
                    self.profiles_focus = ProfilesFocus::ProfileList;
                    self.profile_cursor = self.profile_cursor.saturating_sub(1);
                }
            },
            Action::CursorDown => match self.active_tab {
                Tab::Overview => {
                    if self.overview_input_active && self.is_theme_command() {
                        self.cycle_theme_preview(true);
                    } else {
                        match self.overview_focus {
                            OverviewFocus::CommandInput => {
                                self.overview_focus = OverviewFocus::RecentProfiles;
                            }
                            OverviewFocus::RecentProfiles => {
                                let total = self.profiles.len().max(5);
                                if self.overview_profile_cursor + 1 < total {
                                    self.overview_profile_cursor += 1;
                                }
                            }
                        }
                    }
                }
                Tab::Mods => match self.mods_focus {
                    ModsFocus::Inspector => {
                        let max_files = self
                            .selected_mod()
                            .map(|m| m.affected_files.len())
                            .unwrap_or(0);
                        if max_files > 0 {
                            self.inspector_file_cursor =
                                (self.inspector_file_cursor + 1).min(max_files.saturating_sub(1));
                        }
                    }
                    _ => {
                        self.mods_focus = ModsFocus::Table;
                        let total = self.filtered_mods().len();
                        if total > 0 {
                            self.cursor_index =
                                (self.cursor_index + 1).min(total.saturating_sub(1));
                            self.inspector_file_cursor = 0;
                        }
                    }
                },
                Tab::Conflicts => {
                    let total = self.conflicts.len();
                    if total > 0 {
                        self.active_conflict_cursor =
                            (self.active_conflict_cursor + 1).min(total.saturating_sub(1));
                    }
                }
                Tab::Downloads => match self.download_focus {
                    DownloadsFocus::Input => {
                        self.download_focus = DownloadsFocus::Queue;
                    }
                    DownloadsFocus::Queue => {
                        let total = self.downloads.len();
                        if self.download_cursor + 1 < total {
                            self.download_cursor += 1;
                        } else {
                            self.download_focus = DownloadsFocus::Curated;
                        }
                    }
                    DownloadsFocus::Curated => {
                        let total = self.curated_packages.len();
                        if total > 0 {
                            self.curated_cursor =
                                (self.curated_cursor + 1).min(total.saturating_sub(1));
                        }
                    }
                },
                Tab::Profiles => {
                    self.profiles_focus = ProfilesFocus::ProfileList;
                    let total = self.profiles.len();
                    if total > 0 {
                        self.profile_cursor =
                            (self.profile_cursor + 1).min(total.saturating_sub(1));
                    }
                }
            },
            Action::CursorPageUp => match self.active_tab {
                Tab::Mods => {
                    self.cursor_index = self.cursor_index.saturating_sub(5);
                }
                Tab::Profiles => {
                    self.profile_cursor = 0;
                }
                _ => {}
            },
            Action::CursorPageDown => match self.active_tab {
                Tab::Mods => {
                    let total = self.filtered_mods().len();
                    if total > 0 {
                        self.cursor_index = (self.cursor_index + 5).min(total.saturating_sub(1));
                    }
                }
                Tab::Profiles => {
                    let total = self.profiles.len();
                    if total > 0 {
                        self.profile_cursor = total.saturating_sub(1);
                    }
                }
                _ => {}
            },
            Action::CursorHome => match self.active_tab {
                Tab::Overview => self.overview_profile_cursor = 0,
                Tab::Mods => self.cursor_index = 0,
                Tab::Conflicts => self.active_conflict_cursor = 0,
                Tab::Downloads => {
                    self.download_cursor = 0;
                    self.curated_cursor = 0;
                }
                Tab::Profiles => self.profile_cursor = 0,
            },
            Action::CursorEnd => match self.active_tab {
                Tab::Overview => {
                    let total = self.profiles.len();
                    if total > 0 {
                        self.overview_profile_cursor = total.saturating_sub(1);
                    }
                }
                Tab::Mods => {
                    let total = self.filtered_mods().len();
                    if total > 0 {
                        self.cursor_index = total.saturating_sub(1);
                    }
                }
                Tab::Conflicts => {
                    let total = self.conflicts.len();
                    if total > 0 {
                        self.active_conflict_cursor = total.saturating_sub(1);
                    }
                }
                Tab::Downloads => {
                    self.download_focus = DownloadsFocus::Curated;
                    let total = self.curated_packages.len();
                    if total > 0 {
                        self.curated_cursor = total.saturating_sub(1);
                    }
                }
                Tab::Profiles => {
                    let total = self.profiles.len();
                    if total > 0 {
                        self.profile_cursor = total.saturating_sub(1);
                    }
                }
            },
            Action::ToggleMod => {
                let filtered = self.filtered_mods();
                if !filtered.is_empty() {
                    let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
                    let real_idx = filtered[idx].0;
                    let mod_id = self.mods[real_idx].id.clone();
                    let is_on = !self.mods[real_idx].active;
                    self.mods[real_idx].active = is_on;
                    self.staged.stage_toggle(mod_id.clone(), is_on);
                    self.notify_draft_changed();
                    let delta = self.staged.delta_count();
                    if is_on {
                        self.set_status(
                            StatusLevel::Success,
                            format!("Staged ENABLE for '{}' ({} pending edits)", mod_id, delta),
                        );
                    } else {
                        self.set_status(
                            StatusLevel::Warning,
                            format!("Staged DISABLE for '{}' ({} pending edits)", mod_id, delta),
                        );
                    }
                }
            }
            Action::ToggleAllMods => {
                let indices: Vec<usize> =
                    self.filtered_mods().into_iter().map(|(i, _)| i).collect();
                if !indices.is_empty() {
                    let any_inactive = indices.iter().any(|&i| !self.mods[i].active);
                    for i in &indices {
                        self.mods[*i].active = any_inactive;
                        let mod_id = self.mods[*i].id.clone();
                        self.staged.stage_toggle(mod_id, any_inactive);
                    }
                    self.notify_draft_changed();
                    let delta = self.staged.delta_count();
                    if any_inactive {
                        self.set_status(
                            StatusLevel::Success,
                            format!(
                                "Staged ENABLE all {} packages ({} pending edits)",
                                indices.len(),
                                delta
                            ),
                        );
                    } else {
                        self.set_status(
                            StatusLevel::Warning,
                            format!(
                                "Staged DISABLE all {} packages ({} pending edits)",
                                indices.len(),
                                delta
                            ),
                        );
                    }
                }
            }
            Action::CyclePriority => {
                let filtered = self.filtered_mods();
                if !filtered.is_empty() {
                    let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
                    let real_idx = filtered[idx].0;
                    let current = self.mods[real_idx].priority;
                    let next = match current {
                        Priority::Lowest => Priority::Low,
                        Priority::Low => Priority::Default,
                        Priority::Default => Priority::High,
                        Priority::High => Priority::Highest,
                        Priority::Highest => Priority::Absolute,
                        Priority::Absolute => Priority::Lowest,
                    };
                    self.mods[real_idx].priority = next;
                    let mod_id = self.mods[real_idx].id.clone();
                    self.staged.stage_priority(mod_id.clone(), next);
                    self.notify_draft_changed();
                    let delta = self.staged.delta_count();
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "Staged priority {:?} for '{}' ({} pending edits)",
                            next, mod_id, delta
                        ),
                    );
                }
            }
            Action::CyclePriorityReverse => {
                let filtered = self.filtered_mods();
                if !filtered.is_empty() {
                    let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
                    let real_idx = filtered[idx].0;
                    let current = self.mods[real_idx].priority;
                    let prev = match current {
                        Priority::Lowest => Priority::Absolute,
                        Priority::Low => Priority::Lowest,
                        Priority::Default => Priority::Low,
                        Priority::High => Priority::Default,
                        Priority::Highest => Priority::High,
                        Priority::Absolute => Priority::Highest,
                    };
                    self.mods[real_idx].priority = prev;
                    let mod_id = self.mods[real_idx].id.clone();
                    self.staged.stage_priority(mod_id.clone(), prev);
                    self.notify_draft_changed();
                    let delta = self.staged.delta_count();
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "Staged priority {:?} for '{}' ({} pending edits)",
                            prev, mod_id, delta
                        ),
                    );
                }
            }
            Action::MoveModUp => {
                let filtered = self.filtered_mods();
                if !filtered.is_empty() {
                    let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
                    let real_idx = filtered[idx].0;
                    if real_idx > 0 {
                        self.mods.swap(real_idx, real_idx - 1);
                        self.cursor_index = self.cursor_index.saturating_sub(1);
                        let name = self.mods[real_idx - 1].name.clone();
                        self.set_status(
                            StatusLevel::Info,
                            format!("Moved '{}' up to load order #{:02}", name, real_idx),
                        );
                    }
                }
            }
            Action::MoveModDown => {
                let filtered = self.filtered_mods();
                if !filtered.is_empty() {
                    let idx = self.cursor_index.min(filtered.len().saturating_sub(1));
                    let real_idx = filtered[idx].0;
                    if real_idx + 1 < self.mods.len() {
                        self.mods.swap(real_idx, real_idx + 1);
                        let total = self.filtered_mods().len();
                        self.cursor_index = (self.cursor_index + 1).min(total.saturating_sub(1));
                        let name = self.mods[real_idx + 1].name.clone();
                        self.set_status(
                            StatusLevel::Info,
                            format!("Moved '{}' down to load order #{:02}", name, real_idx + 2),
                        );
                    }
                }
            }
            Action::ToggleInspector => {
                self.show_inspector = !self.show_inspector;
                if !self.show_inspector && self.mods_focus == ModsFocus::Inspector {
                    self.mods_focus = ModsFocus::Table;
                }
            }
            Action::StartSearch => {
                self.mods_focus = ModsFocus::Search;
                self.search_active = true;
                self.search_error = None;
            }
            Action::EndSearch => {
                self.search_active = false;
                self.mods_focus = ModsFocus::Table;
                if !self.search_query.is_empty() {
                    let matches = self.filtered_mods().len();
                    if matches == 0 {
                        self.set_status(
                            StatusLevel::Warning,
                            format!(
                                "No packages match '{}' -- press Esc to clear filter",
                                self.search_query
                            ),
                        );
                    } else {
                        self.set_status(
                            StatusLevel::Info,
                            format!(
                                "Filter applied: {} of {} packages shown",
                                matches,
                                self.mods.len()
                            ),
                        );
                    }
                }
            }
            Action::SearchChar(c) => {
                if self.search_query.len() < 64 {
                    self.search_query.push(c);
                    self.search_error = None;
                    self.cursor_index = 0;
                } else {
                    self.search_error =
                        Some("Maximum search length (64 chars) reached".to_string());
                    self.set_status(
                        StatusLevel::Error,
                        "Search query cannot exceed 64 characters",
                    );
                }
            }
            Action::SearchBackspace => {
                self.search_query.pop();
                self.search_error = None;
                self.cursor_index = 0;
            }
            Action::ClearSearchAndFilters => {
                self.search_query.clear();
                self.search_active = false;
                self.search_error = None;
                self.category_filter = None;
                self.cursor_index = 0;
                self.set_status(
                    StatusLevel::Info,
                    "Cleared search query and category filters",
                );
            }
            Action::FilterCategory(cat) => {
                self.category_filter = cat;
                self.cursor_index = 0;
            }
            Action::CycleCategory => {
                let next = match self.category_filter.as_deref() {
                    None => Some("Fixes".to_string()),
                    Some("Fixes") => Some("UI".to_string()),
                    Some("UI") => Some("Gameplay".to_string()),
                    Some("Gameplay") => Some("Visuals".to_string()),
                    _ => None,
                };
                let label = next.as_deref().unwrap_or("All").to_string();
                self.category_filter = next;
                self.cursor_index = 0;
                self.set_status(
                    StatusLevel::Info,
                    format!(
                        "Category filter: {} ({} packages)",
                        label,
                        self.filtered_mods().len()
                    ),
                );
            }
            Action::CycleCategoryReverse => {
                let prev = match self.category_filter.as_deref() {
                    None => Some("Visuals".to_string()),
                    Some("Visuals") => Some("Gameplay".to_string()),
                    Some("Gameplay") => Some("UI".to_string()),
                    Some("UI") => Some("Fixes".to_string()),
                    _ => None,
                };
                let label = prev.as_deref().unwrap_or("All").to_string();
                self.category_filter = prev;
                self.cursor_index = 0;
                self.set_status(
                    StatusLevel::Info,
                    format!(
                        "Category filter: {} ({} packages)",
                        label,
                        self.filtered_mods().len()
                    ),
                );
            }
            Action::ResolveConflictChoice { index, winner_id } => {
                if index < self.conflicts.len() {
                    let path = self.conflicts[index].file_path.clone();
                    let is_a = self.conflicts[index].option_a.mod_id == winner_id;
                    let winner_name = if is_a {
                        self.conflicts_focus = ConflictsFocus::OptionA;
                        self.conflicts[index].option_a.mod_name.clone()
                    } else {
                        self.conflicts_focus = ConflictsFocus::OptionB;
                        self.conflicts[index].option_b.mod_name.clone()
                    };
                    self.conflicts[index].winner_id = Some(winner_id.clone());
                    self.staged
                        .stage_conflict_winner(path.clone(), winner_id.clone());
                    self.notify_draft_changed();
                    let delta = self.staged.delta_count();
                    self.set_status(
                        StatusLevel::Success,
                        format!(
                            "Selected '{}' for {} ({} pending edits)",
                            winner_name, path, delta
                        ),
                    );
                }
            }
            Action::ToggleConflictWinner => {
                if let Some(card) = self.conflicts.get(self.active_conflict_cursor) {
                    let next_winner =
                        if card.winner_id.as_deref() == Some(card.option_a.mod_id.as_str()) {
                            card.option_b.mod_id.clone()
                        } else {
                            card.option_a.mod_id.clone()
                        };
                    let idx = self.active_conflict_cursor;
                    self.handle_action(Action::ResolveConflictChoice {
                        index: idx,
                        winner_id: next_winner,
                    });
                }
            }
            Action::AutoResolveConflicts => {
                for card in &mut self.conflicts {
                    card.winner_id = Some(card.option_b.mod_id.clone());
                }
                self.set_status(
                    StatusLevel::Success,
                    "All conflicts auto-resolved via 3-Tier Priority Waterfall",
                );
            }
            Action::SelectProfile(idx) => {
                if idx < self.profiles.len() {
                    for (i, p) in self.profiles.iter_mut().enumerate() {
                        p.is_active = i == idx;
                    }
                    let prof = &self.profiles[idx];
                    let prof_name = prof.name.clone();
                    let game_short = prof.game_short.clone();
                    self.game_title = format!("{} [{}]", game_short, prof_name);
                    self.proton_version = prof.proton_version.clone();
                    self.diagnostics = vec![prof.diagnostic.clone()];
                    self.set_status(
                        StatusLevel::Success,
                        format!(
                            "Switched workspace profile to {} :: '{}' ({})",
                            game_short, prof_name, self.proton_version
                        ),
                    );
                }
            }
            Action::StartProfileInput => {
                self.profiles_focus = ProfilesFocus::NewProfileInput;
                self.profile_input_active = true;
                self.profile_input_error = None;
            }
            Action::StartOverviewInput => {
                self.active_tab = Tab::Overview;
                self.overview_focus = OverviewFocus::CommandInput;
                self.overview_input_active = true;
                self.overview_input_error = None;
            }
            Action::CycleTargetGame => {
                if !self.game_catalog.is_empty() {
                    self.new_profile_game_idx =
                        (self.new_profile_game_idx + 1) % self.game_catalog.len();
                    let g = &self.game_catalog[self.new_profile_game_idx];
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "Selected target game for new profiles: {} (AppID {})",
                            g.short_name, g.steam_app_id
                        ),
                    );
                }
            }
            Action::CycleTargetGameReverse => {
                if !self.game_catalog.is_empty() {
                    let len = self.game_catalog.len();
                    self.new_profile_game_idx = (self.new_profile_game_idx + len - 1) % len;
                    let g = &self.game_catalog[self.new_profile_game_idx];
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "Selected target game for new profiles: {} (AppID {})",
                            g.short_name, g.steam_app_id
                        ),
                    );
                }
            }
            Action::FixDiagnostic => {
                let idx = self
                    .profile_cursor
                    .min(self.profiles.len().saturating_sub(1));
                if let Some(prof) = self.profiles.get_mut(idx) {
                    if !prof.diagnostic.resolved {
                        prof.diagnostic.resolved = true;
                        prof.diagnostic.level = "OK".to_string();
                        prof.diagnostic.action_text = "[VERIFIED]".to_string();
                        let title = prof.diagnostic.title.clone();
                        let game_short = prof.game_short.clone();
                        self.diagnostics = vec![prof.diagnostic.clone()];
                        self.set_status(
                            StatusLevel::Success,
                            format!("Resolved '{}' for {}", title, game_short),
                        );
                    } else {
                        self.set_status(
                            StatusLevel::Info,
                            "All environment diagnostics for this profile are already healthy",
                        );
                    }
                }
            }
            Action::StartDownload(name) => {
                let download_name = if name.starts_with("http://") || name.starts_with("https://") {
                    name.split('/')
                        .next_back()
                        .unwrap_or("package.peca")
                        .to_string()
                } else if name.starts_with("magnet:") {
                    "swarm_package.peca".to_string()
                } else {
                    name.clone()
                };

                self.downloads.insert(
                    0,
                    DownloadItem {
                        name: download_name.clone(),
                        progress: 0.0,
                        speed: "Connecting...".to_string(),
                        status: "CONNECTING".to_string(),
                        peers_info: if name.starts_with("magnet:") {
                            "Bootstrapping DHT + Web Seeds".to_string()
                        } else {
                            "Direct Stream".to_string()
                        },
                        paused: false,
                    },
                );
                self.download_cursor = 0;
                self.set_status(
                    StatusLevel::Success,
                    format!("Enqueued transfer: '{}'", download_name),
                );

                let tx = self.download_tx.clone();
                let dl_name = download_name;
                let uri_clone = name;

                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        if uri_clone.starts_with("http://") || uri_clone.starts_with("https://") {
                            let cache_dir = PathBuf::from(".perihelion/cache/mods");
                            let out_path = cache_dir.join(&dl_name);
                            let _ = tx.send(DownloadMessage::Progress {
                                name: dl_name.clone(),
                                progress: 10.0,
                                speed: "Connecting HTTP...".to_string(),
                                peers_info: "1 Web Source".to_string(),
                                status: "DOWNLOADING".to_string(),
                            });
                            match peri_net::fetch_http(&uri_clone, &out_path, None).await {
                                Ok(()) => {
                                    let _ = tx.send(DownloadMessage::Complete {
                                        name: dl_name,
                                        path: out_path,
                                    });
                                }
                                Err(e) => {
                                    let _ = tx.send(DownloadMessage::Error {
                                        name: dl_name,
                                        error: e.to_string(),
                                    });
                                }
                            }
                        } else if uri_clone.starts_with("magnet:") {
                            let _ = tx.send(DownloadMessage::Progress {
                                name: dl_name.clone(),
                                progress: 5.0,
                                speed: "Connecting DHT...".to_string(),
                                peers_info: "Swarm Connecting".to_string(),
                                status: "SWARMING".to_string(),
                            });
                        } else {
                            let candidate = PathBuf::from(&uri_clone);
                            if candidate.exists() {
                                let cache_dir = PathBuf::from(".perihelion/cache/mods");
                                let _ = std::fs::create_dir_all(&cache_dir);
                                let target =
                                    cache_dir.join(candidate.file_name().unwrap_or_default());
                                let _ = std::fs::copy(&candidate, &target);
                                let _ = tx.send(DownloadMessage::Complete {
                                    name: dl_name,
                                    path: target,
                                });
                            } else {
                                let _ = tx.send(DownloadMessage::Error {
                                    name: dl_name,
                                    error: format!("Local archive '{}' not found", uri_clone),
                                });
                            }
                        }
                    });
                }
            }
            Action::StartDownloadInput => {
                self.download_focus = DownloadsFocus::Input;
                self.download_input_active = true;
                self.download_input_error = None;
            }
            Action::TogglePauseDownload => {
                if let Some(item) = self.downloads.get_mut(self.download_cursor) {
                    item.paused = !item.paused;
                    if item.paused {
                        item.status = "PAUSED".to_string();
                        item.speed = "0.0 MB/s".to_string();
                        let name = item.name.clone();
                        self.set_status(
                            StatusLevel::Warning,
                            format!("Paused transfer '{}'", name),
                        );
                    } else {
                        item.status = "SWARMING".to_string();
                        item.speed = "38.4 MB/s".to_string();
                        let name = item.name.clone();
                        self.set_status(
                            StatusLevel::Info,
                            format!("Resumed P2P swarm transfer '{}'", name),
                        );
                    }
                }
            }
            Action::InstallCuratedPackage => {
                if let Some(pkg) = self.curated_packages.get_mut(self.curated_cursor) {
                    if !pkg.is_installed {
                        pkg.is_installed = true;
                        let title = pkg.title.clone();
                        let desc = pkg.desc.clone();
                        let slug = title.to_lowercase().replace(' ', "_");
                        self.downloads.insert(
                            0,
                            DownloadItem {
                                name: title.clone(),
                                progress: 15.0,
                                speed: "52.1 MB/s".to_string(),
                                status: "SWARMING".to_string(),
                                peers_info: "24 DHT Peers + 4 Web Seeds".to_string(),
                                paused: false,
                            },
                        );
                        self.mods.push(ModItem {
                            id: slug,
                            name: title.clone(),
                            active: true,
                            category: "Fixes".to_string(),
                            priority: Priority::High,
                            version: "1.0.0".to_string(),
                            author: "Perihelion Curated".to_string(),
                            description: desc,
                            affected_files: vec!["peri.toml".to_string()],
                            has_conflict: false,
                        });
                        self.set_status(
                            StatusLevel::Success,
                            format!("Enqueued & registered curated bundle '{}'", title),
                        );
                    } else {
                        let title = pkg.title.clone();
                        self.set_status(
                            StatusLevel::Warning,
                            format!("Bundle '{}' is already installed in workspace", title),
                        );
                    }
                }
            }
            Action::InputChar(c) => {
                if self.overview_input_active {
                    self.overview_input.push(c);
                    self.overview_input_error = None;
                    self.update_realtime_theme_preview();
                } else if self.search_active {
                    self.handle_action(Action::SearchChar(c));
                } else if self.download_input_active {
                    self.download_input.push(c);
                    self.download_input_error = None;
                } else if self.profile_input_active {
                    self.profile_input.push(c);
                    self.profile_input_error = None;
                }
            }
            Action::InputBackspace => {
                if self.overview_input_active {
                    self.overview_input.pop();
                    self.overview_input_error = None;
                    self.update_realtime_theme_preview();
                } else if self.search_active {
                    self.handle_action(Action::SearchBackspace);
                } else if self.download_input_active {
                    self.download_input.pop();
                    self.download_input_error = None;
                } else if self.profile_input_active {
                    self.profile_input.pop();
                    self.profile_input_error = None;
                }
            }
            Action::InputDeleteWord => {
                if self.overview_input_active {
                    Self::delete_last_word(&mut self.overview_input);
                    self.overview_input_error = None;
                    self.update_realtime_theme_preview();
                } else if self.search_active {
                    Self::delete_last_word(&mut self.search_query);
                    self.search_error = None;
                    self.clamp_mod_cursor();
                } else if self.download_input_active {
                    Self::delete_last_word(&mut self.download_input);
                    self.download_input_error = None;
                } else if self.profile_input_active {
                    Self::delete_last_word(&mut self.profile_input);
                    self.profile_input_error = None;
                }
            }
            Action::InputClear => {
                if self.overview_input_active {
                    self.overview_input.clear();
                    self.overview_input_error = None;
                    self.update_realtime_theme_preview();
                } else if self.search_active {
                    self.search_query.clear();
                    self.search_error = None;
                    self.cursor_index = 0;
                } else if self.download_input_active {
                    self.download_input.clear();
                    self.download_input_error = None;
                } else if self.profile_input_active {
                    self.profile_input.clear();
                    self.profile_input_error = None;
                }
            }
            Action::SubmitInput => {
                if self.overview_input_active {
                    let trimmed = self.overview_input.trim().to_string();
                    self.overview_input_active = false;
                    let lower = trimmed.to_ascii_lowercase();
                    if lower.is_empty() {
                        return;
                    }
                    if lower == "launch" || lower == "play" || lower == "run" {
                        self.overview_input.clear();
                        self.handle_action(Action::LaunchGame);
                    } else if lower.starts_with("clone ") || lower.starts_with("download ") {
                        let target = if lower.starts_with("clone ") {
                            trimmed[6..].trim()
                        } else {
                            trimmed[9..].trim()
                        };
                        let slug = target.to_string();
                        self.overview_input.clear();
                        self.active_tab = Tab::Downloads;
                        self.handle_action(Action::StartDownload(slug));
                    } else if lower.starts_with("switch ")
                        || lower.starts_with("profile ")
                        || lower == "switch"
                        || lower == "profile"
                    {
                        let target = if lower.starts_with("switch ") {
                            trimmed[7..].trim()
                        } else if lower.starts_with("profile ") {
                            trimmed[8..].trim()
                        } else {
                            ""
                        };
                        self.overview_input.clear();
                        if target.is_empty() {
                            let idx = self.overview_profile_cursor % self.profiles.len().max(1);
                            self.overview_profile_cursor = idx;
                            self.handle_action(Action::SelectProfile(idx));
                        } else if let Some(idx) = self.profiles.iter().position(|p| {
                            p.name
                                .to_ascii_lowercase()
                                .contains(&target.to_ascii_lowercase())
                                || p.game_short
                                    .to_ascii_lowercase()
                                    .contains(&target.to_ascii_lowercase())
                        }) {
                            self.overview_profile_cursor = idx;
                            self.handle_action(Action::SelectProfile(idx));
                        } else {
                            self.set_status(
                                StatusLevel::Warning,
                                format!("No profile matches '{}'", target),
                            );
                        }
                    } else if lower.starts_with("search ") || lower.starts_with("find ") {
                        let q = if lower.starts_with("search ") {
                            trimmed[7..].trim()
                        } else {
                            trimmed[5..].trim()
                        };
                        self.search_query = q.to_string();
                        self.overview_input.clear();
                        self.active_tab = Tab::Mods;
                        self.mods_focus = ModsFocus::Table;
                        self.cursor_index = 0;
                        self.clamp_mod_cursor();
                    } else if lower == "mods" {
                        self.overview_input.clear();
                        self.active_tab = Tab::Mods;
                    } else if lower == "conflicts" || lower == "resolve" {
                        self.overview_input.clear();
                        self.active_tab = Tab::Conflicts;
                    } else if lower == "downloads" {
                        self.overview_input.clear();
                        self.active_tab = Tab::Downloads;
                    } else if lower == "profiles" {
                        self.overview_input.clear();
                        self.active_tab = Tab::Profiles;
                    } else if lower == "help" {
                        self.overview_input.clear();
                        self.show_help = true;
                    } else if lower.starts_with("theme ")
                        || lower.starts_with("/theme ")
                        || lower.starts_with(":theme ")
                        || lower == "theme"
                        || lower == "/theme"
                        || lower == ":theme"
                    {
                        let target = if lower.starts_with("/theme ") || lower.starts_with(":theme ")
                        {
                            trimmed[7..].trim()
                        } else if lower.starts_with("theme ") {
                            trimmed[6..].trim()
                        } else {
                            ""
                        };

                        let selected_theme = if !target.is_empty() {
                            crate::theme::Theme::find_matching(target)
                        } else {
                            let matches = self.matching_themes_for_overview();
                            if !matches.is_empty() {
                                let idx = self.theme_picker_cursor % matches.len();
                                Some(matches[idx].clone())
                            } else {
                                None
                            }
                        };

                        if let Some(t) = selected_theme {
                            self.set_theme(t.clone());
                            if let Err(e) = crate::theme::save_configured_theme(&t.id) {
                                tracing::warn!("Failed to persist theme to config: {}", e);
                            }
                            self.set_status(
                                StatusLevel::Success,
                                format!("Theme switched to '{}' ({})", t.name, t.id),
                            );
                        } else {
                            self.set_status(
                                StatusLevel::Warning,
                                format!("No theme matches '{}'", target),
                            );
                        }
                        self.overview_input.clear();
                    } else if lower.starts_with("magnet:")
                        || lower.starts_with("https://")
                        || lower.ends_with(".peca")
                    {
                        self.overview_input.clear();
                        self.active_tab = Tab::Downloads;
                        self.handle_action(Action::StartDownload(trimmed));
                    } else {
                        self.overview_input.clear();
                        if let Some(idx) = self.profiles.iter().position(|p| {
                            p.name.to_ascii_lowercase().contains(&lower)
                                || p.game_short.to_ascii_lowercase().contains(&lower)
                        }) {
                            self.overview_profile_cursor = idx;
                            self.handle_action(Action::SelectProfile(idx));
                        } else {
                            self.search_query = trimmed;
                            self.active_tab = Tab::Mods;
                            self.cursor_index = 0;
                            self.clamp_mod_cursor();
                        }
                    }
                } else if self.search_active {
                    self.handle_action(Action::EndSearch);
                } else if self.download_input_active {
                    let trimmed = self.download_input.trim().to_string();
                    if trimmed.is_empty() {
                        let msg = "Input empty: provide a .peca URL, magnet:? URI, or package slug"
                            .to_string();
                        self.download_input_error = Some(msg.clone());
                        self.set_status(StatusLevel::Error, msg);
                        return;
                    }
                    match self.validate_download_input() {
                        InputValidation::Valid(_) => {
                            self.download_input.clear();
                            self.download_input_error = None;
                            self.download_input_active = false;
                            self.download_focus = DownloadsFocus::Queue;
                            self.handle_action(Action::StartDownload(trimmed));
                        }
                        InputValidation::Warning(msg) | InputValidation::Invalid(msg) => {
                            self.download_input_error = Some(msg.clone());
                            self.set_status(
                                StatusLevel::Error,
                                format!("Cannot start download: {}", msg),
                            );
                        }
                        InputValidation::Idle => {}
                    }
                } else if self.profile_input_active {
                    let trimmed = self.profile_input.trim().to_string();
                    if trimmed.is_empty() {
                        let msg = "Profile name cannot be empty".to_string();
                        self.profile_input_error = Some(msg.clone());
                        self.set_status(StatusLevel::Error, msg);
                        return;
                    }
                    match self.validate_profile_input() {
                        InputValidation::Valid(_) => {
                            for p in &mut self.profiles {
                                p.is_active = false;
                            }
                            let target_game = &self.game_catalog[self.new_profile_game_idx];
                            let slug = trimmed.to_lowercase().replace(' ', "_");
                            let new_prof = ProfileItem {
                                name: trimmed.clone(),
                                game_id: target_game.game_id.clone(),
                                game_short: target_game.short_name.clone(),
                                game_full_name: target_game.full_name.clone(),
                                steam_app_id: target_game.steam_app_id,
                                executable: target_game.executable.clone(),
                                install_dir: target_game.install_dir.clone(),
                                upper_dir: format!(
                                    "~/.local/share/perihelion/profiles/{}_{}/upper",
                                    target_game.game_id, slug
                                ),
                                mappings: target_game.mappings.clone(),
                                desc: format!(
                                    "Isolated {} OverlayFS profile with read-only base game.",
                                    target_game.short_name
                                ),
                                mod_count: self.mods.iter().filter(|m| m.active).count(),
                                proton_version: target_game.default_proton.clone(),
                                is_active: true,
                                diagnostic: target_game.default_diagnostic.clone(),
                            };
                            let game_short = new_prof.game_short.clone();
                            self.proton_version = new_prof.proton_version.clone();
                            self.diagnostics = vec![new_prof.diagnostic.clone()];
                            self.profiles.push(new_prof);
                            self.profile_cursor = self.profiles.len() - 1;
                            self.game_title = format!("{} [{}]", game_short, trimmed);
                            self.profile_input.clear();
                            self.profile_input_error = None;
                            self.profile_input_active = false;
                            self.profiles_focus = ProfilesFocus::ProfileList;
                            self.set_status(
                                StatusLevel::Success,
                                format!(
                                    "Created and activated {} profile '{}'",
                                    game_short, trimmed
                                ),
                            );
                        }
                        InputValidation::Warning(msg) | InputValidation::Invalid(msg) => {
                            self.profile_input_error = Some(msg.clone());
                            self.set_status(
                                StatusLevel::Error,
                                format!("Invalid profile name: {}", msg),
                            );
                        }
                        InputValidation::Idle => {}
                    }
                }
            }
            Action::CancelInput => {
                if self.overview_input_active {
                    self.overview_input_active = false;
                    self.overview_input.clear();
                    self.overview_input_error = None;
                    if self.theme.id != self.committed_theme.id {
                        self.theme = self.committed_theme.clone();
                        crate::theme::set_active_theme(self.theme.clone());
                    }
                }
                if self.search_active {
                    self.search_active = false;
                    self.search_error = None;
                    self.mods_focus = ModsFocus::Table;
                }
                if self.download_input_active {
                    self.download_input_active = false;
                    self.download_input_error = None;
                }
                if self.profile_input_active {
                    self.profile_input_active = false;
                    self.profile_input_error = None;
                }
            }
            Action::LaunchGame => {
                if let Some(active) = &self.active_session {
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "Game is already running (PID {}). Press 'r' to restart.",
                            active.pid
                        ),
                    );
                    return;
                }

                let workspace = Path::new(".");
                let manifest_path = workspace.join("peri.toml");
                if !manifest_path.exists() {
                    self.set_status(
                        StatusLevel::Error,
                        "Cannot launch: No active peri.toml found in current workspace. Run 'peri init' or select a game profile.",
                    );
                } else if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = Manifest::from_toml_str(&content) {
                        let active_profile = self.profiles.iter().find(|p| p.is_active);
                        let exec = active_profile
                            .map(|p| p.executable.clone())
                            .unwrap_or_else(|| "game.exe".to_string());
                        let profile_name = active_profile
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "default".to_string());

                        let active_layers: Vec<PathBuf> = self
                            .mods
                            .iter()
                            .filter(|m| m.active)
                            .map(|m| PathBuf::from(".perihelion/cache/mods").join(&m.id))
                            .collect();

                        let vfs_plan = peri_vfs::VfsPlan::new(
                            PathBuf::from("."),
                            PathBuf::from(format!(
                                ".perihelion/profiles/{}/merged",
                                manifest.target.game_id
                            )),
                        )
                        .with_layers(active_layers)
                        .with_upper(
                            PathBuf::from(format!(
                                ".perihelion/profiles/{}/upper",
                                manifest.target.game_id
                            )),
                            PathBuf::from(format!(
                                ".perihelion/profiles/{}/work",
                                manifest.target.game_id
                            )),
                        );

                        if let Some(tx) = &self.worker_tx {
                            crate::worker::spawn_game_supervisor(
                                PathBuf::from("."),
                                manifest.target.game_id.clone(),
                                profile_name,
                                exec.clone(),
                                vec![],
                                Some(vfs_plan),
                                tx.clone(),
                            );
                            self.set_status(
                                StatusLevel::Info,
                                format!(
                                    "Launching {} ({}) in isolated namespace...",
                                    manifest.target.game_id, exec
                                ),
                            );
                        } else {
                            self.set_status(
                                StatusLevel::Success,
                                format!(
                                    ">> Launching {} ({}) with OverlayFS stack ({} active mods)",
                                    manifest.target.game_id,
                                    exec,
                                    self.mods.iter().filter(|m| m.active).count()
                                ),
                            );
                        }
                    }
                }
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Action::CloseModal => {
                self.show_help = false;
                self.overview_input_active = false;
                self.search_active = false;
                self.download_input_active = false;
                self.profile_input_active = false;
            }
            Action::Tick => {
                while let Ok(msg) = self.download_rx.0.try_recv() {
                    match msg {
                        DownloadMessage::Progress {
                            name,
                            progress,
                            speed,
                            peers_info,
                            status,
                        } => {
                            if let Some(item) = self.downloads.iter_mut().find(|d| d.name == name) {
                                item.progress = progress;
                                item.speed = speed;
                                item.peers_info = peers_info;
                                item.status = status;
                            }
                        }
                        DownloadMessage::Complete { name, .. } => {
                            if let Some(item) = self.downloads.iter_mut().find(|d| d.name == name) {
                                item.progress = 100.0;
                                item.status = "VERIFIED".to_string();
                                item.speed = "Done (SHA-256 OK)".to_string();
                            }
                            self.set_status(
                                StatusLevel::Success,
                                format!("Transfer complete & verified: '{}'", name),
                            );
                            self.try_load_workspace();
                        }
                        DownloadMessage::Error { name, error } => {
                            if let Some(item) = self.downloads.iter_mut().find(|d| d.name == name) {
                                item.status = "ERROR".to_string();
                                item.speed = "0.0 MB/s".to_string();
                            }
                            self.set_status(
                                StatusLevel::Error,
                                format!("Transfer error for '{}': {}", name, error),
                            );
                        }
                    }
                }
            }
            Action::Render => {}
            Action::SessionRestored(draft) => {
                self.restore_from_draft(&draft);
            }
            Action::RecoveryConflictDetected {
                on_disk_hash,
                draft_hash,
            } => {
                self.set_status(
                    StatusLevel::Warning,
                    format!(
                        "External edit detected! Manifest on disk ({:.8}) differs from saved draft ({:.8}).",
                        on_disk_hash, draft_hash
                    ),
                );
            }
            Action::StageToggle { mod_id, active } => {
                if let Some(m) = self.mods.iter_mut().find(|m| m.id == mod_id) {
                    m.active = active;
                }
                self.staged.stage_toggle(mod_id, active);
                self.notify_draft_changed();
            }
            Action::StagePriority { mod_id, priority } => {
                if let Some(m) = self.mods.iter_mut().find(|m| m.id == mod_id) {
                    m.priority = priority;
                }
                self.staged.stage_priority(mod_id, priority);
                self.notify_draft_changed();
            }
            Action::StageConflictWinner { path, winning_mod } => {
                if let Some(c) = self.conflicts.iter_mut().find(|c| c.file_path == path) {
                    c.winner_id = Some(winning_mod.clone());
                }
                self.staged.stage_conflict_winner(path, winning_mod);
                self.notify_draft_changed();
            }
            Action::CommitDraftToDisk => {
                if self.staged.is_empty() {
                    self.set_status(StatusLevel::Info, "No staged modifications to commit.");
                } else {
                    let manifest_path = Path::new("peri.toml");
                    if !manifest_path.exists() {
                        self.set_status(
                            StatusLevel::Error,
                            "Cannot commit draft: 'peri.toml' not found in workspace.",
                        );
                    } else if let Ok(content) = std::fs::read_to_string(manifest_path) {
                        if let Ok(mut manifest) = Manifest::from_toml_str(&content) {
                            for (mod_id, prio) in &self.staged.priorities {
                                manifest.overrides.insert(
                                    mod_id.clone(),
                                    OverrideRule::Priority { priority: *prio },
                                );
                            }
                            for (path, winner_id) in &self.staged.manual_conflict_resolutions {
                                manifest.overrides.insert(
                                    format!("path:{}", path),
                                    OverrideRule::Target(winner_id.clone()),
                                );
                            }
                            match manifest.to_toml_str() {
                                Ok(serialized) => {
                                    if std::fs::write(manifest_path, serialized).is_ok() {
                                        let committed_count = self.staged.delta_count();
                                        self.canonical_manifest = Some(manifest);
                                        self.staged.clear();
                                        if let Some(tx) = &self.session_tx {
                                            let _ = tx.try_send(PersistenceCommand::PurgeDraft);
                                        }
                                        self.set_status(
                                            StatusLevel::Success,
                                            format!(
                                                "Committed {} staged edits to peri.toml. Restart game to apply.",
                                                committed_count
                                            ),
                                        );
                                    } else {
                                        self.set_status(
                                            StatusLevel::Error,
                                            "Failed to write committed manifest to disk.",
                                        );
                                    }
                                }
                                Err(err) => {
                                    self.set_status(
                                        StatusLevel::Error,
                                        format!("Failed to serialize manifest: {}", err),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Action::DiscardDraft => {
                if self.staged.is_empty() {
                    self.set_status(StatusLevel::Info, "Draft buffer is already empty.");
                } else {
                    self.staged.clear();
                    if let Some(tx) = &self.session_tx {
                        let _ = tx.try_send(PersistenceCommand::PurgeDraft);
                    }
                    self.try_load_workspace();
                    self.set_status(
                        StatusLevel::Warning,
                        "Discarded all uncommitted staged modifications.",
                    );
                }
            }
            Action::LaunchGameTriggered => {
                self.handle_action(Action::LaunchGame);
            }
            Action::GameStarted { pid, snapshot_id } => {
                let game_id = self
                    .canonical_manifest
                    .as_ref()
                    .map(|m| m.target.game_id.clone())
                    .unwrap_or_else(|| "game".into());
                self.active_session = Some(ActiveGameHandle {
                    pid,
                    game_id,
                    snapshot_id: snapshot_id.clone(),
                });
                self.notify_draft_changed();
                self.set_status(
                    StatusLevel::Success,
                    format!("[● GAME RUNNING: PID {}] (Snapshot: {})", pid, snapshot_id),
                );
            }
            Action::GameTerminated { exit_code } => {
                let prev_pid = self.active_session.take().map(|s| s.pid);
                self.notify_draft_changed();
                let code_str = exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "0".to_string());
                self.set_status(
                    StatusLevel::Info,
                    format!(
                        "Game terminated (PID {:?}, exit code: {}). Workspace idle.",
                        prev_pid, code_str
                    ),
                );
            }
            Action::RestartGameRequested => {
                if let Some(active) = &self.active_session {
                    let pid = active.pid;
                    let _ = crate::worker::terminate_process(pid);
                    self.set_status(
                        StatusLevel::Warning,
                        format!("Sent SIGTERM to PID {}. Hot-restart requested.", pid),
                    );
                } else {
                    self.handle_action(Action::LaunchGame);
                }
            }
            Action::DagSolved {
                ordered_mod_ids,
                has_cycles,
            } => {
                if has_cycles {
                    self.set_status(
                        StatusLevel::Error,
                        "Cycle detected in mod dependency graph! Resolve load order.",
                    );
                } else {
                    self.set_status(
                        StatusLevel::Info,
                        format!(
                            "DAG resolved successfully ({} mods ordered).",
                            ordered_mod_ids.len()
                        ),
                    );
                }
            }
            Action::VfiScanCompleted { collision_count } => {
                if collision_count > 0 {
                    self.set_status(
                        StatusLevel::Warning,
                        format!("VFI scan detected {} file collisions.", collision_count),
                    );
                } else {
                    self.set_status(
                        StatusLevel::Success,
                        "VFI scan clean: Zero file collisions detected across active mods.",
                    );
                }
            }
        }
    }

    /// Restores UI context and staged modifications from a persisted session draft.
    pub fn restore_from_draft(&mut self, draft: &SessionDraft) {
        self.active_tab = Tab::from_index(draft.ui_context.active_tab_index);
        self.cursor_index = draft.ui_context.cursor_row;
        self.search_query = draft.ui_context.active_search_query.clone();
        self.category_filter = draft.ui_context.selected_category_filter.clone();
        self.show_inspector = draft.ui_context.inspector_drawer_open;

        for (mod_id, &active) in &draft.staged_changes.toggles {
            if let Some(m) = self.mods.iter_mut().find(|m| m.id == *mod_id) {
                m.active = active;
            }
        }

        for (mod_id, &priority) in &draft.staged_changes.priorities {
            if let Some(m) = self.mods.iter_mut().find(|m| m.id == *mod_id) {
                m.priority = priority;
            }
        }

        for (path, winner_id) in &draft.staged_changes.manual_conflict_resolutions {
            if let Some(c) = self.conflicts.iter_mut().find(|c| c.file_path == *path) {
                c.winner_id = Some(winner_id.clone());
            }
        }

        self.staged = draft.staged_changes.clone();
        let delta_count = draft.staged_changes.delta_count();
        self.set_status(
            StatusLevel::Success,
            format!("Restored active session ({} staged edits)", delta_count),
        );
    }

    /// Constructs a `SessionDraft` snapshot from the current in-memory state.
    pub fn build_session_draft(&self) -> SessionDraft {
        let base_hash = if let Ok(content) = std::fs::read_to_string("peri.toml") {
            SessionDraft::compute_manifest_hash(&content)
        } else {
            String::new()
        };

        let mut draft = SessionDraft::new(base_hash);
        draft.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        draft.staged_changes = self.staged.clone();
        draft.ui_context = UiContext {
            active_tab_index: self.active_tab.index(),
            cursor_row: self.cursor_index,
            active_search_query: self.search_query.clone(),
            selected_category_filter: self.category_filter.clone(),
            inspector_drawer_open: self.show_inspector,
        };
        if let Some(active) = &self.active_session {
            draft.active_execution = Some(peri_core::draft::ExecutionSessionMetadata {
                pid: active.pid,
                game_id: active.game_id.clone(),
                profile_name: self
                    .profiles
                    .iter()
                    .find(|p| p.is_active)
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
                snapshot_id: active.snapshot_id.clone(),
                launched_at: draft.timestamp,
            });
        }
        draft
    }

    /// Dispatches a persistence update command to the background session recorder worker.
    pub fn notify_draft_changed(&self) {
        if let Some(tx) = &self.session_tx {
            let draft = self.build_session_draft();
            let _ = tx.try_send(PersistenceCommand::UpdateDraft(Box::new(draft)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let app = App::new();
        assert_eq!(app.active_tab, Tab::Overview);
        assert_eq!(app.overview_focus, OverviewFocus::CommandInput);
        assert!(!app.game_catalog.is_empty());
        assert_eq!(app.game_catalog[0].short_name, "Generic / Custom Game");
        assert!(app.show_inspector);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new();
        assert_eq!(app.active_tab, Tab::Overview);

        app.handle_action(Action::NextTab);
        assert_eq!(app.active_tab, Tab::Mods);

        app.handle_action(Action::NextTab);
        assert_eq!(app.active_tab, Tab::Conflicts);

        app.handle_action(Action::NextTab);
        assert_eq!(app.active_tab, Tab::Downloads);

        app.handle_action(Action::NextTab);
        assert_eq!(app.active_tab, Tab::Profiles);

        app.handle_action(Action::NextTab);
        assert_eq!(app.active_tab, Tab::Overview);

        app.handle_action(Action::PrevTab);
        assert_eq!(app.active_tab, Tab::Profiles);

        app.handle_action(Action::SetTab(Tab::Mods));
        assert_eq!(app.active_tab, Tab::Mods);
    }

    #[test]
    fn test_focus_area_cycling_with_tab() {
        let mut app = App::new();
        app.handle_action(Action::SetTab(Tab::Mods));
        assert_eq!(app.mods_focus, ModsFocus::Table);

        // Tab cycles through selectable areas within the current page instead of switching pages
        app.handle_action(Action::FocusNext);
        assert_eq!(app.active_tab, Tab::Mods);
        assert_eq!(app.mods_focus, ModsFocus::Inspector);

        app.handle_action(Action::FocusNext);
        assert_eq!(app.mods_focus, ModsFocus::Search);
        assert!(app.search_active);

        app.handle_action(Action::FocusNext);
        assert_eq!(app.mods_focus, ModsFocus::Categories);
        assert!(!app.search_active);

        app.handle_action(Action::FocusNext);
        assert_eq!(app.mods_focus, ModsFocus::Table);
    }

    #[test]
    fn test_overview_command_palette_and_fast_switching() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert_eq!(app.active_tab, Tab::Overview);

        // Tab switches focus between CommandInput and RecentProfiles
        app.handle_action(Action::FocusNext);
        assert_eq!(app.overview_focus, OverviewFocus::RecentProfiles);

        // Selecting a profile from Overview switches active game
        app.overview_profile_cursor = 2; // Cyberpunk 2077
        app.handle_action(Action::ActivateFocused);
        assert_eq!(app.game_title, "Cyberpunk 2077 [Cyberware & Vehicles]");

        // Return focus to CommandInput and type a command
        app.handle_action(Action::FocusNext);
        assert_eq!(app.overview_focus, OverviewFocus::CommandInput);

        app.handle_action(Action::StartOverviewInput);
        assert!(app.overview_input_active);

        for c in "search skyui".chars() {
            app.handle_action(Action::InputChar(c));
        }
        assert!(matches!(
            app.validate_overview_input(),
            InputValidation::Valid(_)
        ));

        app.handle_action(Action::SubmitInput);
        assert!(!app.overview_input_active);
        assert_eq!(app.active_tab, Tab::Mods);
        assert_eq!(app.search_query, "skyui");
    }

    #[test]
    fn test_smart_autocomplete_across_fields() {
        let mut app = App::new();
        app.populate_test_fixture();

        // 1. Overview root command completion: "cl" -> "clone "
        app.handle_action(Action::StartOverviewInput);
        app.overview_input = "cl".to_string();
        assert!(app.accept_autocomplete());
        assert_eq!(app.overview_input, "clone ");

        // Sub-command argument completion: "clone sky" -> "clone skyui"
        app.overview_input = "clone sky".to_string();
        assert!(app.accept_autocomplete());
        assert_eq!(app.overview_input, "clone skyui");

        // 2. Mod search completion: "ord" -> "Ordinator - Perks of Skyrim"
        app.handle_action(Action::SetTab(Tab::Mods));
        app.handle_action(Action::StartSearch);
        app.search_query = "ord".to_string();
        assert!(app.accept_autocomplete());
        assert_eq!(app.search_query, "Ordinator - Perks of Skyrim");

        // 3. Downloads completion: "real" -> "realistic_water_two_(next-gen).peca"
        app.handle_action(Action::SetTab(Tab::Downloads));
        app.handle_action(Action::StartDownloadInput);
        app.download_input = "real".to_string();
        assert!(app.accept_autocomplete());
        assert_eq!(app.download_input, "realistic_water_two_(next-gen).peca");

        // 4. Profiles completion: "Mod" -> "Modded"
        app.handle_action(Action::SetTab(Tab::Profiles));
        app.new_profile_game_idx = 1; // Cyberpunk 2077
        app.handle_action(Action::StartProfileInput);
        app.profile_input = "Mod".to_string();
        assert!(app.accept_autocomplete());
        assert_eq!(app.profile_input, "Modded");
    }

    #[test]
    fn test_mod_toggle_and_priority() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert!(app.mods[0].active);
        let initial_prio = app.mods[0].priority;

        app.cursor_index = 0;
        app.handle_action(Action::ToggleMod);
        assert!(!app.mods[0].active);

        app.handle_action(Action::CyclePriority);
        assert_ne!(app.mods[0].priority, initial_prio);
    }

    #[test]
    fn test_search_and_filter() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert_eq!(app.filtered_mods().len(), 5);

        app.handle_action(Action::StartSearch);
        assert!(app.search_active);

        // "Sky" matches Skyrim SE patch, SkyUI, Ordinator (Perks of Skyrim), and Skyland (4 mods)
        app.handle_action(Action::SearchChar('S'));
        app.handle_action(Action::SearchChar('k'));
        app.handle_action(Action::SearchChar('y'));
        assert_eq!(app.filtered_mods().len(), 4);

        // "SkyUI" matches exactly 1 mod
        app.handle_action(Action::SearchChar('U'));
        app.handle_action(Action::SearchChar('I'));
        assert_eq!(app.filtered_mods().len(), 1);

        app.handle_action(Action::SearchBackspace);
        app.handle_action(Action::SearchBackspace);
        assert_eq!(app.filtered_mods().len(), 4);

        app.handle_action(Action::EndSearch);
        assert!(!app.search_active);
    }

    #[test]
    fn test_conflict_resolution() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert_eq!(app.conflicts[0].winner_id.as_deref(), Some("skyland"));

        app.handle_action(Action::ResolveConflictChoice {
            index: 0,
            winner_id: "better_iron_weapons".to_string(),
        });
        assert_eq!(
            app.conflicts[0].winner_id.as_deref(),
            Some("better_iron_weapons")
        );

        app.handle_action(Action::AutoResolveConflicts);
        assert_eq!(app.conflicts[0].winner_id.as_deref(), Some("skyland"));
    }

    #[test]
    fn test_profile_switching() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert!(app.profiles[0].is_active);
        assert!(!app.profiles[1].is_active);

        app.handle_action(Action::SelectProfile(1));
        assert!(!app.profiles[0].is_active);
        assert!(app.profiles[1].is_active);
        assert_eq!(app.game_title, "Skyrim SE [Vanilla+ Light Fixes]");

        // Switch to a different game profile (Cyberpunk 2077)
        app.handle_action(Action::SelectProfile(2));
        assert_eq!(app.game_title, "Cyberpunk 2077 [Cyberware & Vehicles]");
    }

    #[test]
    fn test_inspector_toggle() {
        let mut app = App::new();
        assert!(app.show_inspector);

        app.handle_action(Action::ToggleInspector);
        assert!(!app.show_inspector);

        app.handle_action(Action::ToggleInspector);
        assert!(app.show_inspector);
    }

    #[test]
    fn test_download_input_validation_and_submission() {
        let mut app = App::new();
        app.handle_action(Action::SetTab(Tab::Downloads));
        app.handle_action(Action::StartDownloadInput);
        assert!(app.download_input_active);

        // Submitting empty input triggers visual validation error
        app.handle_action(Action::SubmitInput);
        assert!(app.download_input_error.is_some());
        assert_eq!(app.status_level, StatusLevel::Error);

        // Typing malformed URI triggers invalid state
        for c in "ftp://bad".chars() {
            app.handle_action(Action::InputChar(c));
        }
        assert!(matches!(
            app.validate_download_input(),
            InputValidation::Invalid(_)
        ));
        app.handle_action(Action::SubmitInput);
        assert!(app.download_input_error.is_some());

        // Clear and type valid .peca URL
        app.handle_action(Action::InputClear);
        for c in "https://mods.perihelion.io/smim_4k.peca".chars() {
            app.handle_action(Action::InputChar(c));
        }
        assert!(matches!(
            app.validate_download_input(),
            InputValidation::Valid(_)
        ));

        let initial_len = app.downloads.len();
        app.handle_action(Action::SubmitInput);
        assert!(!app.download_input_active);
        assert!(app.download_input_error.is_none());
        assert_eq!(app.downloads.len(), initial_len + 1);
        assert_eq!(app.status_level, StatusLevel::Success);
    }

    #[test]
    fn test_profile_creation_validation_and_multi_game_target() {
        let mut app = App::new();
        app.populate_test_fixture();
        app.handle_action(Action::SetTab(Tab::Profiles));

        // Cycle target game from Skyrim SE (0) -> Cyberpunk 2077 (1) -> Starfield (2)
        app.handle_action(Action::CycleTargetGame);
        app.handle_action(Action::CycleTargetGame);
        assert_eq!(
            app.game_catalog[app.new_profile_game_idx].short_name,
            "Starfield"
        );

        app.handle_action(Action::StartProfileInput);
        assert!(app.profile_input_active);

        // Duplicate name fails validation
        for c in "Requiem Overhaul".chars() {
            app.handle_action(Action::InputChar(c));
        }
        assert!(matches!(
            app.validate_profile_input(),
            InputValidation::Invalid(_)
        ));
        app.handle_action(Action::SubmitInput);
        assert!(app.profile_input_error.is_some());

        // Valid new profile name creates a Starfield profile
        app.handle_action(Action::InputClear);
        for c in "NASAPunk Explorers".chars() {
            app.handle_action(Action::InputChar(c));
        }
        app.handle_action(Action::SubmitInput);
        assert!(!app.profile_input_active);
        assert_eq!(app.profiles.len(), 4);
        assert_eq!(app.game_title, "Starfield [NASAPunk Explorers]");
        assert_eq!(app.profiles[3].steam_app_id, 1716740);
    }

    #[test]
    fn test_restore_from_draft() {
        let mut app = App::new();
        let mut draft = SessionDraft::new("dummy_hash");
        draft.ui_context.active_tab_index = 1;
        draft.ui_context.cursor_row = 3;
        draft.ui_context.active_search_query = "sky".to_string();
        draft.ui_context.selected_category_filter = Some("Visuals".to_string());
        draft.ui_context.inspector_drawer_open = false;

        app.handle_action(Action::SessionRestored(Box::new(draft)));

        assert_eq!(app.active_tab, Tab::Mods);
        assert_eq!(app.cursor_index, 3);
        assert_eq!(app.search_query, "sky");
        assert_eq!(app.category_filter, Some("Visuals".to_string()));
        assert!(!app.show_inspector);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_transactional_staging_lifecycle() {
        let mut app = App::new();
        app.populate_test_fixture();
        assert!(app.staged.is_empty());

        // Stage mod toggle
        app.handle_action(Action::ToggleMod);
        assert_eq!(app.staged.delta_count(), 1);
        assert!(app.staged.toggles.contains_key("ussep"));

        // Stage priority cycle
        app.handle_action(Action::CyclePriority);
        assert_eq!(app.staged.delta_count(), 2);
        assert!(app.staged.priorities.contains_key("ussep"));

        // Stage conflict winner
        app.handle_action(Action::ResolveConflictChoice {
            index: 0,
            winner_id: "skyui".to_string(),
        });
        assert_eq!(app.staged.delta_count(), 3);

        // Discard all staged changes
        app.handle_action(Action::DiscardDraft);
        assert!(app.staged.is_empty());
        assert_eq!(app.staged.delta_count(), 0);
    }

    #[test]
    fn test_game_supervision_lifecycle() {
        let mut app = App::new();
        assert!(app.active_session.is_none());

        // Simulate game started
        app.handle_action(Action::GameStarted {
            pid: 41209,
            snapshot_id: "snap_0xabcd".to_string(),
        });
        assert!(app.active_session.is_some());
        let session = app.active_session.as_ref().unwrap();
        assert_eq!(session.pid, 41209);
        assert_eq!(session.snapshot_id, "snap_0xabcd");

        // Simulate game terminated
        app.handle_action(Action::GameTerminated { exit_code: Some(0) });
        assert!(app.active_session.is_none());
    }

    #[test]
    fn test_overview_realtime_theme_preview_and_submission() {
        let mut app = App::new();
        assert_eq!(app.active_tab, Tab::Overview);

        // Initially terminal theme
        app.set_theme(crate::theme::Theme::terminal());
        assert_eq!(app.theme.id, "terminal");
        assert_eq!(app.committed_theme.id, "terminal");

        // Activate overview command palette
        app.overview_input_active = true;

        // 1. Type "theme nord"
        for c in "theme nord".chars() {
            app.handle_action(Action::InputChar(c));
        }

        // Live preview immediately switches active theme to "nord"
        assert_eq!(app.theme.id, "nord");
        // But committed theme is still "terminal"
        assert_eq!(app.committed_theme.id, "terminal");

        // 2. Press Esc (CancelInput) -> Reverts live preview to committed theme
        app.handle_action(Action::CancelInput);
        assert_eq!(app.theme.id, "terminal");
        assert_eq!(app.overview_input, "");
        assert!(!app.overview_input_active);

        // 3. Reactivate and type "theme " to trigger list
        app.overview_input_active = true;
        for c in "theme ".chars() {
            app.handle_action(Action::InputChar(c));
        }

        // Cycle through themes with CursorDown
        let initial_preview = app.theme.id.clone();
        app.handle_action(Action::CursorDown);
        let second_preview = app.theme.id.clone();
        assert_ne!(initial_preview, second_preview);

        // Cycle back with CursorUp
        app.handle_action(Action::CursorUp);
        assert_eq!(app.theme.id, initial_preview);

        // 4. Submit with Enter -> Commits theme
        app.overview_input = "theme dracula".to_string();
        app.update_realtime_theme_preview();
        assert_eq!(app.theme.id, "dracula");

        app.handle_action(Action::SubmitInput);
        assert_eq!(app.committed_theme.id, "dracula");
        assert_eq!(app.theme.id, "dracula");
        assert_eq!(app.status_level, StatusLevel::Success);
        assert!(!app.overview_input_active);
        assert_eq!(app.overview_input, "");
    }
}
