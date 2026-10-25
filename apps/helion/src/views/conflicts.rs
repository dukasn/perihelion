use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::{
    action::ConflictsFocus,
    app::App,
    theme::{
        card_block, keycap, panel_block, pill, pill_solid, COLOR_AMBER, COLOR_AMBER_BG,
        COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL, COLOR_CYAN, COLOR_CYAN_BG, COLOR_GREEN,
        COLOR_GREEN_BG, COLOR_KEYCAP_BG, COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
        COLOR_TEXT_SECONDARY,
    },
};

pub fn render_conflicts_view(f: &mut Frame, app: &App, area: Rect) {
    let main_block = panel_block(true)
        .title(
            Line::from(Span::styled(
                " ◆ Virtual File Index (VFI) Conflict Resolver ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " 3-TIER WATERFALL SOLVER ",
                Style::default()
                    .fg(COLOR_AMBER)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        );
    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header & auto-resolve CTA
            Constraint::Length(1),  // Divider
            Constraint::Length(12), // Active Conflict Card
            Constraint::Length(1),  // Spacing
            Constraint::Min(6),     // Resolved Conflicts list
        ])
        .split(inner);

    // 1. Header & Auto-Resolve Selectable Button
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(42)])
        .split(chunks[0]);

    let title_lines = vec![
        Line::from(Span::styled(
            "File-Level Collision Resolution",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "When two packages supply the same VFS path, choose which layer sits higher in OverlayFS.",
            Style::default().fg(COLOR_TEXT_SECONDARY),
        )),
    ];
    f.render_widget(Paragraph::new(title_lines), header_chunks[0]);

    let auto_focused = app.conflicts_focus == ConflictsFocus::AutoResolve;
    let auto_btn_span = if auto_focused {
        pill_solid(
            ">> [r / Enter] Auto-Resolve with Priority (FOCUSED)",
            COLOR_CYAN,
        )
    } else {
        pill(
            "[r] Auto-Resolve with Priority",
            COLOR_CYAN,
            COLOR_CYAN_BG,
            true,
        )
    };
    let auto_btn = Paragraph::new(Line::from(vec![auto_btn_span])).alignment(Alignment::Right);
    f.render_widget(auto_btn, header_chunks[1]);

    let sep_line = Line::from(Span::styled(
        "─".repeat(chunks[1].width as usize),
        Style::default().fg(COLOR_SEPARATOR),
    ));
    f.render_widget(Paragraph::new(sep_line), chunks[1]);

    // 2. Active Conflict Card
    if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
        let card_container = card_block(COLOR_AMBER).title(
            Line::from(vec![
                Span::raw(" "),
                pill("! ACTIVE COLLISION", COLOR_AMBER, COLOR_AMBER_BG, true),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        );
        let card_inner = card_container.inner(chunks[2]);
        f.render_widget(card_container, chunks[2]);

        let card_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title & VFS path
                Constraint::Length(1), // Spacing
                Constraint::Min(6),    // Side-by-side options A & B
            ])
            .split(card_inner);

        let title_line = Line::from(vec![
            Span::styled(
                &card.title,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(COLOR_BORDER_NORMAL)),
            pill(&card.file_path, COLOR_CYAN, COLOR_CYAN_BG, true),
        ]);
        f.render_widget(Paragraph::new(title_line), card_sections[0]);

        // Side-by-side choices with 2-col gap
        let option_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(49),
                Constraint::Length(2),
                Constraint::Percentage(49),
            ])
            .split(card_sections[2]);

        let is_a_winner = card.winner_id.as_deref() == Some(&card.option_a.mod_id);
        let is_b_winner = card.winner_id.as_deref() == Some(&card.option_b.mod_id);
        let a_focused = app.conflicts_focus == ConflictsFocus::OptionA;
        let b_focused = app.conflicts_focus == ConflictsFocus::OptionB;

        // Option A Card
        let border_a = if a_focused {
            COLOR_BORDER_FOCUSED
        } else if is_a_winner {
            COLOR_GREEN
        } else {
            COLOR_BORDER_NORMAL
        };
        let opt_a_block = card_block(border_a).title(Line::from(vec![
            Span::raw(" "),
            keycap("a"),
            Span::styled(
                if a_focused {
                    " Option A (FOCUSED) "
                } else {
                    " Option A "
                },
                Style::default()
                    .fg(if a_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_SECONDARY
                    })
                    .add_modifier(if a_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]));
        let inner_a = opt_a_block.inner(option_chunks[0]);
        f.render_widget(opt_a_block, option_chunks[0]);

        let a_badge = if is_a_winner {
            pill_solid("● WINNING LAYER", COLOR_GREEN)
        } else if a_focused {
            pill(
                ">> PRESS [SPACE/ENTER] TO SELECT",
                COLOR_CYAN,
                COLOR_CYAN_BG,
                true,
            )
        } else {
            pill(
                "○ PRESS [a] TO SELECT",
                COLOR_TEXT_MUTED,
                COLOR_KEYCAP_BG,
                false,
            )
        };
        let a_lines = vec![
            Line::from(vec![
                Span::styled(
                    &card.option_a.mod_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
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
        } else if is_b_winner {
            COLOR_GREEN
        } else {
            COLOR_BORDER_NORMAL
        };
        let opt_b_block = card_block(border_b).title(Line::from(vec![
            Span::raw(" "),
            keycap("b"),
            Span::styled(
                if b_focused {
                    " Option B (FOCUSED) "
                } else {
                    " Option B "
                },
                Style::default()
                    .fg(if b_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_SECONDARY
                    })
                    .add_modifier(if b_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]));
        let inner_b = opt_b_block.inner(option_chunks[2]);
        f.render_widget(opt_b_block, option_chunks[2]);

        let b_badge = if is_b_winner {
            pill_solid("● WINNING LAYER", COLOR_GREEN)
        } else if b_focused {
            pill(
                ">> PRESS [SPACE/ENTER] TO SELECT",
                COLOR_CYAN,
                COLOR_CYAN_BG,
                true,
            )
        } else {
            pill(
                "○ PRESS [b] TO SELECT",
                COLOR_TEXT_MUTED,
                COLOR_KEYCAP_BG,
                false,
            )
        };
        let b_lines = vec![
            Line::from(vec![
                Span::styled(
                    &card.option_b.mod_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                b_badge,
            ]),
            Line::from(""),
            Line::from(Span::styled(
                &card.option_b.description,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
        ];
        f.render_widget(Paragraph::new(b_lines).wrap(Wrap { trim: true }), inner_b);
    }

    // 3. Resolved Conflicts list
    let resolved_focused = app.conflicts_focus == ConflictsFocus::ResolvedList;
    let res_border = if resolved_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };
    let res_block = card_block(res_border).title(
        Line::from(Span::styled(
            if resolved_focused {
                " ◆ Resolved by Tier-2 Priority Weight (FOCUSED) "
            } else {
                " ◆ Resolved by Tier-2 Priority Weight "
            },
            Style::default()
                .fg(if resolved_focused {
                    COLOR_CYAN
                } else {
                    COLOR_TEXT_MUTED
                })
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Left),
    );
    let res_inner = res_block.inner(chunks[4]);
    f.render_widget(res_block, chunks[4]);

    let mut resolved_lines = Vec::new();
    for res in &app.resolved_conflicts {
        resolved_lines.push(Line::from(vec![
            pill("+ RESOLVED", COLOR_GREEN, COLOR_GREEN_BG, true),
            Span::raw("  "),
            Span::styled(
                &res.title,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(COLOR_BORDER_NORMAL)),
            Span::styled(&res.file_path, Style::default().fg(COLOR_CYAN)),
        ]));
        resolved_lines.push(Line::from(vec![
            Span::styled("   ╰─ ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &res.resolution_note,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]));
    }

    let resolved_widget = Paragraph::new(resolved_lines).wrap(Wrap { trim: false });
    f.render_widget(resolved_widget, res_inner);
}
