use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "peri",
    author = "Perihelion Contributors",
    version,
    about = "Game-agnostic native Linux package manager & VFS orchestrator",
    long_about = "Perihelion is a high-performance native Linux mod manager and VFS orchestrator\n\
                  that uses kernel-level OverlayFS and isolated namespaces to deliver safe,\n\
                  declarative game modding."
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

    #[arg(last = true, help = "Arguments passed through to the game executable")]
    pub args: Vec<String>,
}
