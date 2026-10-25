use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    action::{StatusLevel, Tab},
    app::App,
    theme::{
        pill, pill_solid, style_play_button, COLOR_AMBER, COLOR_AMBER_BG, COLOR_BLUE,
        COLOR_BORDER_NORMAL, COLOR_BORDER_SUBTLE, COLOR_CYAN, COLOR_GREEN, COLOR_RED,
        COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
    views::{
        conflicts::render_conflicts_view, downloads::render_downloads_view,
        help::render_help_modal, mods::render_mods_view, overview::render_overview_view,
        profiles::render_profiles_view,
    },
};

pub fn render_ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top App Bar
            Constraint::Length(1), // Header bottom rule
            Constraint::Min(10),   // Main Workspace
            Constraint::Length(1), // Footer top rule
            Constraint::Length(1), // Telemetry & Status Bar
            Constraint::Length(1), // Segmented Keybinding Bar
        ])
        .split(size);

    render_header(f, app, root_chunks[0], root_chunks[1]);

    // 1-column horizontal breathing margin around the main workspace
    let workspace_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(20),
            Constraint::Length(1),
        ])
        .split(root_chunks[2])[1];

    match app.active_tab {
        Tab::Overview => render_overview_view(f, app, workspace_area),
        Tab::Mods => render_mods_view(f, app, workspace_area),
        Tab::Conflicts => render_conflicts_view(f, app, workspace_area),
        Tab::Downloads => render_downloads_view(f, app, workspace_area),
        Tab::Profiles => render_profiles_view(f, app, workspace_area),
    }

    render_footer(f, app, root_chunks[3], root_chunks[4], root_chunks[5]);

    if app.show_help {
        render_help_modal(f, size);
    }
}

fn render_header(f: &mut Frame, app: &App, bar_area: Rect, sep_area: Rect) {
    let left_width = (20 + app.game_title.chars().count() as u16).min(48);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width), // Brand & Active Workspace
            Constraint::Min(56),            // Navigation Pages (1-5)
            Constraint::Length(28),         // VFS Read-Only & Launch CTA
        ])
        .split(bar_area);

    // 1. Brand Badge & Game Profile
    let title_line = Line::from(vec![
        Span::raw(" "),
        pill_solid("◆ PERIHELION", COLOR_CYAN),
        Span::raw(" "),
        Span::styled(
            &app.game_title,
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(COLOR_BORDER_NORMAL)),
    ]);
    f.render_widget(Paragraph::new(title_line), header_chunks[0]);

    // 2. Top-Level Pages Bar (1-5)
    let mod_count = app.mods.len();
    let conflict_count = app.conflicts.len();
    let profile_count = app.profiles.len();

    let tabs_meta = [
        (Tab::Overview, "1", "Overview", None),
        (Tab::Mods, "2", "Mods", Some((mod_count.to_string(), false))),
        (
            Tab::Conflicts,
            "3",
            "Conflicts",
            if conflict_count > 0 {
                Some((conflict_count.to_string(), true))
            } else {
                None
            },
        ),
        (Tab::Downloads, "4", "Downloads", None),
        (
            Tab::Profiles,
            "5",
            "Profiles",
            Some((profile_count.to_string(), false)),
        ),
    ];

    let mut tab_spans = Vec::new();
    for (tab, key, label, badge) in tabs_meta {
        let is_active = app.active_tab == tab;

        if is_active {
            let text = if let Some((count, _)) = badge {
                format!("{}: {} ({})", key, label, count)
            } else {
                format!("{}: {}", key, label)
            };
            tab_spans.push(pill_solid(text, COLOR_CYAN));
        } else {
            tab_spans.push(Span::styled(
                key.to_string(),
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ));
            tab_spans.push(Span::styled(
                format!(" {}", label),
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ));
            if let Some((count, is_warn)) = badge {
                tab_spans.push(Span::raw(" "));
                if is_warn {
                    tab_spans.push(pill(count, COLOR_AMBER, COLOR_AMBER_BG, true));
                } else {
                    tab_spans.push(Span::styled(
                        format!("({})", count),
                        Style::default().fg(COLOR_TEXT_MUTED),
                    ));
                }
            }
        }
        tab_spans.push(Span::raw("   "));
    }

    f.render_widget(Paragraph::new(Line::from(tab_spans)), header_chunks[1]);

    // 3. Immutable VFS Isolation Indicator & Launch Game Button
    let vfs_span = Span::styled(
        "VFS:RO ",
        Style::default()
            .fg(COLOR_GREEN)
            .add_modifier(Modifier::BOLD),
    );
    let play_btn_span = Span::styled(" >> LAUNCH [F9] ", style_play_button());

    let right_line = Line::from(vec![vfs_span, play_btn_span, Span::raw(" ")]);
    let right_widget = Paragraph::new(right_line).alignment(Alignment::Right);
    f.render_widget(right_widget, header_chunks[2]);

    // Subtle divider line below header
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_area.width as usize),
        Style::default().fg(COLOR_BORDER_SUBTLE),
    ));
    f.render_widget(Paragraph::new(sep_line), sep_area);
}

fn render_footer(f: &mut Frame, app: &App, sep_area: Rect, status_area: Rect, hints_area: Rect) {
    // Horizontal divider rule separating workspace from footer
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_area.width as usize),
        Style::default().fg(COLOR_BORDER_SUBTLE),
    ));
    f.render_widget(Paragraph::new(sep_line), sep_area);

    // Row 1: Structured Telemetry & Status Row
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(38), Constraint::Length(56)])
        .split(status_area);

    let active_count = app.mods.iter().filter(|m| m.active).count();
    let total_mods = app.mods.len();
    let conflict_count = app.conflicts.len();

    let status_line = if app.is_any_input_active() {
        let mut spans = vec![
            Span::raw(" "),
            pill_solid("INPUT", COLOR_CYAN),
            Span::raw(" "),
        ];
        if let Some(msg) = &app.status_message {
            let msg_color = match app.status_level {
                StatusLevel::Error => COLOR_RED,
                StatusLevel::Warning => COLOR_AMBER,
                StatusLevel::Success => COLOR_GREEN,
                StatusLevel::Info => COLOR_TEXT_SECONDARY,
            };
            spans.push(Span::styled(
                msg,
                Style::default().fg(msg_color).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                "Editing active field -- Enter to submit, Esc to cancel",
                Style::default().fg(COLOR_TEXT_PRIMARY),
            ));
        }
        Line::from(spans)
    } else if let Some(msg) = &app.status_message {
        let (badge_label, fg) = match app.status_level {
            StatusLevel::Success => ("+ OK", COLOR_GREEN),
            StatusLevel::Warning => ("! NOTICE", COLOR_AMBER),
            StatusLevel::Error => ("x ERROR", COLOR_RED),
            StatusLevel::Info => ("* INFO", COLOR_BLUE),
        };
        Line::from(vec![
            Span::raw(" "),
            pill_solid(badge_label, fg),
            Span::raw(" "),
            Span::styled(msg, Style::default().fg(fg).add_modifier(Modifier::BOLD)),
        ])
    } else {
        let mut spans = vec![
            Span::raw(" "),
            pill_solid("READY", COLOR_GREEN),
            Span::raw(" "),
            Span::styled(
                format!("{}/{} Layers", active_count, total_mods),
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(COLOR_BORDER_NORMAL)),
            Span::styled("DAG: 0 Cycles", Style::default().fg(COLOR_TEXT_SECONDARY)),
        ];
        if conflict_count > 0 {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(COLOR_BORDER_NORMAL),
            ));
            spans.push(Span::styled(
                format!("[!] {} Collision", conflict_count),
                Style::default()
                    .fg(COLOR_AMBER)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(status_line), status_chunks[0]);

    let (cursor_pos, cursor_total) = match app.active_tab {
        Tab::Overview => (app.overview_profile_cursor + 1, app.profiles.len().max(1)),
        Tab::Mods => {
            let len = app.filtered_mods().len();
            (if len == 0 { 0 } else { app.cursor_index + 1 }, len)
        }
        Tab::Conflicts => (app.active_conflict_cursor + 1, app.conflicts.len().max(1)),
        Tab::Downloads => (app.download_cursor + 1, app.downloads.len().max(1)),
        Tab::Profiles => (app.profile_cursor + 1, app.profiles.len().max(1)),
    };

    let right_status_line = Line::from(vec![
        Span::styled("Focus: ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(
            app.current_focus_label(),
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(COLOR_BORDER_NORMAL)),
        Span::styled("Proton: ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(
            &app.proton_version,
            Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(COLOR_BORDER_NORMAL)),
        Span::styled(
            format!("{:02}/{:02} ", cursor_pos, cursor_total),
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(right_status_line).alignment(Alignment::Right),
        status_chunks[1],
    );

    // Row 2: Segmented Keybinding Bar (Left: Context Shortcuts, Right: Global Actions)
    let hint_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(50), Constraint::Length(34)])
        .split(hints_area);

    let hints: &[(&str, &str)] = if app.is_any_input_active() {
        &[
            ("Enter", "Submit"),
            ("Tab", "Next Area"),
            ("Esc", "Cancel"),
            ("Ctrl+W", "Del Word"),
            ("Ctrl+U", "Clear"),
        ]
    } else {
        match app.active_tab {
            Tab::Overview => &[
                ("Tab", "Focus Area"),
                ("↑/↓", "Select Profile"),
                ("Enter", "Run / Switch"),
                ("/", "Command Bar"),
                ("c", "Clone"),
                ("1-5", "Pages"),
            ],
            Tab::Mods => &[
                ("Tab", "Focus Area"),
                ("↑/↓", "Move"),
                ("Space", "Toggle"),
                ("J/K", "Order"),
                ("p", "Priority"),
                ("c", "Category"),
                ("/", "Filter"),
                ("1-5", "Pages"),
            ],
            Tab::Conflicts => &[
                ("Tab", "Focus Area"),
                ("Space/Enter", "Select"),
                ("a/b", "Option A/B"),
                ("r", "Auto-Resolve"),
                ("1-5", "Pages"),
            ],
            Tab::Downloads => &[
                ("Tab", "Focus Area"),
                ("↑/↓", "Navigate"),
                ("Enter/Space", "Edit/Pause/Install"),
                ("u", "Focus URI"),
                ("1-5", "Pages"),
            ],
            Tab::Profiles => &[
                ("Tab", "Focus Area"),
                ("↑/↓", "Select Profile"),
                ("Enter", "Activate"),
                ("n", "New Profile"),
                ("t", "Target Game"),
                ("f", "Fix Check"),
                ("1-5", "Pages"),
            ],
        }
    };

    let mut left_hints = vec![Span::raw(" ")];
    for (idx, (key, label)) in hints.iter().enumerate() {
        left_hints.push(Span::styled(
            *key,
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ));
        left_hints.push(Span::styled(
            format!(" {}", label),
            Style::default().fg(COLOR_TEXT_MUTED),
        ));
        if idx + 1 < hints.len() {
            left_hints.push(Span::styled(
                "   ",
                Style::default().fg(COLOR_BORDER_SUBTLE),
            ));
        }
    }
    f.render_widget(Paragraph::new(Line::from(left_hints)), hint_chunks[0]);

    let right_hints = Line::from(vec![
        Span::styled(
            "?",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Help", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(" │ ", Style::default().fg(COLOR_BORDER_SUBTLE)),
        Span::styled(
            "F9",
            Style::default()
                .fg(COLOR_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Launch", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(" │ ", Style::default().fg(COLOR_BORDER_SUBTLE)),
        Span::styled(
            "q",
            Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit ", Style::default().fg(COLOR_TEXT_SECONDARY)),
    ]);
    f.render_widget(
        Paragraph::new(right_hints).alignment(Alignment::Right),
        hint_chunks[1],
    );
}
