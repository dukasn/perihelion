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

use peri_core::profile::GameProfile;
use peri_vfs::SteamDiscovery;

use super::actions::{handle_new, parse_mapping};
use super::resolution::resolve_game_profile;
use crate::cli::ProfileNewArgs;
use crate::PROFILES_DIR;

#[test]
fn test_parse_mapping_colon() {
    let mapping = parse_mapping("mods:archive/pc/mod").unwrap();
    assert_eq!(mapping.from_prefix, "mods");
    assert_eq!(mapping.to_prefix, "archive/pc/mod");
}

#[test]
fn test_parse_mapping_arrow() {
    let mapping = parse_mapping("SKSE -> Data/SKSE").unwrap();
    assert_eq!(mapping.from_prefix, "SKSE");
    assert_eq!(mapping.to_prefix, "Data/SKSE");
}

#[test]
fn test_parse_mapping_invalid() {
    assert!(parse_mapping("invalid_spec").is_err());
    assert!(parse_mapping(":target").is_err());
    assert!(parse_mapping("source:").is_err());
}

#[test]
fn test_find_executable_candidate() {
    let tmp = tempfile::tempdir().unwrap();
    let game_dir = tmp.path();
    fs::create_dir_all(game_dir.join("bin/x64")).unwrap();
    fs::write(game_dir.join("bin/x64/game.exe"), b"").unwrap();

    let config = peri_core::ProviderConfig {
        binary_subdirs: vec!["bin/x64".to_string()],
        binary_extensions: vec!["exe".to_string()],
        ..Default::default()
    };
    let candidate = SteamDiscovery::find_executable_candidate_with_config(game_dir, &config);
    assert_eq!(candidate, Some("bin/x64/game.exe".to_string()));
}

#[tokio::test]
async fn test_handle_new_creates_profile_file() {
    let cur = std::env::current_dir().unwrap();
    let tmp = tempfile::Builder::new()
        .prefix(".tmp_peri_test_")
        .tempdir_in(&cur)
        .unwrap();
    let ws = tmp.path();

    let args = ProfileNewArgs {
        game_id: "elden_ring".to_string(),
        name: Some("Elden Ring".to_string()),
        steam_id: Some(1245620),
        game_path: None,
        exec: Some("Game/eldenring.exe".to_string()),
        mappings: vec!["mods:Game/mods".to_string()],
        global: false,
        from_steam: None,
        workspace: ws.to_path_buf(),
        force: false,
    };

    handle_new(args).await.unwrap();

    let profile_file = ws.join(PROFILES_DIR).join("elden_ring.toml");
    assert!(profile_file.exists());

    let content = fs::read_to_string(profile_file).unwrap();
    let parsed = GameProfile::from_toml_str(&content).unwrap();
    assert_eq!(parsed.id, "elden_ring");
    assert_eq!(parsed.name, "Elden Ring");
    assert_eq!(parsed.steam_app_id, Some(1245620));
    assert_eq!(
        parsed.default_executable.as_deref(),
        Some("Game/eldenring.exe")
    );
    assert_eq!(parsed.mappings.len(), 1);
    assert_eq!(parsed.mappings[0].from_prefix, "mods");
    assert_eq!(parsed.mappings[0].to_prefix, "Game/mods");
}

#[tokio::test]
async fn test_handle_new_auto_detects_engine_layout() {
    let cur = std::env::current_dir().unwrap();
    let tmp = tempfile::Builder::new()
        .prefix(".tmp_peri_test_")
        .tempdir_in(&cur)
        .unwrap();
    let ws = tmp.path();

    // Create mock Bethesda game install
    let mock_game = ws.join("mock_starfield");
    fs::create_dir_all(mock_game.join("Data")).unwrap();
    fs::write(mock_game.join("Data/Starfield.esm"), b"").unwrap();
    fs::write(mock_game.join("Starfield.exe"), b"").unwrap();

    // Supply engine signature recipe via workspace tier
    let sigs_dir = ws.join(".perihelion/signatures");
    fs::create_dir_all(&sigs_dir).unwrap();
    let sig_toml = r#"
id = "bethesda_creation"
name = "Bethesda Creation Engine"
priority = 100
anchors = ["Data"]
markers = ["Data/*.esm", "Data/*.esp", "*_loader.exe"]
binary_subdirs = [".", "bin"]
binary_extensions = ["exe"]
mappings = [
    { from = "Data", to = "Data" },
    { from = "SKSE", to = "Data/SKSE" },
]
"#;
    fs::write(sigs_dir.join("bethesda_creation.toml"), sig_toml).unwrap();

    let args = ProfileNewArgs {
        game_id: "starfield".to_string(),
        name: None,
        steam_id: None,
        game_path: Some(mock_game),
        exec: None,
        mappings: vec![],
        global: false,
        from_steam: None,
        workspace: ws.to_path_buf(),
        force: false,
    };

    handle_new(args).await.unwrap();

    let profile_file = ws.join(PROFILES_DIR).join("starfield.toml");
    assert!(profile_file.exists());

    let content = fs::read_to_string(profile_file).unwrap();
    let parsed = GameProfile::from_toml_str(&content).unwrap();

    // Engine layout auto-detected: mappings populated from bethesda_creation signature
    assert!(parsed.mappings.iter().any(|m| m.from_prefix == "SKSE"));
    assert_eq!(parsed.default_executable.as_deref(), Some("Starfield.exe"));
}

#[test]
fn test_resolve_game_profile_cascading() {
    let cur = std::env::current_dir().unwrap();
    let tmp = tempfile::Builder::new()
        .prefix(".tmp_peri_test_")
        .tempdir_in(&cur)
        .unwrap();
    let ws = tmp.path();

    let profiles_dir = ws.join(PROFILES_DIR);
    fs::create_dir_all(&profiles_dir).unwrap();
    let custom_profile = r#"
id = "custom_rpg"
name = "Custom RPG"
default_executable = "rpg.exe"

[[mappings]]
from_prefix = "Data"
to_prefix = "Data"
"#;
    fs::write(profiles_dir.join("custom_rpg.toml"), custom_profile).unwrap();

    let resolved = resolve_game_profile(ws, "custom_rpg");
    assert_eq!(resolved.id, "custom_rpg");
    assert_eq!(resolved.name, "Custom RPG");
    assert_eq!(resolved.default_executable.as_deref(), Some("rpg.exe"));

    // Fallback to generic
    let fallback = resolve_game_profile(ws, "non_existent_game");
    assert_eq!(fallback.id, "non_existent_game");
    assert_eq!(fallback.name, "non_existent_game");
}
