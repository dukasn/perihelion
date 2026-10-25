use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::theme::{
    keycap, COLOR_BORDER_FOCUSED, COLOR_CYAN, COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

pub fn render_help_modal(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(68, 78, area);

    // Clear background behind dialog
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(
            Style::default()
                .fg(COLOR_BORDER_FOCUSED)
                .add_modifier(Modifier::BOLD),
        )
        .padding(Padding::new(3, 3, 1, 1))
        .title(
            Line::from(Span::styled(
                " ◆ Helion — Mod Fearlessly. Share Instantly. ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " [Esc / ? / F1] Close ",
                Style::default().fg(COLOR_TEXT_MUTED),
            ))
            .alignment(Alignment::Right),
        );

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rule = "─".repeat(inner.width as usize);

    let help_lines = vec![
        Line::from(Span::styled(
            "SELECTABLE AREA FOCUS & PAGE NAVIGATION",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            rule.clone(),
            Style::default().fg(COLOR_SEPARATOR),
        )),
        shortcut_row(
            "Tab / S-Tab ",
            "Cycle focus through selectable areas in the current page",
        ),
        shortcut_row(
            "Space / Enter",
            "Activate or toggle the currently focused area / item",
        ),
        shortcut_row(
            "1 - 5 ([ / ])",
            "Switch top-level pages (Overview, Mods, Conflicts, Downloads, Profiles)",
        ),
        shortcut_row(
            "/  ·  c     ",
            "Focus Quick Action Bar on Overview [1] or Filter / Input on other tabs",
        ),
        shortcut_row(
            "↑/k  ↓/j    ",
            "Navigate items vertically; g/Home & G/End jump to top/bottom",
        ),
        shortcut_row("Ctrl+U / D  ", "Page Up / Page Down through package lists"),
        Line::from(""),
        Line::from(Span::styled(
            "MOD LOAD ORDER, FILTERING & REORDERING",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            rule.clone(),
            Style::default().fg(COLOR_SEPARATOR),
        )),
        shortcut_row(
            "Space / Enter",
            "Toggle selected package Active ([x] ON) / Disabled ([ ] OFF)",
        ),
        shortcut_row(
            "a           ",
            "Batch toggle all currently visible packages On / Off",
        ),
        shortcut_row(
            "J / K       ",
            "Move selected mod Down / Up in the load order (also Alt+↑/↓)",
        ),
        shortcut_row("p / P (+ / -)", "Cycle Priority Tier forward / backward"),
        shortcut_row("c / C       ", "Cycle Category Filter forward / backward"),
        shortcut_row(
            "/  ·  Ctrl+L",
            "Focus search input well · Clear active search & category filters",
        ),
        shortcut_row("i           ", "Show / hide the Package Inspector drawer"),
        Line::from(""),
        Line::from(Span::styled(
            "MULTI-GAME PROFILES, DOWNLOADS & EXECUTION",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(rule, Style::default().fg(COLOR_SEPARATOR))),
        shortcut_row(
            "Enter / Esc ",
            "Validate & submit active input field / Cancel input editing",
        ),
        shortcut_row(
            "Ctrl+W / U  ",
            "Delete previous word / Clear entire input field",
        ),
        shortcut_row(
            "n / t / f   ",
            "New Profile input [4] · Cycle Target Game [4] · Fix Check [4]",
        ),
        shortcut_row(
            "u           ",
            "Focus Direct URI / Magnet / Slug input in Downloads [3]",
        ),
        shortcut_row(
            "F9 / Shift+L",
            "Mount OverlayFS stack & launch game inside isolated namespace",
        ),
    ];

    let p = Paragraph::new(help_lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
}

fn shortcut_row<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        keycap(key),
        Span::raw("   "),
        Span::styled(desc, Style::default().fg(COLOR_TEXT_PRIMARY)),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
