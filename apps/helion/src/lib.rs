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

//! Helion Terminal User Interface Library
//!
//! Exposes presentation state, UI rendering routines, event handling,
//! and background worker actor pipelines for `helion`.

pub mod action;
pub mod app;
pub mod autocomplete;
pub mod cli;
pub mod event;
pub mod theme;
pub mod ui;
pub mod views;
pub mod worker;

pub use cli::{cli_styles, HelionArgs};
