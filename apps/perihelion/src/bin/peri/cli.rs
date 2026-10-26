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

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "peri",
    author = "Eduardo Marques Santos and Perihelion Contributors",
    version,
    about = "Mod Fearlessly. Share Instantly.",
    long_about = "Mod Fearlessly. Share Instantly.\n\n\
                  No 40-step installation guides. No throttled 2 MB/s download caps.\n\
                  Perihelion turns entire modded setups into lightweight, shareable\n\
                  blueprints you can sync and play in one command.\n\n\
                  ENGINEERED DIFFERENTLY (UNDER THE HOOD):\n\
                  • Kernel OverlayFS: In-memory file composition with zero disk duplication.\n\
                  • Unprivileged Namespaces: Complete sandbox isolation without root / sudo.\n\
                  • P2P Swarm Transport: Uncapped multi-gigabyte BitTorrent .peca streaming.\n\
                  • Deterministic DAG Solver: Conflict-free 3-tier waterfall resolution."
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose debug logs")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initializes a Perihelion workspace mapped to a specific game profile")]
    Init(InitArgs),

    #[command(about = "Fetches a mod/modpack, parses its manifest, and adds it to the VFS cache")]
    Get(GetArgs),

    #[command(about = "Downloads, unpacks, and validates the entire dependency tree")]
    Sync(SyncArgs),

    #[command(about = "Compiles a local mod directory into a validated .peca archive")]
    Package(PackageArgs),

    #[command(about = "Runs global dependency DAG checks and validates file integrity")]
    Check(CheckArgs),

    #[command(about = "Spawns the namespace, constructs the VFS layer, and executes the game")]
    Run(RunArgs),

    #[command(
        about = "Inspects or initializes global configuration (~/.config/perihelion/config.toml)"
    )]
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(help = "The target game identifier (e.g., cyberpunk_2077, starfield, generic)")]
    pub game_id: String,

    #[arg(short, long, default_value = ".", help = "Target workspace directory")]
    pub path: PathBuf,

    #[arg(long, help = "Custom name for this modpack / workspace")]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    #[arg(help = "The URI (HTTP URL, Magnet link, or local path) to fetch")]
    pub uri: String,

    #[arg(short, long, help = "Explicit target filename in the cache")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[arg(short, long, default_value = ".", help = "Path to the workspace root")]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct PackageArgs {
    #[arg(
        short,
        long,
        default_value = ".",
        help = "Path to mod folder containing peri.toml"
    )]
    pub source: PathBuf,

    #[arg(short, long, help = "Output .peca archive path")]
    pub output: Option<PathBuf>,

    #[arg(
        short,
        long,
        default_value = "3",
        help = "Zstandard compression level (1-22)"
    )]
    pub level: i32,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(short, long, default_value = ".", help = "Path to the workspace root")]
    pub workspace: PathBuf,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(short, long, default_value = ".", help = "Path to the workspace root")]
    pub workspace: PathBuf,

    #[arg(
        long,
        help = "Simulate OverlayFS mount options without spawning kernel namespaces"
    )]
    pub dry_run: bool,

    #[arg(long, help = "Override executable command")]
    pub exec: Option<String>,

    #[arg(
        long,
        help = "Explicit path to Proton binary or installation directory"
    )]
    pub proton: Option<PathBuf>,

    #[arg(long, help = "Explicit path to Wine prefix or compatdata directory")]
    pub prefix: Option<PathBuf>,

    #[arg(long, help = "Force direct native execution without Proton")]
    pub native: bool,

    #[arg(last = true, help = "Arguments passed through to the game executable")]
    pub args: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: Option<ConfigAction>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    #[command(about = "Displays the path to the global config.toml")]
    Path,
    #[command(about = "Displays the active global configuration")]
    Show,
    #[command(about = "Initializes default ~/.config/perihelion/config.toml")]
    Init,
}
