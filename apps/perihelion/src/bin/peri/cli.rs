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

use clap::{Args, Parser, Subcommand, ValueHint};
use helion::cli_styles;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CliFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Copy)]
pub struct CiFlags {
    pub locked: bool,
    pub offline: bool,
    pub format: CliFormat,
}

impl CiFlags {
    pub fn from_cli(cli: &Cli) -> Self {
        let locked = cli.locked || cli.frozen;
        let offline = cli.offline || cli.frozen;
        Self {
            locked,
            offline,
            format: cli.format,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "peri",
    author = "Eduardo Marques Santos and Perihelion Contributors",
    version,
    about = "Next-generation, game-agnostic native Linux mod manager and VFS orchestrator",
    long_about = "peri - Game-agnostic native Linux mod manager and VFS orchestrator.\n\n\
                  Manages mod lists declaratively, resolves dependency graphs, and runs\n\
                  games inside isolated kernel OverlayFS namespaces without root permissions.",
    styles = cli_styles(),
    color = clap::ColorChoice::Auto,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Enable verbose debug and tracing logs
    #[arg(short, long, global = true, help_heading = "Logging")]
    pub verbose: bool,

    /// Disallow lockfile updates; fail if peri.lock is missing or out of sync
    #[arg(long, global = true, help_heading = "Automation")]
    pub locked: bool,

    /// Disallow any network requests; rely strictly on local cache
    #[arg(long, global = true, help_heading = "Automation")]
    pub offline: bool,

    /// Equivalent to --locked --offline
    #[arg(long, global = true, help_heading = "Automation")]
    pub frozen: bool,

    /// Select stdout formatting (text, json)
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "text",
        help_heading = "Automation"
    )]
    pub format: CliFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a workspace mapped to a target game profile
    #[command(
        about = "Initialize a workspace mapped to a target game profile",
        after_help = "EXAMPLES:\n  \
                      peri init cyberpunk_2077\n  \
                      peri init starfield -p ~/Games/Starfield --name \"Constellation Pack\""
    )]
    Init(InitArgs),

    /// Fetch a mod archive from HTTP or BitTorrent and add it to cache
    #[command(
        about = "Fetch a mod archive from HTTP or BitTorrent and add it to cache",
        after_help = "EXAMPLES:\n  \
                      peri get https://mods.example.com/cyber_tweaks.peca\n  \
                      peri get \"magnet:?xt=urn:btih:e3b0c44298fc...\""
    )]
    Get(GetArgs),

    /// Fetch a mod archive and declare it as a dependency in peri.toml
    #[command(
        about = "Fetch a mod archive and declare it as a dependency in peri.toml",
        after_help = "EXAMPLES:\n  \
                      peri add ./dist/skse64-2.2.6.peca\n  \
                      peri add https://mods.example.com/skyui-5.2.0.peca --req \"^5.2\""
    )]
    Add(AddArgs),

    /// Remove a dependency from peri.toml
    #[command(
        about = "Remove a dependency from peri.toml",
        after_help = "EXAMPLES:\n  \
                      peri remove skyui"
    )]
    Remove(RemoveArgs),

    /// Solve dependencies, verify checksums, and unpack missing layers
    #[command(
        about = "Solve dependencies, verify checksums, and unpack missing layers",
        after_help = "EXAMPLES:\n  \
                      peri sync\n  \
                      peri sync -w ~/Games/Cyberpunk"
    )]
    Sync(SyncArgs),

    /// Pack a mod folder containing peri.toml into a validated .peca archive
    #[command(
        about = "Pack a mod folder containing peri.toml into a validated .peca archive",
        after_help = "EXAMPLES:\n  \
                      peri package -s ./my-mod\n  \
                      peri package -s ./my-mod -o ./dist/mod-1.0.0.peca --level 19"
    )]
    Package(PackageArgs),

    /// Scan workspace manifests for dependency cycles, missing files, and collisions
    #[command(
        about = "Scan workspace manifests for dependency cycles, missing files, and collisions",
        after_help = "EXAMPLES:\n  \
                      peri check\n  \
                      peri check -w ~/Games/Starfield"
    )]
    Check(CheckArgs),

    /// Mount the OverlayFS stack inside an unprivileged namespace and execute the game
    #[command(
        about = "Mount the OverlayFS stack inside an unprivileged namespace and execute the game",
        after_help = "EXAMPLES:\n  \
                      peri run\n  \
                      peri run --dry-run\n  \
                      peri run --exec ./bin/x64/Cyberpunk2077.exe\n  \
                      peri run -- --skip-intro +exec autoexec.cfg"
    )]
    Run(RunArgs),

    /// Inspect or initialize global configuration (~/.config/perihelion/config.toml)
    #[command(
        about = "Inspect or initialize global configuration (~/.config/perihelion/config.toml)",
        after_help = "EXAMPLES:\n  \
                      peri config path\n  \
                      peri config show\n  \
                      peri config init"
    )]
    Config(ConfigArgs),

    /// Manage declarative game profiles and VFS layout translations
    #[command(
        about = "Manage declarative game profiles and VFS layout translations",
        after_help = "EXAMPLES:\n  \
                      peri profile new cyberpunk_2077 --name \"Cyberpunk 2077\" --exec bin/x64/Cyberpunk2077.exe --map mods:archive/pc/mod\n  \
                      peri profile new elden_ring --from-steam 1245620 --global\n  \
                      peri profile list\n  \
                      peri profile show cyberpunk_2077"
    )]
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Target game identifier (e.g., cyberpunk_2077, starfield, generic)
    #[arg(value_name = "GAME_ID")]
    pub game_id: String,

    /// Target workspace directory to initialize
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub path: PathBuf,

    /// Custom name for this modpack workspace
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// URI (HTTP URL, Magnet link, or local path) to fetch
    #[arg(value_name = "URI", value_hint = ValueHint::AnyPath)]
    pub uri: String,

    /// Explicit target filename in the package cache
    #[arg(
        short,
        long,
        value_name = "FILE",
        value_hint = ValueHint::FilePath
    )]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// URI (HTTP URL, Magnet link, or local path) of the .peca archive
    #[arg(value_name = "URI", value_hint = ValueHint::AnyPath)]
    pub uri: String,

    /// Version requirement to record (defaults to ^<archive version>)
    #[arg(long, value_name = "REQ")]
    pub req: Option<String>,

    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Name of the dependency to remove
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct PackageArgs {
    /// Path to mod directory containing root peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub source: PathBuf,

    /// Destination path for the generated .peca archive
    #[arg(
        short,
        long,
        value_name = "FILE",
        value_hint = ValueHint::FilePath
    )]
    pub output: Option<PathBuf>,

    /// Zstandard compression level (1-22)
    #[arg(short, long, default_value = "3", value_name = "LEVEL")]
    pub level: i32,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,

    /// Base game installation directory to mount as bottom lowerdir layer (overrides PERI_BASE_GAME and profile settings)
    #[arg(
        long = "base-path",
        alias = "game-path",
        env = "PERI_BASE_GAME",
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "Execution Options"
    )]
    pub base_path: Option<PathBuf>,

    /// Simulate OverlayFS mount arguments without spawning kernel namespaces
    #[arg(long, help_heading = "Execution Options")]
    pub dry_run: bool,

    /// Override the target executable command
    #[arg(long, value_name = "COMMAND", help_heading = "Execution Options")]
    pub exec: Option<String>,

    /// Prefix wrapper command(s) (e.g. gamemoderun, mangohud, "gamescope -w 1920 -h 1080 --")
    #[arg(
        long = "wrapper",
        value_name = "CMD",
        action = clap::ArgAction::Append,
        help_heading = "Execution Options"
    )]
    pub wrappers: Vec<String>,

    /// Run through GameMode (gamemoderun) optimization wrapper
    #[arg(long, help_heading = "Execution Options")]
    pub gamemode: bool,

    /// Run through gamescope micro-compositor with optional arguments (e.g. --gamescope or --gamescope="-w 1920 -h 1080 -f")
    #[arg(
        long,
        value_name = "OPTS",
        num_args = 0..=1,
        default_missing_value = "",
        help_heading = "Execution Options"
    )]
    pub gamescope: Option<String>,

    /// Enable MangoHud performance overlay
    #[arg(long, help_heading = "Execution Options")]
    pub mangohud: bool,

    /// Explicit path to Proton runner binary or installation directory
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        help_heading = "Environment Options"
    )]
    pub proton: Option<PathBuf>,

    /// Execute via system Wine runner (optionally specify custom wine binary path)
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        default_missing_value = "wine",
        help_heading = "Environment Options"
    )]
    pub wine: Option<PathBuf>,

    /// Explicit path to Wine prefix or compatdata directory
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        help_heading = "Environment Options"
    )]
    pub prefix: Option<PathBuf>,

    /// Force native Linux execution without Proton compatibility runners
    #[arg(long, help_heading = "Environment Options")]
    pub native: bool,

    /// Trailing arguments forwarded directly to the game binary
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: Option<ConfigAction>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Display the path to global config.toml
    #[command(about = "Display the path to global config.toml")]
    Path,

    /// Display the active global configuration values
    #[command(about = "Display the active global configuration values")]
    Show,

    /// Initialize default ~/.config/perihelion/config.toml
    #[command(about = "Initialize default ~/.config/perihelion/config.toml")]
    Init,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Debug, Subcommand)]
pub enum ProfileAction {
    /// Create or scaffold a new game profile TOML definition
    #[command(
        about = "Create or scaffold a new game profile TOML definition",
        after_help = "EXAMPLES:\n  \
                      peri profile new cyberpunk_2077 --name \"Cyberpunk 2077\" --exec bin/x64/Cyberpunk2077.exe --map mods:archive/pc/mod\n  \
                      peri profile new elden_ring --steam-id 1245620 --global\n  \
                      peri profile new skyrim_se --from-steam 489830"
    )]
    New(ProfileNewArgs),

    /// List all discovered game profiles across workspace, user, and system paths
    #[command(
        about = "List all discovered game profiles across workspace, user, and system paths"
    )]
    List(ProfileListArgs),

    /// Inspect a specific game profile and its VFS layout prefix mappings
    #[command(
        about = "Inspect a specific game profile and its VFS layout prefix mappings",
        after_help = "EXAMPLES:\n  \
                      peri profile show cyberpunk_2077\n  \
                      peri profile show generic --raw"
    )]
    Show(ProfileShowArgs),
}

#[derive(Debug, Args)]
pub struct ProfileNewArgs {
    /// Target game identifier (e.g., cyberpunk_2077, elden_ring)
    #[arg(value_name = "GAME_ID")]
    pub game_id: String,

    /// Human-readable display name for the game
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,

    /// Steam Application ID for Proton and Steam library integration
    #[arg(long, value_name = "APP_ID")]
    pub steam_id: Option<u64>,

    /// Installation path of the game to inspect and auto-detect engine layout
    #[arg(
        long = "game-path",
        alias = "base-path",
        env = "PERI_BASE_GAME",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub game_path: Option<PathBuf>,

    /// Default executable binary path relative to game root (e.g., bin/game.exe)
    #[arg(short, long, value_name = "PATH")]
    pub exec: Option<String>,

    /// VFS layout prefix mapping in 'from_prefix:to_prefix' format (repeatable)
    #[arg(short, long = "map", value_name = "FROM:TO")]
    pub mappings: Vec<String>,

    /// Save globally to ~/.config/perihelion/profiles/ instead of workspace
    #[arg(short, long)]
    pub global: bool,

    /// Auto-detect game title and executable from local Steam library using the Steam App ID
    #[arg(long, value_name = "APP_ID")]
    pub from_steam: Option<u64>,

    /// Target workspace directory (used when saving locally; defaults to ".")
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,

    /// Overwrite existing profile if it already exists
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ProfileListArgs {
    /// Target workspace directory (defaults to ".")
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct ProfileShowArgs {
    /// Target game identifier to inspect
    #[arg(value_name = "GAME_ID")]
    pub game_id: String,

    /// Target workspace directory (defaults to ".")
    #[arg(
        short,
        long,
        default_value = ".",
        value_name = "DIR",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: PathBuf,

    /// Output raw TOML instead of formatted summary
    #[arg(long)]
    pub raw: bool,
}
