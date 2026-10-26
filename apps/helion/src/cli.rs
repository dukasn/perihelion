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

//! Command-Line Interface and Clap Styling for Helion.

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, ValueHint};
use std::path::PathBuf;

/// Produces consistent, high-contrast ANSI styles across Perihelion and Helion CLIs.
pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::BrightWhite.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::BrightCyan.on_default())
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
        .valid(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "helion",
    author = "Eduardo Marques Santos and Perihelion Contributors",
    version,
    about = "Terminal-native dashboard for mod curation and VFS diagnostics",
    long_about = "Helion - Keyboard-first interactive terminal dashboard for Perihelion.\n\n\
                  Curate mod lists, inspect file-level collisions, monitor P2P swarms,\n\
                  and switch declarative game profiles at 60 FPS without leaving your terminal.",
    styles = cli_styles(),
    color = clap::ColorChoice::Auto
)]
pub struct HelionArgs {
    /// Path to workspace root directory containing peri.toml
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = ".",
        value_hint = ValueHint::DirPath,
        help_heading = "Workspace Options"
    )]
    pub workspace: PathBuf,

    /// Initial view tab to focus on launch (overview, mods, conflicts, downloads, profiles)
    #[arg(short, long, value_name = "TAB", help_heading = "View Options")]
    pub tab: Option<String>,

    /// Ignore uncommitted crash recovery drafts and discard dirty session journals
    #[arg(long, help_heading = "Recovery Options")]
    pub no_recovery: bool,

    /// Enable verbose debug and worker telemetry logs
    #[arg(short, long, global = true, help_heading = "Logging")]
    pub verbose: bool,
}
