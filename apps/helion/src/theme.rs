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
pub const COLOR_BORDER_FOCUSED: Color = Color::LightBlue;
pub const COLOR_SEPARATOR: Color = Color::DarkGray;

// Focus & Selection Highlight: strictly Light Blue for focused elements only
pub const COLOR_FOCUS: Color = Color::LightBlue;
pub const COLOR_CYAN: Color = Color::LightBlue;
pub const COLOR_CYAN_BG: Color = Color::Black;

// Neutral tones for non-focused elements (dimmed white / gray)
pub const COLOR_BLUE: Color = Color::Gray;
pub const COLOR_BLUE_BG: Color = Color::Black;

pub const COLOR_INDIGO: Color = Color::Gray;
pub const COLOR_INDIGO_BG: Color = Color::Black;

// Functional status indicators (minimal usage only)
pub const COLOR_GREEN: Color = Color::Green;
pub const COLOR_GREEN_BG: Color = Color::Black;

pub const COLOR_AMBER: Color = Color::Yellow;
pub const COLOR_AMBER_BG: Color = Color::Black;

pub const COLOR_RED: Color = Color::Red;
pub const COLOR_RED_BG: Color = Color::Black;

// Minimalist Typography: White for primary, Dimmed White (Gray) for secondary, DarkGray for muted
pub const COLOR_TEXT_PRIMARY: Color = Color::White;
pub const COLOR_TEXT_SECONDARY: Color = Color::Gray;
pub const COLOR_TEXT_MUTED: Color = Color::DarkGray;
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

#[allow(dead_code)]
pub fn style_active_tab() -> Style {
    Style::default()
        .fg(COLOR_CYAN)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

/// Selected row / line style across tables, lists, and palettes:
/// Inverted full highlight background with black text (like focused title labels),
/// providing unambiguous visual focus across any terminal theme.
pub fn style_selected_row() -> Style {
    Style::default()
        .bg(COLOR_FOCUS)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Subtle/secondary elements on a selected row:
/// Black text on full highlight background.
pub fn style_selected_row_secondary() -> Style {
    Style::default().bg(COLOR_FOCUS).fg(Color::Black)
}

/// Style for right-aligned block title labels (shortcuts, counts, statuses) on fenced blocks:
/// When active/focused, highlighted text foreground matches the highlighted border/label color (`COLOR_FOCUS`).
/// When inactive/unfocused, it matches the normal border color (`COLOR_BORDER_NORMAL`).
/// No background changes are applied (background remains transparent).
pub fn style_block_right_label(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(COLOR_FOCUS)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_BORDER_NORMAL)
    }
}

/// Renders a block title label. When focused, applies an inverted solid background
/// (using the terminal's theme via `Modifier::REVERSED`) so the active box is immediately recognizable,
/// eliminating the need for a separate right-side `[FOCUS: ...]` tag.
pub fn box_title_label<'a>(text: impl Into<String>, focused: bool) -> Span<'a> {
    box_title_label_colored(text, COLOR_CYAN, focused)
}

pub fn box_title_label_colored<'a>(
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
                .fg(color)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
    } else {
        Span::styled(
            format!(" {} ", trimmed),
            Style::default()
                .fg(COLOR_TEXT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        )
    }
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

pub fn render_segmented_bar(progress: f32, total_cells: usize) -> Line<'static> {
    let progress = progress.clamp(0.0, 1.0);
    let filled_count = (progress * total_cells as f32).round() as usize;
    let empty_count = total_cells.saturating_sub(filled_count);

    let filled_style = Style::default().fg(Color::Rgb(45, 212, 191)); // Teal / Green
    let empty_style = Style::default().fg(Color::Rgb(55, 65, 81)); // Dim border gray

    Line::from(vec![
        Span::styled("▮".repeat(filled_count), filled_style),
        Span::styled("▯".repeat(empty_count), empty_style),
        Span::styled(
            format!(" {:>3}%", (progress * 100.0) as u32),
            Style::default().fg(Color::DarkGray),
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

        // Test clamped 0% and 100%
        let zero = render_segmented_bar(-0.1, 8);
        assert_eq!(zero.spans[0].content, "");
        assert_eq!(zero.spans[1].content, "▯".repeat(8));
        assert_eq!(zero.spans[2].content, "   0%");

        let hundred = render_segmented_bar(1.5, 8);
        assert_eq!(hundred.spans[0].content, "▮".repeat(8));
        assert_eq!(hundred.spans[1].content, "");
        assert_eq!(hundred.spans[2].content, " 100%");
    }
}
