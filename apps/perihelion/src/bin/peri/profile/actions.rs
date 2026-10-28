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
use std::path::PathBuf;
use tracing::info_span;

use peri_core::profile::{GameProfile, LayoutMapping};
use peri_core::report::ProfileSummaryReport;
use peri_vfs::{detect_engine_layout, find_executable_for_engine, SteamDiscovery};

use crate::cli::{
    CiFlags, CliFormat, ProfileAction, ProfileArgs, ProfileListArgs, ProfileNewArgs,
    ProfileShowArgs,
};
use crate::error::CliError;
use crate::profile::resolution::{find_profile_with_source, scan_all_profiles, ProfileSource};
use crate::{validate_workspace_path, PROFILES_DIR};

pub async fn handle_profile(args: ProfileArgs, ci_flags: &CiFlags) -> Result<(), CliError> {
    match args.action {
        ProfileAction::New(new_args) => handle_new(new_args).await,
        ProfileAction::List(list_args) => handle_list(list_args, ci_flags).await,
        ProfileAction::Show(show_args) => handle_show(show_args).await,
    }
}

pub async fn handle_new(args: ProfileNewArgs) -> Result<(), CliError> {
    let span = info_span!("profile_new", game_id = %args.game_id);
    let _enter = span.enter();

    let config = crate::config::load_global_config_with_workspace(Some(&args.workspace))?;
    let provider = config.provider("steam");

    // 1. Steam Detection
    let steam_app_id = args.steam_id.or(args.from_steam);
    let mut detected_title = None;
    let mut detected_exec = None;
    let mut detected_install_dir = args.game_path;

    if let Some(app_id) = steam_app_id {
        let steam_roots = SteamDiscovery::known_steam_roots_with_config(&provider);
        let mut libraries = Vec::new();
        for root in &steam_roots {
            libraries.extend(SteamDiscovery::discover_library_folders_with_config(
                root, &provider,
            ));
        }
        if let Some(app) =
            SteamDiscovery::find_installed_app_with_config(&libraries, app_id, &provider)
        {
            detected_title = Some(app.name);
            detected_exec = app.executable;
            if detected_install_dir.is_none() {
                detected_install_dir = Some(app.install_path);
            }
        }
    }

    // 2. Engine Layout Discovery via Signatures
    let mut detected_engine_name = None;
    let mut engine_mappings = Vec::new();

    if let Some(ref install_dir) = detected_install_dir {
        if let Some(engine) = detect_engine_layout(install_dir, config.signatures()) {
            detected_engine_name = Some(format!("{} ({})", engine.name, engine.id));
            engine_mappings = engine.mappings.clone();

            if detected_exec.is_none() {
                detected_exec = find_executable_for_engine(install_dir, &engine);
            }
        }
    }

    let name = args.name.or(detected_title).unwrap_or_else(|| {
        args.game_id
            .replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    });

    let default_executable = args.exec.or(detected_exec);

    let mut mappings = Vec::new();
    for spec in &args.mappings {
        mappings.push(parse_mapping(spec)?);
    }

    // If no explicit mappings were passed on CLI:
    // 1. Prefer engine layout discovered from directory signatures
    // 2. Fallback to declarative mappings from config LUT / target ID
    if mappings.is_empty() {
        if !engine_mappings.is_empty() {
            mappings = engine_mappings;
        } else {
            mappings = config.resolve_mappings(steam_app_id, Some(&args.game_id));
        }
    }

    let profile = GameProfile {
        id: args.game_id.clone(),
        name: name.clone(),
        steam_app_id,
        default_executable: default_executable.clone(),
        game_path: detected_install_dir.clone(),
        mappings: mappings.clone(),
    };

    // 3. Target Directory Resolution
    let target_dir = if args.global {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config).join("perihelion/profiles")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config/perihelion/profiles")
        } else {
            return Err(CliError::ProfileCommand(
                "Could not determine user config directory ($HOME or $XDG_CONFIG_HOME not set)"
                    .into(),
            ));
        }
    } else {
        let workspace_path = validate_workspace_path(&args.workspace)?;
        workspace_path.join(PROFILES_DIR)
    };

    fs::create_dir_all(&target_dir).map_err(|e| CliError::Workspace(e.to_string()))?;
    let target_file = target_dir.join(format!("{}.toml", args.game_id));

    if target_file.exists() && !args.force {
        return Err(CliError::ProfileCommand(format!(
            "Profile definition '{}' already exists. Use --force to overwrite.",
            target_file.display()
        )));
    }

    let toml_str = profile.to_toml_str()?;
    fs::write(&target_file, toml_str).map_err(|e| CliError::Workspace(e.to_string()))?;

    println!("✨ Created game profile '{}'", args.game_id);
    println!("   Location:    {}", target_file.display());
    println!("   Display:     {}", name);
    if let Some(id) = steam_app_id {
        println!("   Steam AppID: {}", id);
    }
    if let Some(ref exe) = default_executable {
        println!("   Executable:  {}", exe);
    }
    if let Some(ref inst) = detected_install_dir {
        println!("   Install Dir: {}", inst.display());
    }
    if let Some(ref eng) = detected_engine_name {
        println!("   Engine:      {}", eng);
    }
    if mappings.is_empty() {
        println!("   VFS Rules:   Direct 1:1 overlay (no prefix translation)");
    } else {
        println!("   VFS Rules:   {} prefix mapping(s):", mappings.len());
        for m in &mappings {
            println!("     • {}/*  -->  {}/*", m.from_prefix, m.to_prefix);
        }
    }

    Ok(())
}

pub async fn handle_list(args: ProfileListArgs, ci_flags: &CiFlags) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let span = info_span!("profile_list", workspace = %workspace.display());
    let _enter = span.enter();

    let config = crate::config::load_global_config_with_workspace(Some(&workspace))?;
    let provider = config.provider("steam");
    let steam_roots = SteamDiscovery::known_steam_roots_with_config(&provider);
    let mut steam_libs = Vec::new();
    for root in &steam_roots {
        steam_libs.extend(SteamDiscovery::discover_library_folders_with_config(
            root, &provider,
        ));
    }

    let discovered = scan_all_profiles(&workspace, &steam_libs);

    if ci_flags.format == CliFormat::Json {
        let reports: Vec<ProfileSummaryReport> = discovered
            .into_iter()
            .map(|item| ProfileSummaryReport {
                id: item.profile.id,
                name: item.profile.name,
                steam_app_id: item.profile.steam_app_id,
                default_executable: item.profile.default_executable,
                game_path: item.profile.game_path,
                source_tier: item.source.as_str().to_string(),
            })
            .collect();
        let json_str = serde_json::to_string_pretty(&reports)
            .map_err(|e| CliError::Workspace(e.to_string()))?;
        println!("{json_str}");
        return Ok(());
    }

    println!(
        "{:<18} {:<32} {:<10} {:<24} {:<8} {:<12}",
        "GAME ID", "NAME", "STEAM ID", "EXECUTABLE", "RULES", "SOURCE"
    );
    println!("{}", "─".repeat(110));

    for item in &discovered {
        let steam_str = item
            .profile
            .steam_app_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".to_string());
        let exe_str = item
            .profile
            .default_executable
            .as_deref()
            .unwrap_or("game.exe");
        let rules_str = item.profile.mappings.len().to_string();

        let installed_marker = if item.is_installed && item.source != ProfileSource::Generic {
            " [Installed]"
        } else {
            ""
        };

        println!(
            "{:<18} {:<32} {:<10} {:<24} {:<8} {}{}",
            item.profile.id,
            truncate_str(&item.profile.name, 30),
            steam_str,
            truncate_str(exe_str, 22),
            rules_str,
            item.source,
            installed_marker
        );
    }

    Ok(())
}

pub async fn handle_show(args: ProfileShowArgs) -> Result<(), CliError> {
    let workspace = validate_workspace_path(&args.workspace)?;
    let (profile, file_path, source) = find_profile_with_source(&workspace, &args.game_id);

    if args.raw {
        let toml_str = profile.to_toml_str()?;
        print!("{}", toml_str);
        return Ok(());
    }

    println!("Game Profile: {}", profile.id);
    println!("  Display Name:       {}", profile.name);
    println!("  Source:             {}", source);
    if let Some(ref p) = file_path {
        println!("  File Path:          {}", p.display());
    }
    println!(
        "  Steam AppID:        {}",
        profile
            .steam_app_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "None / Unset".to_string())
    );
    println!(
        "  Default Executable: {}",
        profile.default_executable.as_deref().unwrap_or("game.exe")
    );
    if let Some(ref gp) = profile.game_path {
        println!("  Game Path:          {}", gp.display());
    }

    if profile.mappings.is_empty() {
        println!("  VFS Layout Rules:   None (direct 1:1 overlay without prefix translation)");
    } else {
        println!(
            "  VFS Layout Rules:   ({} prefix rule{})",
            profile.mappings.len(),
            if profile.mappings.len() == 1 { "" } else { "s" }
        );
        for m in &profile.mappings {
            println!(
                "    • {:<16} --> {:<16}",
                format!("{}/*", m.from_prefix),
                format!("{}/*", m.to_prefix)
            );
        }
    }

    Ok(())
}

pub fn parse_mapping(spec: &str) -> Result<LayoutMapping, CliError> {
    let parts: Vec<&str> = if spec.contains(':') {
        spec.split(':').collect()
    } else if spec.contains("->") {
        spec.split("->").collect()
    } else if spec.contains('=') {
        spec.split('=').collect()
    } else {
        return Err(CliError::ProfileCommand(format!(
            "Invalid mapping format '{}'. Expected 'from_prefix:to_prefix' (e.g. 'mods:archive/pc/mod').",
            spec
        )));
    };

    if parts.len() != 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return Err(CliError::ProfileCommand(format!(
            "Invalid mapping format '{}'. Both source and target prefixes must be non-empty.",
            spec
        )));
    }

    Ok(LayoutMapping {
        from_prefix: parts[0].trim().trim_matches('/').to_string(),
        to_prefix: parts[1].trim().trim_matches('/').to_string(),
    })
}

pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
