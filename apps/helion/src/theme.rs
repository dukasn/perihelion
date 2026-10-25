#![allow(dead_code)]

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Padding},
};

// Terminal-native palette: uses Color::Reset for surfaces so Helion adapts
// seamlessly to the user's active terminal background, transparency, and color scheme.
pub const COLOR_BG: Color = Color::Reset;
pub const COLOR_HEADER_BG: Color = Color::Reset;
pub const COLOR_PANEL_BG: Color = Color::Reset;
pub const COLOR_CARD_BG: Color = Color::Reset;
pub const COLOR_ROW_ALT: Color = Color::Reset;
pub const COLOR_SURFACE: Color = Color::DarkGray;
pub const COLOR_KEYCAP_BG: Color = Color::Black;

// Input Wells: use terminal ANSI Black (0) to create a subtle sunken well within the active theme
pub const COLOR_INPUT_BG: Color = Color::Black;
pub const COLOR_INPUT_ACTIVE_BG: Color = Color::Black;
pub const COLOR_INPUT_ERROR_BG: Color = Color::Black;
pub const COLOR_INPUT_VALID_BG: Color = Color::Black;

// Borders & Dividers (mapped to terminal ANSI colors)
pub const COLOR_BORDER_SUBTLE: Color = Color::DarkGray;
pub const COLOR_BORDER_NORMAL: Color = Color::DarkGray;
pub const COLOR_BORDER_FOCUSED: Color = Color::Cyan;
pub const COLOR_SEPARATOR: Color = Color::DarkGray;

// Semantic Accents (terminal ANSI palette)
pub const COLOR_CYAN: Color = Color::Cyan;
pub const COLOR_CYAN_BG: Color = Color::Black;

// ANSI Blue (4) is notoriously low-contrast on dark terminals; LightBlue (12) reads cleanly.
pub const COLOR_BLUE: Color = Color::LightBlue;
pub const COLOR_BLUE_BG: Color = Color::Black;

pub const COLOR_INDIGO: Color = Color::Magenta;
pub const COLOR_INDIGO_BG: Color = Color::Black;

pub const COLOR_GREEN: Color = Color::Green;
pub const COLOR_GREEN_BG: Color = Color::Black;

pub const COLOR_AMBER: Color = Color::Yellow;
pub const COLOR_AMBER_BG: Color = Color::Black;

pub const COLOR_RED: Color = Color::Red;
pub const COLOR_RED_BG: Color = Color::Black;

// Typography (adapts to terminal foreground)
pub const COLOR_TEXT_PRIMARY: Color = Color::Reset;
// Hierarchy: PRIMARY (fg + bold) > SECONDARY (fg) > MUTED (Gray). DarkGray is reserved
// for borders/rules only -- it fails contrast for readable text on most dark themes.
pub const COLOR_TEXT_SECONDARY: Color = Color::Reset;
pub const COLOR_TEXT_MUTED: Color = Color::Gray;
pub const COLOR_TEXT_BRACKET: Color = Color::DarkGray;

pub fn style_header_title() -> Style {
    Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)
}

pub fn style_text_primary() -> Style {
    Style::default().fg(COLOR_TEXT_PRIMARY)
}

pub fn style_text_secondary() -> Style {
    Style::default().fg(COLOR_TEXT_SECONDARY)
}

pub fn style_text_muted() -> Style {
    Style::default().fg(COLOR_TEXT_MUTED)
}

/// Dimmed placeholder style for input fields when idle or empty.
pub fn style_placeholder() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM)
}

/// Dimmed ghost-text style for inline autocomplete suggestions.
pub fn style_ghost_text() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM | Modifier::ITALIC)
}

pub fn style_accent_green() -> Style {
    Style::default().fg(COLOR_GREEN)
}

pub fn style_accent_cyan() -> Style {
    Style::default().fg(COLOR_CYAN)
}

pub fn style_accent_amber() -> Style {
    Style::default().fg(COLOR_AMBER)
}

pub fn style_active_tab() -> Style {
    Style::default()
        .fg(COLOR_CYAN)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

pub fn style_inactive_tab() -> Style {
    Style::default().fg(COLOR_TEXT_MUTED)
}

pub fn style_border_focused() -> Style {
    Style::default()
        .fg(COLOR_BORDER_FOCUSED)
        .add_modifier(Modifier::BOLD)
}

pub fn style_border_normal() -> Style {
    Style::default().fg(COLOR_BORDER_NORMAL)
}

pub fn style_play_button() -> Style {
    Style::default()
        .fg(COLOR_GREEN)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

/// Renders a terminal-native inline tag: colored text only, no brackets.
/// Brackets on every tag created visual noise ("[x] ON [Fixes] [* Standard]"); color and
/// weight alone carry the semantics, and spacing between tags is handled by callers.
pub fn pill<'a>(text: impl Into<String>, fg: Color, _bg: Color, bold: bool) -> Span<'a> {
    let mut style = Style::default().fg(fg);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text.into(), style)
}

/// Renders an inverted high-contrast pill using `Modifier::REVERSED` so it adapts
/// to the user's terminal foreground/background colors.
pub fn pill_solid<'a>(text: impl Into<String>, fg: Color) -> Span<'a> {
    Span::styled(
        format!(" {} ", text.into()),
        Style::default()
            .fg(fg)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    )
}

pub fn keycap<'a>(key: impl Into<String>) -> Span<'a> {
    Span::styled(
        key.into(),
        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
    )
}

/// Top-level panel. Focus is communicated by border *color* only; the border *shape*
/// stays Rounded so focus changes never cause the layout to visually "jump".
pub fn panel_block<'a>(focused: bool) -> Block<'a> {
    let border_color = if focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .padding(Padding::new(2, 2, 1, 0))
}

/// Nested card. Horizontal padding only: vertical padding inside small fixed-height cards
/// consumed every content row (which is why profile cards rendered empty).
pub fn card_block<'a>(border_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0))
}

/// Input well. Same Rounded shape as everything else; state is expressed by color
/// (cyan = editing, green = valid, red = error) so the field never changes geometry.
pub fn input_block<'a>(active: bool, has_error: bool, is_valid: bool) -> Block<'a> {
    let border_color = if has_error {
        COLOR_RED
    } else if is_valid {
        COLOR_GREEN
    } else if active {
        COLOR_CYAN
    } else {
        COLOR_BORDER_NORMAL
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0))
}
