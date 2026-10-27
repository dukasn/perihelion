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

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
    Frame,
};

use crate::{
    action::{StatusLevel, Tab},
    app::App,
    theme::{
        COLOR_AMBER, COLOR_BORDER_SUBTLE, COLOR_CYAN, COLOR_FOCUS, COLOR_GREEN, COLOR_RED,
        COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
    views::{
        conflicts::render_conflicts_view,
        downloads::render_downloads_view,
        help::render_help_modal,
        mods::render_mods_view,
        overview::render_overview_view,
        profiles::{render_game_wizard_modal, render_profiles_view, render_steam_importer_modal},
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
            Constraint::Length(1), // Minimal Context & Status Footer
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

    render_footer(f, app, root_chunks[3], root_chunks[4]);

    if app.show_help {
        render_help_modal(f, size);
    } else if app.show_steam_importer {
        render_steam_importer_modal(f, app, size);
    } else if app.show_game_wizard {
        render_game_wizard_modal(f, app, size);
    }
}

fn render_header(f: &mut Frame, app: &App, bar_area: Rect, sep_area: Rect) {
    let delta = app.staged.delta_count();
    let right_len = if delta > 0 { 38 } else { 16 };
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(40),           // Standard Ratatui Tabs
            Constraint::Length(right_len), // Right-aligned Staged + Launch CTA
        ])
        .split(bar_area);

    let mod_count = app.mods.len();
    let conflict_count = app.conflicts.len();

    let titles: Vec<Line> = vec![
        // 1:Overview
        if app.active_tab == Tab::Overview {
            Line::from(vec![Span::styled(
                "1:Overview",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("1:", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Overview", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ])
        },
        // 2:Mods [5]
        if app.active_tab == Tab::Mods {
            Line::from(vec![
                Span::styled(
                    "2:Mods",
                    Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" [{}]", mod_count),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("2:", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Mods", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    format!(" [{}]", mod_count),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ])
        },
        // 3:Conflicts [1]
        if conflict_count > 0 {
            if app.active_tab == Tab::Conflicts {
                Line::from(vec![
                    Span::styled(
                        "3:Conflicts",
                        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" [{}]", conflict_count),
                        Style::default()
                            .fg(COLOR_AMBER)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("3:", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled("Conflicts", Style::default().fg(COLOR_TEXT_SECONDARY)),
                    Span::styled(
                        format!(" [{}]", conflict_count),
                        Style::default().fg(COLOR_AMBER),
                    ),
                ])
            }
        } else if app.active_tab == Tab::Conflicts {
            Line::from(vec![Span::styled(
                "3:Conflicts",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("3:", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Conflicts", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ])
        },
        // 4:Downloads
        if app.active_tab == Tab::Downloads {
            Line::from(vec![Span::styled(
                "4:Downloads",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("4:", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Downloads", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ])
        },
        // 5:Profiles
        if app.active_tab == Tab::Profiles {
            Line::from(vec![Span::styled(
                "5:Profiles",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("5:", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Profiles", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ])
        },
    ];

    let active_index = match app.active_tab {
        Tab::Overview => 0,
        Tab::Mods => 1,
        Tab::Conflicts => 2,
        Tab::Downloads => 3,
        Tab::Profiles => 4,
    };

    let tabs = Tabs::new(titles)
        .select(active_index)
        .highlight_style(Style::default())
        .divider(Span::styled(
            " | ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ))
        .padding(" ", "");

    f.render_widget(tabs, header_chunks[0]);

    // Right-aligned Launch CTA & Pending Staged Indicator
    let mut right_spans = Vec::new();
    let delta = app.staged.delta_count();
    if delta > 0 {
        right_spans.push(Span::styled(
            format!("[{} PENDING (*)] ", delta),
            Style::default()
                .fg(COLOR_AMBER)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::styled(
            "[F5 Apply] ",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(active) = &app.active_session {
        right_spans.push(Span::styled(
            format!("[● PID {}] ", active.pid),
            Style::default()
                .fg(COLOR_GREEN)
                .add_modifier(Modifier::BOLD),
        ));
    }
    right_spans.push(Span::styled(
        "F9",
        Style::default()
            .fg(COLOR_TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD),
    ));
    right_spans.push(Span::styled(
        " Launch ",
        Style::default().fg(COLOR_TEXT_SECONDARY),
    ));

    f.render_widget(
        Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
        header_chunks[1],
    );

    // Subtle divider line below header
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_area.width as usize),
        Style::default().fg(COLOR_BORDER_SUBTLE),
    ));
    f.render_widget(Paragraph::new(sep_line), sep_area);
}

fn render_footer(f: &mut Frame, app: &App, sep_area: Rect, footer_area: Rect) {
    // Horizontal divider rule separating workspace from footer
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_area.width as usize),
        Style::default().fg(COLOR_BORDER_SUBTLE),
    ));
    f.render_widget(Paragraph::new(sep_line), sep_area);

    let (cursor_pos, cursor_total) = match app.active_tab {
        Tab::Overview => (app.overview_profile_cursor + 1, 5),
        Tab::Mods => {
            let len = app.filtered_mods().len();
            (if len == 0 { 0 } else { app.cursor_index + 1 }, len)
        }
        Tab::Conflicts => {
            let len = app.conflicts.len();
            (
                if len == 0 {
                    0
                } else {
                    app.active_conflict_cursor + 1
                },
                len,
            )
        }
        Tab::Downloads => {
            let len = app.downloads.len();
            (if len == 0 { 0 } else { app.download_cursor + 1 }, len)
        }
        Tab::Profiles => {
            let len = app.profiles.len();
            (if len == 0 { 0 } else { app.profile_cursor + 1 }, len)
        }
    };

    let mut right_spans = Vec::new();
    if app.active_tab == Tab::Profiles && footer_area.width >= 100 {
        right_spans.push(Span::styled(
            "[g] Define Game  [s] Scan Steam  |  ",
            Style::default().fg(COLOR_FOCUS),
        ));
    }
    if footer_area.width >= 115 {
        right_spans.push(Span::styled(
            format!("Row {}/{}", cursor_pos, cursor_total),
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled(
            &app.proton_version,
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled(
            "[hold ",
            Style::default().fg(COLOR_TEXT_MUTED),
        ));
        right_spans.push(Span::styled(
            "?",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::styled(
            " for hotkeys] ",
            Style::default().fg(COLOR_TEXT_MUTED),
        ));
    } else if footer_area.width >= 90 {
        right_spans.push(Span::styled(
            format!("Row {}/{}", cursor_pos, cursor_total),
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled(
            &app.proton_version,
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled("[", Style::default().fg(COLOR_TEXT_MUTED)));
        right_spans.push(Span::styled(
            "?",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::styled(
            " Hotkeys] ",
            Style::default().fg(COLOR_TEXT_MUTED),
        ));
    } else if footer_area.width >= 70 {
        right_spans.push(Span::styled(
            format!("Row {}/{}", cursor_pos, cursor_total),
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled("[", Style::default().fg(COLOR_TEXT_MUTED)));
        right_spans.push(Span::styled(
            "?",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::styled(
            " Hotkeys] ",
            Style::default().fg(COLOR_TEXT_MUTED),
        ));
    } else if footer_area.width >= 50 {
        right_spans.push(Span::styled(
            format!("Row {}/{}", cursor_pos, cursor_total),
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ));
        right_spans.push(Span::styled(
            "  |  ",
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ));
        right_spans.push(Span::styled("[", Style::default().fg(COLOR_TEXT_MUTED)));
        right_spans.push(Span::styled(
            "?",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::styled("] ", Style::default().fg(COLOR_TEXT_MUTED)));
    }

    let right_width = right_spans
        .iter()
        .map(|s| s.content.chars().count())
        .sum::<usize>() as u16;

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(right_width)])
        .split(footer_area);

    let game_label = if app.game_title == "No Active Workspace" {
        "No Active Workspace"
    } else if let Some(prof) = app.profiles.iter().find(|p| p.is_active) {
        prof.game_short.as_str()
    } else if let Some(bracket_pos) = app.game_title.find(" [") {
        &app.game_title[..bracket_pos]
    } else {
        &app.game_title
    };

    let mut left_spans = vec![
        Span::raw(" "),
        Span::styled(
            game_label,
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ::  ", Style::default().fg(COLOR_TEXT_MUTED)),
    ];

    if app.is_any_input_active() {
        left_spans.push(Span::styled(
            "Editing input -- Enter to submit, Esc to cancel",
            Style::default().fg(COLOR_CYAN),
        ));
    } else if let Some(msg) = &app.status_message {
        let msg_color = match app.status_level {
            StatusLevel::Error => COLOR_RED,
            StatusLevel::Warning => COLOR_AMBER,
            StatusLevel::Success => COLOR_TEXT_SECONDARY,
            StatusLevel::Info => COLOR_TEXT_SECONDARY,
        };
        left_spans.push(Span::styled(msg, Style::default().fg(msg_color)));
    } else {
        let active_count = app.mods.iter().filter(|m| m.active).count();
        let total_mods = app.mods.len();
        let delta = app.staged.delta_count();
        if delta > 0 {
            if footer_area.width >= 100 {
                left_spans.push(Span::styled(
                    format!(
                        "{} of {} Mods Active | {} Staged Edits (*) [F5 Apply / Shift+D Discard]",
                        active_count, total_mods, delta
                    ),
                    Style::default().fg(COLOR_AMBER),
                ));
            } else {
                left_spans.push(Span::styled(
                    format!(
                        "{} of {} Mods | {} Staged (*)",
                        active_count, total_mods, delta
                    ),
                    Style::default().fg(COLOR_AMBER),
                ));
            }
        } else {
            left_spans.push(Span::styled(
                format!("{} of {} Mods Active (0 cycles)", active_count, total_mods),
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ));
        }
    }

    f.render_widget(Paragraph::new(Line::from(left_spans)), footer_chunks[0]);
    f.render_widget(
        Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
        footer_chunks[1],
    );
}
