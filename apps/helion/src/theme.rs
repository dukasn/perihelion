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

#![allow(dead_code)]

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

// -----------------------------------------------------------------------------
// 1. ColorToken: Flexible, Human-Friendly Color Representation
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorToken {
    Named(String),
}

impl ColorToken {
    pub fn new(val: impl Into<String>) -> Self {
        Self::Named(val.into())
    }

    pub fn to_color(&self) -> Color {
        match self {
            Self::Named(s) => {
                let trimmed = s.trim();
                if trimmed.starts_with('#') {
                    Self::parse_hex(trimmed).unwrap_or(Color::Reset)
                } else {
                    match trimmed.to_ascii_lowercase().as_str() {
                        "reset" | "transparent" => Color::Reset,
                        "black" => Color::Black,
                        "red" => Color::Red,
                        "green" => Color::Green,
                        "yellow" | "amber" => Color::Yellow,
                        "blue" => Color::Blue,
                        "magenta" | "purple" => Color::Magenta,
                        "cyan" => Color::Cyan,
                        "gray" | "grey" => Color::Gray,
                        "dark_gray" | "darkgray" => Color::DarkGray,
                        "light_red" | "lightred" => Color::LightRed,
                        "light_green" | "lightgreen" => Color::LightGreen,
                        "light_yellow" | "lightyellow" => Color::LightYellow,
                        "light_blue" | "lightblue" => Color::LightBlue,
                        "light_magenta" | "lightmagenta" => Color::LightMagenta,
                        "light_cyan" | "lightcyan" => Color::LightCyan,
                        "white" => Color::White,
                        _ => Self::parse_hex(trimmed).unwrap_or(Color::Reset),
                    }
                }
            }
        }
    }

    fn parse_hex(hex: &str) -> Option<Color> {
        let clean = hex.trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
            let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
            let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        } else if clean.len() == 3 {
            let r = u8::from_str_radix(&clean[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&clean[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&clean[2..3], 16).ok()? * 17;
            Some(Color::Rgb(r, g, b))
        } else {
            None
        }
    }
}

// -----------------------------------------------------------------------------
// 2. Token Groups
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceTokens {
    pub bg: ColorToken,
    pub panel_bg: ColorToken,
    pub card_bg: ColorToken,
    pub input_bg: ColorToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderTokens {
    pub normal: ColorToken,
    pub focused: ColorToken,
    pub subtle: ColorToken,
    pub separator: ColorToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextTokens {
    pub primary: ColorToken,
    pub secondary: ColorToken,
    pub muted: ColorToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionTokens {
    pub bg: ColorToken,
    pub fg: ColorToken,
    pub secondary_fg: ColorToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccentTokens {
    pub focus: ColorToken,
    pub cyan: ColorToken,
    pub green: ColorToken,
    pub amber: ColorToken,
    pub red: ColorToken,
}

// -----------------------------------------------------------------------------
// 3. Theme Definition
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub description: String,
    pub surfaces: SurfaceTokens,
    pub borders: BorderTokens,
    pub text: TextTokens,
    pub selection: SelectionTokens,
    pub accents: AccentTokens,
}

impl Default for Theme {
    fn default() -> Self {
        Self::terminal()
    }
}

impl Theme {
    // -------------------------------------------------------------------------
    // Built-in Presets
    // -------------------------------------------------------------------------

    /// 1. Terminal Native (Default): Adapts cleanly to host terminal transparency & ANSI colors.
    pub fn terminal() -> Self {
        Self {
            id: "terminal".to_string(),
            name: "Terminal Native".to_string(),
            description: "Transparent surfaces adapting directly to your host shell palette"
                .to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("reset"),
                panel_bg: ColorToken::new("reset"),
                card_bg: ColorToken::new("reset"),
                input_bg: ColorToken::new("black"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("dark_gray"),
                focused: ColorToken::new("light_blue"),
                subtle: ColorToken::new("dark_gray"),
                separator: ColorToken::new("dark_gray"),
            },
            text: TextTokens {
                primary: ColorToken::new("white"),
                secondary: ColorToken::new("gray"),
                muted: ColorToken::new("dark_gray"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("light_blue"),
                fg: ColorToken::new("white"),
                secondary_fg: ColorToken::new("white"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("light_blue"),
                cyan: ColorToken::new("light_blue"),
                green: ColorToken::new("green"),
                amber: ColorToken::new("yellow"),
                red: ColorToken::new("red"),
            },
        }
    }

    /// 2. Campbell: Windows Terminal / WSL default dark palette.
    pub fn campbell() -> Self {
        Self {
            id: "campbell".to_string(),
            name: "Campbell Dark".to_string(),
            description: "Windows Terminal signature deep #0C0C0C palette with crisp primaries"
                .to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#0C0C0C"),
                panel_bg: ColorToken::new("#0C0C0C"),
                card_bg: ColorToken::new("#161616"),
                input_bg: ColorToken::new("#0C0C0C"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#767676"),
                focused: ColorToken::new("#3B78FF"),
                subtle: ColorToken::new("#303030"),
                separator: ColorToken::new("#303030"),
            },
            text: TextTokens {
                primary: ColorToken::new("#F2F2F2"),
                secondary: ColorToken::new("#CCCCCC"),
                muted: ColorToken::new("#767676"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#3B78FF"),
                fg: ColorToken::new("#FFFFFF"),
                secondary_fg: ColorToken::new("#E0E0E0"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#3B78FF"),
                cyan: ColorToken::new("#3A96DD"),
                green: ColorToken::new("#13A10E"),
                amber: ColorToken::new("#C19C00"),
                red: ColorToken::new("#C50F1F"),
            },
        }
    }

    /// 3. Catppuccin Mocha: Soothing pastel high-contrast dark theme.
    pub fn catppuccin_mocha() -> Self {
        Self {
            id: "catppuccin".to_string(),
            name: "Catppuccin Mocha".to_string(),
            description: "Cosmic pastel harmony featuring #1E1E2E base and sapphire accents"
                .to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#1E1E2E"),
                panel_bg: ColorToken::new("#181825"),
                card_bg: ColorToken::new("#313244"),
                input_bg: ColorToken::new("#11111B"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#45475A"),
                focused: ColorToken::new("#89B4FA"),
                subtle: ColorToken::new("#313244"),
                separator: ColorToken::new("#313244"),
            },
            text: TextTokens {
                primary: ColorToken::new("#CDD6F4"),
                secondary: ColorToken::new("#A6ADC8"),
                muted: ColorToken::new("#6C7086"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#89B4FA"),
                fg: ColorToken::new("#1E1E2E"),
                secondary_fg: ColorToken::new("#313244"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#89B4FA"),
                cyan: ColorToken::new("#89DCEB"),
                green: ColorToken::new("#A6E3A1"),
                amber: ColorToken::new("#F9E2AF"),
                red: ColorToken::new("#F38BA8"),
            },
        }
    }

    /// 4. Tokyo Night: Clean, electric midnight storm palette.
    pub fn tokyo_night() -> Self {
        Self {
            id: "tokyo-night".to_string(),
            name: "Tokyo Night".to_string(),
            description: "Electric midnight blue slate (#1A1B26) with neon cyan highlights"
                .to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#1A1B26"),
                panel_bg: ColorToken::new("#16161E"),
                card_bg: ColorToken::new("#24283B"),
                input_bg: ColorToken::new("#13141C"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#414868"),
                focused: ColorToken::new("#7AA2F7"),
                subtle: ColorToken::new("#24283B"),
                separator: ColorToken::new("#24283B"),
            },
            text: TextTokens {
                primary: ColorToken::new("#C0CAF5"),
                secondary: ColorToken::new("#9AA5CE"),
                muted: ColorToken::new("#565F89"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#7AA2F7"),
                fg: ColorToken::new("#1A1B26"),
                secondary_fg: ColorToken::new("#24283B"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#7AA2F7"),
                cyan: ColorToken::new("#7DCFFF"),
                green: ColorToken::new("#9ECE6A"),
                amber: ColorToken::new("#E0AF68"),
                red: ColorToken::new("#F7768E"),
            },
        }
    }

    /// 5. Nord: Arctic, clean north-bluish palette.
    pub fn nord() -> Self {
        Self {
            id: "nord".to_string(),
            name: "Nordic Frost".to_string(),
            description: "Cool arctic polar night (#2E3440) accented by frost blues".to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#2E3440"),
                panel_bg: ColorToken::new("#242933"),
                card_bg: ColorToken::new("#3B4252"),
                input_bg: ColorToken::new("#1E222A"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#4C566A"),
                focused: ColorToken::new("#88C0D0"),
                subtle: ColorToken::new("#3B4252"),
                separator: ColorToken::new("#3B4252"),
            },
            text: TextTokens {
                primary: ColorToken::new("#ECEFF4"),
                secondary: ColorToken::new("#D8DEE9"),
                muted: ColorToken::new("#616E88"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#88C0D0"),
                fg: ColorToken::new("#2E3440"),
                secondary_fg: ColorToken::new("#3B4252"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#88C0D0"),
                cyan: ColorToken::new("#8FBCBB"),
                green: ColorToken::new("#A3BE8C"),
                amber: ColorToken::new("#EBCB8B"),
                red: ColorToken::new("#BF616A"),
            },
        }
    }

    /// 6. Dracula: High-contrast vampire dark mode.
    pub fn dracula() -> Self {
        Self {
            id: "dracula".to_string(),
            name: "Dracula".to_string(),
            description: "Gothic dark purple (#282A36) paired with vibrant neon accents"
                .to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#282A36"),
                panel_bg: ColorToken::new("#21222C"),
                card_bg: ColorToken::new("#44475A"),
                input_bg: ColorToken::new("#191A21"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#6272A4"),
                focused: ColorToken::new("#BD93F9"),
                subtle: ColorToken::new("#44475A"),
                separator: ColorToken::new("#44475A"),
            },
            text: TextTokens {
                primary: ColorToken::new("#F8F8F2"),
                secondary: ColorToken::new("#E2E2DC"),
                muted: ColorToken::new("#6272A4"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#BD93F9"),
                fg: ColorToken::new("#282A36"),
                secondary_fg: ColorToken::new("#44475A"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#BD93F9"),
                cyan: ColorToken::new("#8BE9FD"),
                green: ColorToken::new("#50FA7B"),
                amber: ColorToken::new("#F1FA8C"),
                red: ColorToken::new("#FF5555"),
            },
        }
    }

    /// 7. Gruvbox Dark Hard: Retro warm groove.
    pub fn gruvbox() -> Self {
        Self {
            id: "gruvbox".to_string(),
            name: "Gruvbox Dark".to_string(),
            description: "Warm retro espresso (#1D2021) with natural earthen tones".to_string(),
            surfaces: SurfaceTokens {
                bg: ColorToken::new("#1D2021"),
                panel_bg: ColorToken::new("#141617"),
                card_bg: ColorToken::new("#32302F"),
                input_bg: ColorToken::new("#141617"),
            },
            borders: BorderTokens {
                normal: ColorToken::new("#665C54"),
                focused: ColorToken::new("#83A598"),
                subtle: ColorToken::new("#3C3836"),
                separator: ColorToken::new("#3C3836"),
            },
            text: TextTokens {
                primary: ColorToken::new("#EBDBB2"),
                secondary: ColorToken::new("#D5C4A1"),
                muted: ColorToken::new("#7C6F64"),
            },
            selection: SelectionTokens {
                bg: ColorToken::new("#83A598"),
                fg: ColorToken::new("#1D2021"),
                secondary_fg: ColorToken::new("#32302F"),
            },
            accents: AccentTokens {
                focus: ColorToken::new("#83A598"),
                cyan: ColorToken::new("#8EC07C"),
                green: ColorToken::new("#B8BB26"),
                amber: ColorToken::new("#FABD2F"),
                red: ColorToken::new("#FB4934"),
            },
        }
    }

    pub fn builtins() -> Vec<Self> {
        vec![
            Self::terminal(),
            Self::campbell(),
            Self::catppuccin_mocha(),
            Self::tokyo_night(),
            Self::nord(),
            Self::dracula(),
            Self::gruvbox(),
        ]
    }

    pub fn find_matching(query: &str) -> Option<Self> {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return None;
        }

        let all = Self::all_available();
        // Exact ID match
        if let Some(t) = all.iter().find(|t| t.id == q) {
            return Some(t.clone());
        }
        // Prefix ID match
        if let Some(t) = all.iter().find(|t| t.id.starts_with(&q)) {
            return Some(t.clone());
        }
        // Substring Name match
        if let Some(t) = all
            .iter()
            .find(|t| t.name.to_ascii_lowercase().contains(&q))
        {
            return Some(t.clone());
        }

        None
    }

    pub fn all_available() -> Vec<Self> {
        let mut themes = Self::builtins();

        // Scan custom user themes from ~/.config/perihelion/themes/*.toml
        if let Some(home) = directories::BaseDirs::new() {
            let user_themes_dir = home.config_dir().join("perihelion").join("themes");
            if user_themes_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(user_themes_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(t) = toml::from_str::<Theme>(&content) {
                                    // Override or append
                                    themes.retain(|existing| existing.id != t.id);
                                    themes.push(t);
                                }
                            }
                        }
                    }
                }
            }
        }

        themes
    }

    // -------------------------------------------------------------------------
    // Resolved Color Getters
    // -------------------------------------------------------------------------

    #[inline]
    pub fn bg(&self) -> Color {
        self.surfaces.bg.to_color()
    }
    #[inline]
    pub fn panel_bg(&self) -> Color {
        self.surfaces.panel_bg.to_color()
    }
    #[inline]
    pub fn card_bg(&self) -> Color {
        self.surfaces.card_bg.to_color()
    }
    #[inline]
    pub fn input_bg(&self) -> Color {
        self.surfaces.input_bg.to_color()
    }

    #[inline]
    pub fn border_normal(&self) -> Color {
        self.borders.normal.to_color()
    }
    #[inline]
    pub fn border_focused(&self) -> Color {
        self.borders.focused.to_color()
    }
    #[inline]
    pub fn border_subtle(&self) -> Color {
        self.borders.subtle.to_color()
    }
    #[inline]
    pub fn separator(&self) -> Color {
        self.borders.separator.to_color()
    }

    #[inline]
    pub fn text_primary(&self) -> Color {
        self.text.primary.to_color()
    }
    #[inline]
    pub fn text_secondary(&self) -> Color {
        self.text.secondary.to_color()
    }
    #[inline]
    pub fn text_muted(&self) -> Color {
        self.text.muted.to_color()
    }

    #[inline]
    pub fn selection_bg(&self) -> Color {
        self.selection.bg.to_color()
    }
    #[inline]
    pub fn selection_fg(&self) -> Color {
        self.selection.fg.to_color()
    }
    #[inline]
    pub fn selection_secondary_fg(&self) -> Color {
        self.selection.secondary_fg.to_color()
    }

    #[inline]
    pub fn focus(&self) -> Color {
        self.accents.focus.to_color()
    }
    #[inline]
    pub fn cyan(&self) -> Color {
        self.accents.cyan.to_color()
    }
    #[inline]
    pub fn green(&self) -> Color {
        self.accents.green.to_color()
    }
    #[inline]
    pub fn amber(&self) -> Color {
        self.accents.amber.to_color()
    }
    #[inline]
    pub fn red(&self) -> Color {
        self.accents.red.to_color()
    }

    // -------------------------------------------------------------------------
    // Semantic Component Styling
    // -------------------------------------------------------------------------

    pub fn selected_row(&self) -> Style {
        Style::default()
            .bg(self.selection_bg())
            .fg(self.selection_fg())
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row_secondary(&self) -> Style {
        Style::default()
            .bg(self.selection_bg())
            .fg(self.selection_secondary_fg())
    }

    pub fn panel_block<'a>(&self, focused: bool) -> Block<'a> {
        let border_color = if focused {
            self.border_focused()
        } else {
            self.border_normal()
        };
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title_style(Style::default().add_modifier(Modifier::BOLD))
            .padding(Padding::new(2, 2, 1, 0))
    }

    pub fn card_block<'a>(&self, border_color: Color) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(1, 1, 0, 0))
    }

    pub fn input_block<'a>(&self, active: bool, has_error: bool, is_valid: bool) -> Block<'a> {
        let border_color = if has_error {
            self.red()
        } else if is_valid {
            self.green()
        } else if active {
            self.cyan()
        } else {
            self.border_normal()
        };

        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(1, 1, 0, 0))
    }

    pub fn box_title_label<'a>(&self, text: impl Into<String>, focused: bool) -> Span<'a> {
        self.box_title_label_colored(text, self.cyan(), focused)
    }

    pub fn box_title_label_colored<'a>(
        &self,
        text: impl Into<String>,
        color: Color,
        focused: bool,
    ) -> Span<'a> {
        let t = text.into();
        let trimmed = t.trim();
        if focused {
            Span::styled(
                format!(" {} ", trimmed),
                Style::default()
                    .bg(color)
                    .fg(self.selection_fg())
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" {} ", trimmed),
                Style::default()
                    .fg(self.text_secondary())
                    .add_modifier(Modifier::BOLD),
            )
        }
    }

    pub fn style_block_right_label(&self, focused: bool) -> Style {
        if focused {
            Style::default()
                .fg(self.focus())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.border_normal())
        }
    }

    pub fn pill<'a>(&self, text: impl Into<String>, fg: Color, bold: bool) -> Span<'a> {
        let mut style = Style::default().fg(fg);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        Span::styled(text.into(), style)
    }

    pub fn pill_solid<'a>(&self, text: impl Into<String>, fg: Color) -> Span<'a> {
        Span::styled(
            format!(" {} ", text.into()),
            Style::default()
                .bg(fg)
                .fg(self.selection_fg())
                .add_modifier(Modifier::BOLD),
        )
    }

    pub fn keycap<'a>(&self, key: impl Into<String>) -> Span<'a> {
        Span::styled(
            key.into(),
            Style::default()
                .fg(self.cyan())
                .add_modifier(Modifier::BOLD),
        )
    }
}

// -----------------------------------------------------------------------------
// 4. Global Active Theme Store & Dynamic Delegators
// -----------------------------------------------------------------------------

static ACTIVE_THEME: RwLock<Option<Theme>> = RwLock::new(None);

pub fn set_active_theme(theme: Theme) {
    if let Ok(mut lock) = ACTIVE_THEME.write() {
        *lock = Some(theme);
    }
}

pub fn active_theme() -> Theme {
    if let Ok(lock) = ACTIVE_THEME.read() {
        if let Some(ref t) = *lock {
            return t.clone();
        }
    }
    Theme::terminal()
}

pub fn load_configured_theme() -> Theme {
    if cfg!(test) {
        return Theme::terminal();
    }

    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "perihelion", "perihelion") {
        let config_file = proj_dirs.config_dir().join(peri_core::CONFIG_FILENAME);
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(cfg) = peri_core::GlobalConfig::from_toml_str(&content) {
                if let Some(t) = Theme::find_matching(&cfg.ui.theme) {
                    return t;
                }
            }
        }
    } else if let Ok(home) = std::env::var("HOME") {
        let config_file = PathBuf::from(home)
            .join(".config/perihelion")
            .join(peri_core::CONFIG_FILENAME);
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(cfg) = peri_core::GlobalConfig::from_toml_str(&content) {
                if let Some(t) = Theme::find_matching(&cfg.ui.theme) {
                    return t;
                }
            }
        }
    }
    Theme::terminal()
}

pub fn save_configured_theme(theme_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(test) {
        return Ok(());
    }

    let config_dir = if let Some(proj_dirs) =
        directories::ProjectDirs::from("com", "perihelion", "perihelion")
    {
        proj_dirs.config_dir().to_path_buf()
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/perihelion")
    } else {
        PathBuf::from(".config/perihelion")
    };

    std::fs::create_dir_all(&config_dir)?;
    let config_file = config_dir.join(peri_core::CONFIG_FILENAME);

    let mut cfg = if config_file.exists() {
        let content = std::fs::read_to_string(&config_file)?;
        peri_core::GlobalConfig::from_toml_str(&content).unwrap_or_default()
    } else {
        peri_core::GlobalConfig::default()
    };

    cfg.ui.theme = theme_id.to_string();
    let serialized = cfg.to_toml_str().map_err(|e| e.to_string())?;
    std::fs::write(&config_file, serialized)?;
    Ok(())
}

// Dynamic color resolvers queried at render time
#[inline]
pub fn color_bg() -> Color {
    active_theme().bg()
}
#[inline]
pub fn color_panel_bg() -> Color {
    active_theme().panel_bg()
}
#[inline]
pub fn color_card_bg() -> Color {
    active_theme().card_bg()
}
#[inline]
pub fn color_input_bg() -> Color {
    active_theme().input_bg()
}

#[inline]
pub fn color_border_normal() -> Color {
    active_theme().border_normal()
}
#[inline]
pub fn color_border_focused() -> Color {
    active_theme().border_focused()
}
#[inline]
pub fn color_border_subtle() -> Color {
    active_theme().border_subtle()
}
#[inline]
pub fn color_separator() -> Color {
    active_theme().separator()
}

#[inline]
pub fn color_text_primary() -> Color {
    active_theme().text_primary()
}
#[inline]
pub fn color_text_secondary() -> Color {
    active_theme().text_secondary()
}
#[inline]
pub fn color_text_muted() -> Color {
    active_theme().text_muted()
}

#[inline]
pub fn color_focus() -> Color {
    active_theme().focus()
}
#[inline]
pub fn color_cyan() -> Color {
    active_theme().cyan()
}
#[inline]
pub fn color_green() -> Color {
    active_theme().green()
}
#[inline]
pub fn color_amber() -> Color {
    active_theme().amber()
}
#[inline]
pub fn color_red() -> Color {
    active_theme().red()
}

// Backwards-compatible constants (provide safe default fallbacks)
pub const COLOR_BG: Color = Color::Reset;
pub const COLOR_HEADER_BG: Color = Color::Reset;
pub const COLOR_PANEL_BG: Color = Color::Reset;
pub const COLOR_CARD_BG: Color = Color::Reset;
pub const COLOR_ROW_ALT: Color = Color::Reset;
pub const COLOR_SURFACE: Color = Color::DarkGray;
pub const COLOR_KEYCAP_BG: Color = Color::Black;

pub const COLOR_INPUT_BG: Color = Color::Black;
pub const COLOR_INPUT_ACTIVE_BG: Color = Color::Black;
pub const COLOR_INPUT_ERROR_BG: Color = Color::Black;
pub const COLOR_INPUT_VALID_BG: Color = Color::Black;

pub const COLOR_BORDER_SUBTLE: Color = Color::DarkGray;
pub const COLOR_BORDER_NORMAL: Color = Color::DarkGray;
pub const COLOR_BORDER_FOCUSED: Color = Color::LightBlue;
pub const COLOR_SEPARATOR: Color = Color::DarkGray;

pub const COLOR_FOCUS: Color = Color::LightBlue;
pub const COLOR_CYAN: Color = Color::LightBlue;
pub const COLOR_CYAN_BG: Color = Color::Black;
pub const COLOR_BLUE: Color = Color::Gray;
pub const COLOR_BLUE_BG: Color = Color::Black;
pub const COLOR_INDIGO: Color = Color::Gray;
pub const COLOR_INDIGO_BG: Color = Color::Black;

pub const COLOR_GREEN: Color = Color::Green;
pub const COLOR_GREEN_BG: Color = Color::Black;
pub const COLOR_AMBER: Color = Color::Yellow;
pub const COLOR_AMBER_BG: Color = Color::Black;
pub const COLOR_RED: Color = Color::Red;
pub const COLOR_RED_BG: Color = Color::Black;

pub const COLOR_TEXT_PRIMARY: Color = Color::White;
pub const COLOR_TEXT_SECONDARY: Color = Color::Gray;
pub const COLOR_TEXT_MUTED: Color = Color::DarkGray;
pub const COLOR_TEXT_BRACKET: Color = Color::DarkGray;

// Seamless helper functions (call into active_theme() for real-time responsiveness)
pub fn style_header_title() -> Style {
    Style::default()
        .fg(color_cyan())
        .add_modifier(Modifier::BOLD)
}

pub fn style_text_primary() -> Style {
    Style::default().fg(color_text_primary())
}

pub fn style_text_secondary() -> Style {
    Style::default().fg(color_text_secondary())
}

pub fn style_text_muted() -> Style {
    Style::default().fg(color_text_muted())
}

pub fn style_placeholder() -> Style {
    Style::default()
        .fg(color_text_muted())
        .add_modifier(Modifier::DIM)
}

pub fn style_ghost_text() -> Style {
    Style::default()
        .fg(color_text_muted())
        .add_modifier(Modifier::DIM | Modifier::ITALIC)
}

pub fn style_accent_green() -> Style {
    Style::default().fg(color_green())
}

pub fn style_accent_cyan() -> Style {
    Style::default().fg(color_cyan())
}

pub fn style_accent_amber() -> Style {
    Style::default().fg(color_amber())
}

pub fn style_active_tab() -> Style {
    Style::default()
        .fg(color_cyan())
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

pub fn style_inactive_tab() -> Style {
    Style::default().fg(color_text_muted())
}

pub fn style_border_focused() -> Style {
    Style::default()
        .fg(color_border_focused())
        .add_modifier(Modifier::BOLD)
}

pub fn style_border_normal() -> Style {
    Style::default().fg(color_border_normal())
}

pub fn style_play_button() -> Style {
    Style::default()
        .fg(color_green())
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

pub fn pill<'a>(text: impl Into<String>, fg: Color, _bg: Color, bold: bool) -> Span<'a> {
    active_theme().pill(text, fg, bold)
}

pub fn pill_solid<'a>(text: impl Into<String>, fg: Color) -> Span<'a> {
    active_theme().pill_solid(text, fg)
}

pub fn keycap<'a>(key: impl Into<String>) -> Span<'a> {
    active_theme().keycap(key)
}

pub fn style_selected_row() -> Style {
    active_theme().selected_row()
}

pub fn style_selected_row_secondary() -> Style {
    active_theme().selected_row_secondary()
}

pub fn style_block_right_label(focused: bool) -> Style {
    active_theme().style_block_right_label(focused)
}

pub fn box_title_label<'a>(text: impl Into<String>, focused: bool) -> Span<'a> {
    active_theme().box_title_label(text, focused)
}

pub fn box_title_label_colored<'a>(
    text: impl Into<String>,
    color: Color,
    focused: bool,
) -> Span<'a> {
    active_theme().box_title_label_colored(text, color, focused)
}

pub fn panel_block<'a>(focused: bool) -> Block<'a> {
    active_theme().panel_block(focused)
}

pub fn card_block<'a>(border_color: Color) -> Block<'a> {
    active_theme().card_block(border_color)
}

pub fn input_block<'a>(active: bool, has_error: bool, is_valid: bool) -> Block<'a> {
    active_theme().input_block(active, has_error, is_valid)
}

pub fn render_segmented_bar(progress: f32, total_cells: usize) -> Line<'static> {
    let progress = progress.clamp(0.0, 1.0);
    let filled_count = (progress * total_cells as f32).round() as usize;
    let empty_count = total_cells.saturating_sub(filled_count);

    let filled_style = Style::default().fg(color_green());
    let empty_style = Style::default().fg(color_border_normal());

    Line::from(vec![
        Span::styled("▮".repeat(filled_count), filled_style),
        Span::styled("▯".repeat(empty_count), empty_style),
        Span::styled(
            format!(" {:>3}%", (progress * 100.0) as u32),
            Style::default().fg(color_text_muted()),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_segmented_bar() {
        let line = render_segmented_bar(0.5, 10);
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content, "▮".repeat(5));
        assert_eq!(line.spans[1].content, "▯".repeat(5));
        assert_eq!(line.spans[2].content, "  50%");

        let zero = render_segmented_bar(-0.1, 8);
        assert_eq!(zero.spans[0].content, "");
        assert_eq!(zero.spans[1].content, "▯".repeat(8));
        assert_eq!(zero.spans[2].content, "   0%");

        let hundred = render_segmented_bar(1.5, 8);
        assert_eq!(hundred.spans[0].content, "▮".repeat(8));
        assert_eq!(hundred.spans[1].content, "");
        assert_eq!(hundred.spans[2].content, " 100%");
    }

    #[test]
    fn test_color_token_parsing() {
        assert_eq!(
            ColorToken::new("#0C0C0C").to_color(),
            Color::Rgb(12, 12, 12)
        );
        assert_eq!(
            ColorToken::new("#FFF").to_color(),
            Color::Rgb(255, 255, 255)
        );
        assert_eq!(ColorToken::new("green").to_color(), Color::Green);
        assert_eq!(ColorToken::new("light_blue").to_color(), Color::LightBlue);
        assert_eq!(ColorToken::new("reset").to_color(), Color::Reset);
    }

    #[test]
    fn test_theme_presets() {
        let builtins = Theme::builtins();
        assert!(builtins.len() >= 5);
        assert_eq!(Theme::campbell().id, "campbell");
        assert_eq!(Theme::catppuccin_mocha().id, "catppuccin");
        assert_eq!(Theme::nord().id, "nord");
        assert_eq!(Theme::dracula().id, "dracula");
    }

    #[test]
    fn test_find_matching() {
        assert_eq!(Theme::find_matching("campbell").unwrap().id, "campbell");
        assert_eq!(Theme::find_matching("cat").unwrap().id, "catppuccin");
        assert_eq!(Theme::find_matching("nord").unwrap().id, "nord");
        assert_eq!(Theme::find_matching("drac").unwrap().id, "dracula");
    }
}
