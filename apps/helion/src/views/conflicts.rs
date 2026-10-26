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
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use crate::{
    action::ConflictsFocus,
    app::App,
    theme::{
        box_title_label, card_block, panel_block, style_block_right_label, style_selected_row,
        style_selected_row_secondary, COLOR_AMBER, COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL,
        COLOR_FOCUS, COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_conflicts_view(f: &mut Frame, app: &App, area: Rect) {
    if app.show_inspector {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(58),
                Constraint::Length(1), // Breathing gap between panes
                Constraint::Percentage(42),
            ])
            .split(area);

        render_conflicts_table_pane(f, app, chunks[0]);
        render_conflicts_inspector_pane(f, app, chunks[2]);
    } else {
        render_conflicts_table_pane(f, app, area);
    }
}

fn render_conflicts_table_pane(f: &mut Frame, app: &App, area: Rect) {
    let total_conflicts = app.conflicts.len();
    let total_resolved = app.resolved_conflicts.len();
    let auto_focused = app.conflicts_focus == ConflictsFocus::AutoResolve;
    let table_focused = app.conflicts_focus == ConflictsFocus::ResolvedList;

    let main_block = panel_block(false)
        .title(
            Line::from(Span::styled(
                " Virtual File Index (VFI) Conflicts ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::styled(
                    format!(
                        " {} Collisions | {} Resolved ",
                        total_conflicts, total_resolved
                    ),
                    style_block_right_label(table_focused),
                ),
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );
    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Action & Waterfall Toolbar
            Constraint::Min(5),    // Conflicts Table
            Constraint::Length(1), // Divider
            Constraint::Length(1), // Bottom Preview / Waterfall Rule
        ])
        .split(inner);

    // 1. Top Action & Waterfall Toolbar
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(36),
            Constraint::Length(2),
            Constraint::Length(36),
        ])
        .split(vertical_layout[0]);

    let auto_btn_block = card_block(if auto_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    })
    .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
    .title(
        Line::from(box_title_label("Auto-Resolve CTA [r]", auto_focused))
            .alignment(Alignment::Left),
    );

    let auto_btn_text = Line::from(vec![Span::styled(
        if auto_focused {
            ">> [r / Enter] Auto-Resolve All with Priority"
        } else {
            "[r] Auto-Resolve All with Priority Waterfall"
        },
        if auto_focused {
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT_SECONDARY)
        },
    )]);
    f.render_widget(
        Paragraph::new(auto_btn_text).block(auto_btn_block),
        top_chunks[0],
    );

    let solver_badge_block = card_block(COLOR_BORDER_NORMAL)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
        .title(
            Line::from(Span::styled(
                " Waterfall Status ",
                Style::default().fg(COLOR_TEXT_MUTED),
            ))
            .alignment(Alignment::Right),
        );
    let solver_text = Line::from(vec![
        Span::styled("Tier-1", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(" -> ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(
            "Tier-2 Priority",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" -> ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled("Tier-3 Hard", Style::default().fg(COLOR_TEXT_MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(solver_text)
            .alignment(Alignment::Right)
            .block(solver_badge_block),
        top_chunks[2],
    );

    // 2. Conflicts Table
    let header_style = Style::default()
        .fg(COLOR_TEXT_SECONDARY)
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from("  STATE").style(header_style),
        Cell::from("COLLISION PATH").style(header_style),
        Cell::from("COMPETING MODS").style(header_style),
        Cell::from("WINNING LAYER").style(header_style),
        Cell::from("RESOLUTION TIER").style(header_style),
    ])
    .height(1);

    let mut rows = Vec::new();

    // Active collisions
    for (idx, card) in app.conflicts.iter().enumerate() {
        let is_selected = table_focused && idx == app.active_conflict_cursor;

        let (cursor_bar, status_span, path_span, competing_span, winner_span, tier_span) =
            if is_selected {
                let sel_bold = style_selected_row();
                let sel_dim = style_selected_row_secondary();

                let cursor = Span::styled("> ", sel_bold);
                let status = Span::styled(
                    "! COLLISION",
                    Style::default()
                        .bg(COLOR_FOCUS)
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                );
                let path = Span::styled(&card.file_path, sel_bold);
                let competing = Span::styled(
                    format!("{} <-> {}", card.option_a.mod_name, card.option_b.mod_name),
                    sel_dim,
                );
                let winner_text = if card.winner_id.as_deref() == Some(&card.option_a.mod_id) {
                    format!("{} (Opt A)", card.option_a.mod_name)
                } else if card.winner_id.as_deref() == Some(&card.option_b.mod_id) {
                    format!("{} (Opt B)", card.option_b.mod_name)
                } else {
                    "Unassigned (Choose a/b)".to_string()
                };
                let winner = Span::styled(winner_text, sel_bold);
                let tier = Span::styled("Tier-2 Priority", sel_dim);
                (cursor, status, path, competing, winner, tier)
            } else {
                let cursor_bar = Span::raw("  ");
                let status_span = Span::styled(
                    "! COLLISION",
                    Style::default()
                        .fg(COLOR_AMBER)
                        .add_modifier(Modifier::BOLD),
                );
                let path_style = Style::default().fg(COLOR_TEXT_PRIMARY);
                let competing_text =
                    format!("{} <-> {}", card.option_a.mod_name, card.option_b.mod_name);
                let winner_text = if card.winner_id.as_deref() == Some(&card.option_a.mod_id) {
                    format!("{} (Opt A)", card.option_a.mod_name)
                } else if card.winner_id.as_deref() == Some(&card.option_b.mod_id) {
                    format!("{} (Opt B)", card.option_b.mod_name)
                } else {
                    "Unassigned (Choose a/b)".to_string()
                };
                let winner_style = if card.winner_id.is_some() {
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(COLOR_TEXT_MUTED)
                };
                (
                    cursor_bar,
                    status_span,
                    Span::styled(&card.file_path, path_style),
                    Span::styled(competing_text, Style::default().fg(COLOR_TEXT_SECONDARY)),
                    Span::styled(winner_text, winner_style),
                    Span::styled("Tier-2 Priority", Style::default().fg(COLOR_TEXT_MUTED)),
                )
            };

        let mut row = Row::new(vec![
            Cell::from(Line::from(vec![cursor_bar, status_span])),
            Cell::from(Line::from(path_span)),
            Cell::from(Line::from(competing_span)),
            Cell::from(Line::from(winner_span)),
            Cell::from(Line::from(tier_span)),
        ])
        .height(1);

        if is_selected {
            row = row.style(style_selected_row());
        }

        rows.push(row);
    }

    // Resolved conflicts
    for res in &app.resolved_conflicts {
        let status_span = Span::styled("[RESOLVED]", Style::default().fg(COLOR_TEXT_MUTED));

        rows.push(
            Row::new(vec![
                Cell::from(Line::from(vec![Span::raw("  "), status_span])),
                Cell::from(Line::from(Span::styled(
                    &res.file_path,
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ))),
                Cell::from(Line::from(Span::styled(
                    &res.title,
                    Style::default().fg(COLOR_TEXT_MUTED),
                ))),
                Cell::from(Line::from(Span::styled(
                    "Resolved in VFS",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ))),
                Cell::from(Line::from(Span::styled(
                    "Tier-2 Rule",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ))),
            ])
            .height(1),
        );
    }

    let widths = [
        Constraint::Length(14),
        Constraint::Percentage(34),
        Constraint::Percentage(26),
        Constraint::Percentage(26),
        Constraint::Length(15),
    ];

    if rows.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "[OK] No File Collisions Detected",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Virtual File Index (VFI) is collision-free across all active OverlayFS layers.",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )]),
        ];
        f.render_widget(
            Paragraph::new(empty_lines).alignment(Alignment::Center),
            vertical_layout[1],
        );
    } else {
        let table = Table::new(rows, widths).header(header).column_spacing(2);

        let mut state = TableState::default();
        state.select(Some(app.active_conflict_cursor));
        f.render_stateful_widget(table, vertical_layout[1], &mut state);
    }

    // 3. Bottom Preview Bar
    let divider = Line::from(Span::styled(
        "─".repeat(vertical_layout[2].width as usize),
        Style::default().fg(COLOR_SEPARATOR),
    ));
    f.render_widget(Paragraph::new(divider), vertical_layout[2]);

    let bottom_spans = vec![
        Span::styled(
            "WATERFALL RESOLVER",
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "Explicit Edge (Tier 1) -> Priority Weight (Tier 2) -> Hard Conflict (Tier 3)",
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ),
        Span::raw("   |   "),
        Span::styled(
            "[a] Option A   [b] Option B   [r] Auto-Resolve   [i] Toggle Inspector",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(bottom_spans)), vertical_layout[3]);
}

fn render_conflicts_inspector_pane(f: &mut Frame, app: &App, area: Rect) {
    let a_focused = app.conflicts_focus == ConflictsFocus::OptionA;
    let b_focused = app.conflicts_focus == ConflictsFocus::OptionB;

    let inspector_focused = a_focused || b_focused;
    let block = card_block(COLOR_BORDER_NORMAL)
        .title(
            Line::from(Span::styled(
                " Collision Comparison Inspector ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " [i] Hide ",
                style_block_right_label(inspector_focused),
            ))
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
        let is_a_winner = card.winner_id.as_deref() == Some(&card.option_a.mod_id);
        let is_b_winner = card.winner_id.as_deref() == Some(&card.option_b.mod_id);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Path & Instructions
                Constraint::Length(1), // Spacer
                Constraint::Length(6), // Option A Card
                Constraint::Length(1), // Spacer
                Constraint::Length(6), // Option B Card
                Constraint::Length(1), // Spacer
                Constraint::Min(3),    // Waterfall Solver Explanation Note
            ])
            .split(inner);

        // Header
        let header_lines = vec![
            Line::from(vec![
                Span::styled("VFS Path: ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &card.file_path,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "Select which package's file sits higher in the OverlayFS lowerdir stack:",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
        ];
        f.render_widget(Paragraph::new(header_lines), sections[0]);

        // Option A Card
        let border_a = if a_focused {
            COLOR_BORDER_FOCUSED
        } else {
            COLOR_BORDER_NORMAL
        };
        let opt_a_block = card_block(border_a)
            .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
            .title(
                Line::from(box_title_label("Option A [a]", a_focused)).alignment(Alignment::Left),
            );
        let inner_a = opt_a_block.inner(sections[2]);
        f.render_widget(opt_a_block, sections[2]);

        let a_badge = if is_a_winner {
            Span::styled(
                "[x] Winning Layer",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else if a_focused {
            Span::styled(
                "> [Enter / a] Select",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("[ ] [a] Select", Style::default().fg(COLOR_TEXT_MUTED))
        };

        let a_lines = vec![
            Line::from(vec![
                Span::styled(
                    &card.option_a.mod_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                a_badge,
            ]),
            Line::from(""),
            Line::from(Span::styled(
                &card.option_a.description,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
        ];
        f.render_widget(Paragraph::new(a_lines).wrap(Wrap { trim: true }), inner_a);

        // Option B Card
        let border_b = if b_focused {
            COLOR_BORDER_FOCUSED
        } else {
            COLOR_BORDER_NORMAL
        };
        let opt_b_block = card_block(border_b)
            .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
            .title(
                Line::from(box_title_label("Option B [b]", b_focused)).alignment(Alignment::Left),
            );
        let inner_b = opt_b_block.inner(sections[4]);
        f.render_widget(opt_b_block, sections[4]);

        let b_badge = if is_b_winner {
            Span::styled(
                "[x] Winning Layer",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else if b_focused {
            Span::styled(
                "> [Enter / b] Select",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("[ ] [b] Select", Style::default().fg(COLOR_TEXT_MUTED))
        };

        let b_lines = vec![
            Line::from(vec![
                Span::styled(
                    &card.option_b.mod_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                b_badge,
            ]),
            Line::from(""),
            Line::from(Span::styled(
                &card.option_b.description,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
        ];
        f.render_widget(Paragraph::new(b_lines).wrap(Wrap { trim: true }), inner_b);

        // Solver explanation note
        let note_lines = vec![
            Line::from(vec![
                Span::styled(
                    "3-Tier Resolution Rule: ",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::styled(
                    "Winner package will be placed higher in lowerdir stack.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "[a]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" or ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "[b]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to choose. Press ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "[r]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to auto-resolve via priority.",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(note_lines).wrap(Wrap { trim: false }),
            sections[6],
        );
    } else {
        let p = Paragraph::new("No active collision selected")
            .style(Style::default().fg(COLOR_TEXT_MUTED));
        f.render_widget(p, inner);
    }
}
