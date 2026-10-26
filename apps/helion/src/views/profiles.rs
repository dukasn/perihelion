use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::{
    action::{InputValidation, ProfilesFocus},
    app::App,
    theme::{
        box_title_label, card_block, input_block, panel_block, style_block_right_label,
        style_ghost_text, style_placeholder, style_selected_row, style_selected_row_secondary,
        COLOR_AMBER, COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL, COLOR_FOCUS, COLOR_RED,
        COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_profiles_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(56),
            Constraint::Length(1), // Breathing gap between panes
            Constraint::Percentage(44),
        ])
        .split(area);

    render_profiles_list_pane(f, app, chunks[0]);
    render_profile_inspector_pane(f, app, chunks[2]);
}

fn render_profiles_list_pane(f: &mut Frame, app: &App, area: Rect) {
    let list_focused = app.profiles_focus == ProfilesFocus::ProfileList;
    let main_block = panel_block(false)
        .title(
            Line::from(Span::styled(
                " Multi-Game Workspaces ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::styled(
                    format!(" {} Profiles ", app.profiles.len()),
                    style_block_right_label(list_focused),
                ),
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );
    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // New Profile Input + Target Game Selector
            Constraint::Length(1), // Divider
            Constraint::Min(10),   // Profiles list
        ])
        .split(inner);

    // 1. Top Toolbar: Create Profile Input (Left) + Target Game Selector (Right)
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(28),
            Constraint::Length(1),
            Constraint::Length(38),
        ])
        .split(chunks[0]);

    let validation = app.validate_profile_input();
    let has_error = matches!(validation, InputValidation::Invalid(_));
    let is_valid = matches!(validation, InputValidation::Valid(_));
    let input_focused =
        app.profile_input_active || app.profiles_focus == ProfilesFocus::NewProfileInput;

    let badge = match &validation {
        InputValidation::Idle => {
            if app.profile_input_active {
                Span::styled("[Esc] Done", style_block_right_label(true))
            } else if input_focused {
                Span::styled("[Enter] to edit", style_block_right_label(true))
            } else {
                Span::styled("[n] New", style_block_right_label(false))
            }
        }
        InputValidation::Valid(msg) => Span::styled(
            format!("[OK] {}", msg),
            style_block_right_label(input_focused),
        ),
        InputValidation::Warning(msg) => Span::styled(
            format!("! {}", msg),
            Style::default()
                .fg(COLOR_AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        InputValidation::Invalid(msg) => Span::styled(
            format!("[X] {}", msg),
            Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
        ),
    };

    let suggestion = if app.profile_input_active {
        crate::autocomplete::Autocomplete::suggest_profile_name(&app.profile_input, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            box_title_label("+ New Profile Name", input_focused),
            Span::raw(" "),
            Span::styled(
                format!("Tab/-> Complete: {}", s.completed_text),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(box_title_label(
            if app.profile_input_active {
                "+ New Profile Name (Editing)"
            } else {
                "+ New Profile Name [n]"
            },
            input_focused,
        ))
    };

    let prof_input_box = input_block(input_focused, has_error, is_valid)
        .title(title_line.alignment(Alignment::Left))
        .title(Line::from(vec![Span::raw(" "), badge, Span::raw(" ")]).alignment(Alignment::Right));

    let input_inner = prof_input_box.inner(top_chunks[0]);
    f.render_widget(prof_input_box, top_chunks[0]);

    let input_line = if app.profile_input_active {
        let mut spans = vec![
            Span::styled(
                "> ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.profile_input,
                Style::default()
                    .fg(if has_error {
                        COLOR_RED
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(COLOR_FOCUS)),
        ];
        if let Some(s) = &suggestion {
            spans.push(Span::styled(&s.ghost_suffix, style_ghost_text()));
        }
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Type profile name (press 'n' to focus)...",
                style_placeholder(),
            ),
        ])
    };
    f.render_widget(Paragraph::new(input_line), input_inner);

    // Target Game Selector Box
    let game_sel_focused = app.profiles_focus == ProfilesFocus::GameSelector;
    let game_box_border = if game_sel_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };
    let game_box = card_block(game_box_border)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
        .title(
            Line::from(box_title_label(
                if game_sel_focused {
                    "Target Game [<-/-> or t]"
                } else {
                    "Target Game [t]"
                },
                game_sel_focused,
            ))
            .alignment(Alignment::Right),
        );
    let game_box_inner = game_box.inner(top_chunks[2]);
    f.render_widget(game_box, top_chunks[2]);

    if let Some(target_game) = app.game_catalog.get(app.new_profile_game_idx) {
        let game_line = Line::from(vec![
            Span::styled("< ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &target_game.short_name,
                Style::default()
                    .fg(if game_sel_focused {
                        COLOR_FOCUS
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" > ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                if target_game.steam_app_id > 0 {
                    format!("AppID {}", target_game.steam_app_id)
                } else {
                    "Native / Custom".to_string()
                },
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]);
        f.render_widget(
            Paragraph::new(game_line).alignment(Alignment::Center),
            game_box_inner,
        );
    }

    let sep_line = Line::from(Span::styled(
        "─".repeat(chunks[1].width as usize),
        Style::default().fg(COLOR_SEPARATOR),
    ));
    f.render_widget(Paragraph::new(sep_line), chunks[1]);

    // 2. Profiles List
    let list_focused =
        !app.profile_input_active && app.profiles_focus == ProfilesFocus::ProfileList;

    if app.profiles.is_empty() {
        let empty_p = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No game profiles configured yet.",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Type a profile name above, select target game with ",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
                Span::styled(
                    "[< / >]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(", and press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[Enter]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to create one.", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
        ])
        .wrap(Wrap { trim: false });
        f.render_widget(empty_p, chunks[2]);
    } else {
        let profile_constraints = vec![Constraint::Length(4); app.profiles.len()];
        let prof_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(profile_constraints)
            .split(chunks[2]);

        for (idx, profile) in app.profiles.iter().enumerate() {
            if idx >= prof_layout.len() {
                break;
            }

            let is_selected = idx == app.profile_cursor;
            let is_focused_card = is_selected && list_focused;
            let border_color = if is_focused_card {
                COLOR_BORDER_FOCUSED
            } else {
                COLOR_BORDER_NORMAL
            };

            let block = card_block(border_color);
            let p_inner = block.inner(prof_layout[idx]);
            f.render_widget(block, prof_layout[idx]);

            let lines = if is_focused_card {
                let sel_bold = style_selected_row();
                let sel_dim = style_selected_row_secondary();
                let state_str = if profile.is_active {
                    "[x] Active"
                } else {
                    "> [Enter] Activate"
                };
                let pkgs_str = format!("{} pkgs", profile.mod_count);
                let used_len = 2
                    + state_str.len()
                    + 2
                    + profile.game_short.len()
                    + 2
                    + profile.name.len()
                    + 2
                    + pkgs_str.len()
                    + 2
                    + profile.proton_version.len();
                let trailing_spaces = (p_inner.width as usize).saturating_sub(used_len);
                let trailing_fill = " ".repeat(trailing_spaces);

                vec![
                    Line::from(vec![
                        Span::styled("> ", sel_bold),
                        Span::styled(state_str, sel_bold),
                        Span::styled("  ", sel_bold),
                        Span::styled(&profile.game_short, sel_bold),
                        Span::styled("  ", sel_bold),
                        Span::styled(&profile.name, sel_bold),
                        Span::styled("  ", sel_bold),
                        Span::styled(pkgs_str, sel_dim),
                        Span::styled("  ", sel_dim),
                        Span::styled(&profile.proton_version, sel_dim),
                        Span::styled(trailing_fill, sel_bold),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&profile.desc, Style::default().fg(COLOR_TEXT_SECONDARY)),
                    ]),
                ]
            } else {
                let state_tag = if profile.is_active {
                    Span::styled(
                        "[x] Active",
                        Style::default()
                            .fg(COLOR_TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] Standby", Style::default().fg(COLOR_TEXT_MUTED))
                };

                vec![
                    Line::from(vec![
                        Span::raw("  "),
                        state_tag,
                        Span::raw("  "),
                        Span::styled(
                            &profile.game_short,
                            Style::default().fg(COLOR_TEXT_SECONDARY),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            &profile.name,
                            Style::default()
                                .fg(COLOR_TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("{} pkgs", profile.mod_count),
                            Style::default().fg(COLOR_TEXT_MUTED),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            &profile.proton_version,
                            Style::default().fg(COLOR_TEXT_MUTED),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&profile.desc, Style::default().fg(COLOR_TEXT_SECONDARY)),
                    ]),
                ]
            };
            f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), p_inner);
        }
    }
}

fn render_profile_inspector_pane(f: &mut Frame, app: &App, area: Rect) {
    let inspector_focused = app.profiles_focus == ProfilesFocus::Inspector;
    let border_col = if inspector_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let block = card_block(border_col)
        .title(
            Line::from(box_title_label(
                "Target & Profile Inspector",
                inspector_focused,
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " [Tab] Focus Inspector ",
                style_block_right_label(inspector_focused),
            ))
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(prof) = app.selected_profile() {
        let rule_str = "─".repeat(inner.width as usize);

        let status_pill = if prof.is_active {
            Span::styled(
                "[x] Mounted Workspace",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "[ ] Inactive Profile",
                Style::default().fg(COLOR_TEXT_MUTED),
            )
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    &prof.name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                status_pill,
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(&prof.game_id, Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled("  |  ", Style::default().fg(COLOR_BORDER_NORMAL)),
                Span::styled(
                    if prof.steam_app_id > 0 {
                        format!("Steam AppID {}", prof.steam_app_id)
                    } else {
                        "Native / Custom Game".to_string()
                    },
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::styled("  |  ", Style::default().fg(COLOR_BORDER_NORMAL)),
                Span::styled(&prof.proton_version, Style::default().fg(COLOR_TEXT_MUTED)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                rule_str.clone(),
                Style::default().fg(COLOR_SEPARATOR),
            )),
            Line::from(Span::styled(
                "GAME TARGET & SANDBOX PATHS",
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Game Title   ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &prof.game_full_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Entry Binary ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &prof.executable,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Base Game RO ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(&prof.install_dir, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
            Line::from(vec![
                Span::styled("Upperdir RW  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(&prof.upper_dir, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
            Line::from(vec![
                Span::styled("Isolation    ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "CLONE_NEWNS | CLONE_NEWUSER (Immutable RO Base)",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                rule_str.clone(),
                Style::default().fg(COLOR_SEPARATOR),
            )),
            Line::from(Span::styled(
                format!("VFS LAYOUT PREFIX MAPPINGS ({} RULES)", prof.mappings.len()),
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        let total_maps = prof.mappings.len();
        for (idx, m) in prof.mappings.iter().enumerate() {
            let connector = if idx + 1 == total_maps {
                "╰─ "
            } else {
                "├─ "
            };
            lines.push(Line::from(vec![
                Span::styled(connector, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    format!("{:<8}", format!("{}/*", m.from_prefix)),
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" --> ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    format!("{}/*", m.to_prefix),
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        )));
        lines.push(Line::from(Span::styled(
            "ENVIRONMENT & RUNTIME DIAGNOSTICS",
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let diag = &prof.diagnostic;

        let action_badge = if diag.resolved {
            Span::styled("[OK] Resolved", Style::default().fg(COLOR_TEXT_MUTED))
        } else if inspector_focused {
            Span::styled(
                "> [Enter / f] Apply Fix",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(&diag.action_text, Style::default().fg(COLOR_TEXT_SECONDARY))
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("! {}", diag.level),
                Style::default().fg(if diag.resolved {
                    COLOR_TEXT_MUTED
                } else {
                    COLOR_AMBER
                }),
            ),
            Span::raw(" "),
            Span::styled(
                &diag.title,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            &diag.message,
            Style::default().fg(COLOR_TEXT_SECONDARY),
        )));
        lines.push(Line::from(vec![
            Span::styled("Action: ", Style::default().fg(COLOR_TEXT_MUTED)),
            action_badge,
        ]));

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    } else {
        let empty_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No profile selected",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Select or create a game profile to inspect environment layout and runtime diagnostics.",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )]),
        ];
        f.render_widget(
            Paragraph::new(empty_lines).wrap(Wrap { trim: false }),
            inner,
        );
    }
}
