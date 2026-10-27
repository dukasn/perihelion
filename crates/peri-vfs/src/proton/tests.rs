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
use tempfile::tempdir;

use super::SteamDiscovery;

#[test]
fn test_discover_library_folders_vdf() {
    let tmp = tempdir().unwrap();
    let steamapps = tmp.path().join("steamapps");
    fs::create_dir_all(&steamapps).unwrap();

    let vdf_content = r#"
"libraryfolders"
{
    "0"
    {
        "path"      "/mnt/games/FastSSD"
        "label"     ""
    }
    "1"
    {
        "path"      "/mnt/storage/Secondary"
        "label"     ""
    }
}
"#;
    fs::write(steamapps.join("libraryfolders.vdf"), vdf_content).unwrap();

    let libs = SteamDiscovery::discover_library_folders(tmp.path());
    assert!(libs.contains(&tmp.path().to_path_buf()));
}

#[test]
fn test_resolve_proton_launch_with_explicit_paths() {
    let tmp = tempdir().unwrap();
    let runner = tmp.path().join("mock_proton");
    let prefix_dir = tmp.path().join("compatdata/489830");
    fs::write(&runner, b"#!/bin/sh\nexit 0").unwrap();
    fs::create_dir_all(&prefix_dir).unwrap();

    let launch =
        SteamDiscovery::resolve_proton_launch(Some(489830), Some(&runner), Some(&prefix_dir))
            .expect("should resolve with explicit paths");

    assert_eq!(launch.runner, runner);
    assert_eq!(launch.compat_data_path, prefix_dir);
    assert_eq!(launch.wine_prefix, prefix_dir.join("pfx"));

    let env_map: std::collections::HashMap<_, _> = launch.env_vars.into_iter().collect();
    assert_eq!(
        env_map.get("SteamAppId").map(|s| s.as_str()),
        Some("489830")
    );
    assert_eq!(
        env_map.get("SteamGameId").map(|s| s.as_str()),
        Some("489830")
    );
}

#[test]
fn test_scan_and_find_installed_steam_apps() {
    let tmp = tempdir().unwrap();
    let steamapps = tmp.path().join("steamapps");
    let common = steamapps.join("common/Witcher 3/bin/x64");
    fs::create_dir_all(&common).unwrap();
    fs::write(common.join("witcher3.exe"), b"").unwrap();

    let acf_game = r#"
"AppState"
{
    "appid"         "292030"
    "name"          "The Witcher 3: Wild Hunt"
    "installdir"    "Witcher 3"
}
"#;
    fs::write(steamapps.join("appmanifest_292030.acf"), acf_game).unwrap();

    let acf_tool = r#"
"AppState"
{
    "appid"         "228980"
    "name"          "Steamworks Common Redistributables"
    "installdir"    "Steamworks Shared"
}
"#;
    fs::write(steamapps.join("appmanifest_228980.acf"), acf_tool).unwrap();

    let libs = vec![tmp.path().to_path_buf()];
    let scanned = SteamDiscovery::scan_installed_apps(&libs);
    assert_eq!(scanned.len(), 2);

    let found = SteamDiscovery::find_installed_app(&libs, 292030).expect("should find Witcher 3");
    assert_eq!(found.name, "The Witcher 3: Wild Hunt");
    assert_eq!(found.installdir, "Witcher 3");
    assert_eq!(found.executable, Some("bin/x64/witcher3.exe".to_string()));

    let tool_found =
        SteamDiscovery::find_installed_app(&libs, 228980).expect("should find Steamworks tool");
    assert_eq!(tool_found.name, "Steamworks Common Redistributables");
    assert_eq!(tool_found.executable, None);

    let missing = SteamDiscovery::find_installed_app(&libs, 999999);
    assert!(missing.is_none());
}

#[test]
fn test_steam_discovery_with_custom_declarative_config() {
    let tmp = tempdir().unwrap();
    let custom_root = tmp.path().join("CustomSteam");
    let custom_lib = tmp.path().join("CustomLibrary");
    let compat_dir = tmp.path().join("CustomCompat");
    fs::create_dir_all(custom_root.join("steamapps")).unwrap();
    fs::create_dir_all(&custom_lib).unwrap();
    fs::create_dir_all(compat_dir.join("GE-Proton9-1")).unwrap();
    fs::write(compat_dir.join("GE-Proton9-1/proton"), b"").unwrap();

    let config = peri_core::ProviderConfig {
        roots: vec![custom_root.clone()],
        libraries: vec![custom_lib.clone()],
        compatibility_dirs: vec![compat_dir.clone()],
        binary_subdirs: vec!["bin/custom".to_string()],
        binary_extensions: vec!["exe".to_string()],
        environment: vec!["CUSTOM_GAME_FLAG".to_string()],
    };

    let roots = SteamDiscovery::known_steam_roots_with_config(&config);
    assert!(roots.contains(&custom_root));

    let active = SteamDiscovery::find_active_steam_root_with_config(&config);
    assert_eq!(active, Some(custom_root.clone()));

    let libs = SteamDiscovery::discover_library_folders_with_config(&custom_root, &config);
    assert!(libs.contains(&custom_root));
    assert!(libs.contains(&custom_lib));

    let protons =
        SteamDiscovery::discover_proton_versions_with_config(&custom_root, &libs, &config);
    assert_eq!(protons.len(), 1);
    assert_eq!(protons[0].name, "GE-Proton9-1");

    // App scanning with custom config
    let acf = r#"
"AppState"
{
    "appid"         "777777"
    "name"          "Custom Game"
    "installdir"    "CustomGame"
}
"#;
    let custom_steamapps = custom_lib.join("steamapps");
    fs::create_dir_all(&custom_steamapps).unwrap();
    fs::write(custom_steamapps.join("appmanifest_777777.acf"), acf).unwrap();
    let custom_bin = custom_lib.join("steamapps/common/CustomGame/bin/custom");
    fs::create_dir_all(&custom_bin).unwrap();
    fs::write(custom_bin.join("game.exe"), b"").unwrap();

    let apps = SteamDiscovery::scan_installed_apps_with_config(&libs, &config);
    let app = apps
        .iter()
        .find(|a| a.app_id == 777777)
        .expect("custom app found");
    assert_eq!(app.executable, Some("bin/custom/game.exe".to_string()));
}
