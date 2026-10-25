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
        card_block, input_block, panel_block, pill, pill_solid, style_ghost_text,
        style_placeholder, COLOR_AMBER, COLOR_AMBER_BG, COLOR_BLUE, COLOR_BLUE_BG,
        COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL, COLOR_CYAN, COLOR_CYAN_BG, COLOR_GREEN,
        COLOR_GREEN_BG, COLOR_INDIGO, COLOR_INDIGO_BG, COLOR_KEYCAP_BG, COLOR_RED, COLOR_RED_BG,
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
    let left_focused = matches!(
        app.profiles_focus,
        ProfilesFocus::NewProfileInput | ProfilesFocus::GameSelector | ProfilesFocus::ProfileList
    );

    let focus_tag = match app.profiles_focus {
        ProfilesFocus::NewProfileInput => pill_solid("FOCUS: NEW PROFILE", COLOR_CYAN),
        ProfilesFocus::GameSelector => pill_solid("FOCUS: TARGET GAME", COLOR_CYAN),
        ProfilesFocus::ProfileList => pill_solid("FOCUS: PROFILES LIST", COLOR_CYAN),
        ProfilesFocus::Inspector => {
            Span::styled(" [Tab] Focus List ", Style::default().fg(COLOR_TEXT_MUTED))
        }
    };

    let main_block = panel_block(left_focused)
        .title(
            Line::from(Span::styled(
                " ◆ Multi-Game Workspaces (Prototype Preview) ",
                Style::default()
                    .fg(if left_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_SECONDARY
                    })
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::styled(
                    format!(" {} Profiles ", app.profiles.len()),
                    Style::default()
                        .fg(COLOR_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("│ ", Style::default().fg(COLOR_TEXT_MUTED)),
                focus_tag,
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
                pill_solid("EDITING", COLOR_CYAN)
            } else if input_focused {
                pill_solid("ENTER TO EDIT", COLOR_CYAN)
            } else {
                pill("[n] New", COLOR_TEXT_MUTED, COLOR_KEYCAP_BG, false)
            }
        }
        InputValidation::Valid(msg) => {
            pill(format!("+ {}", msg), COLOR_GREEN, COLOR_GREEN_BG, true)
        }
        InputValidation::Warning(msg) => {
            pill(format!("! {}", msg), COLOR_AMBER, COLOR_AMBER_BG, true)
        }
        InputValidation::Invalid(msg) => pill(format!("x {}", msg), COLOR_RED, COLOR_RED_BG, true),
    };

    let suggestion = if app.profile_input_active {
        crate::autocomplete::Autocomplete::suggest_profile_name(&app.profile_input, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            Span::styled(
                " + New Profile Name ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            pill(
                format!("Tab/→ Complete: {}", s.completed_text),
                COLOR_CYAN,
                COLOR_KEYCAP_BG,
                true,
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(Span::styled(
            if app.profile_input_active {
                " + New Profile Name (Editing) "
            } else {
                " + New Profile Name [n] "
            },
            Style::default()
                .fg(if has_error {
                    COLOR_RED
                } else if input_focused {
                    COLOR_CYAN
                } else {
                    COLOR_TEXT_SECONDARY
                })
                .add_modifier(Modifier::BOLD),
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
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
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
            Span::styled("█", Style::default().fg(COLOR_CYAN)),
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
            Line::from(Span::styled(
                if game_sel_focused {
                    " ◆ Target Game [←/→ or t] (Focused) "
                } else {
                    " Target Game [t] "
                },
                Style::default()
                    .fg(if game_sel_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_MUTED
                    })
                    .add_modifier(if game_sel_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ))
            .alignment(Alignment::Right),
        );
    let game_box_inner = game_box.inner(top_chunks[2]);
    f.render_widget(game_box, top_chunks[2]);

    if let Some(target_game) = app.game_catalog.get(app.new_profile_game_idx) {
        let game_line = Line::from(vec![
            Span::styled("< ", Style::default().fg(COLOR_TEXT_MUTED)),
            pill_solid(&target_game.short_name, COLOR_CYAN),
            Span::styled(" > ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!("AppID {}", target_game.steam_app_id),
                Style::default().fg(COLOR_TEXT_SECONDARY),
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
        let border_color = if is_selected && list_focused {
            COLOR_BORDER_FOCUSED
        } else if profile.is_active {
            COLOR_GREEN
        } else {
            COLOR_BORDER_NORMAL
        };

        let block = card_block(border_color);
        let p_inner = block.inner(prof_layout[idx]);
        f.render_widget(block, prof_layout[idx]);

        let state_tag = if profile.is_active {
            pill_solid("● ACTIVE", COLOR_GREEN)
        } else if is_selected {
            pill(">> [ENTER] ACTIVATE", COLOR_CYAN, COLOR_CYAN_BG, true)
        } else {
            pill("○ STANDBY", COLOR_TEXT_MUTED, COLOR_KEYCAP_BG, false)
        };

        let cursor_prefix = if is_selected {
            Span::styled(
                if list_focused { "┃ " } else { "▎ " },
                Style::default().fg(COLOR_CYAN),
            )
        } else {
            Span::raw("  ")
        };

        let game_badge_color = match profile.game_id.as_str() {
            "skyrim_se" => COLOR_BLUE,
            "cyberpunk_2077" => COLOR_AMBER,
            "starfield" => COLOR_CYAN,
            _ => COLOR_INDIGO,
        };

        let lines = vec![
            Line::from(vec![
                cursor_prefix,
                state_tag,
                Span::raw(" "),
                pill(&profile.game_short, game_badge_color, COLOR_BLUE_BG, true),
                Span::raw(" "),
                Span::styled(
                    &profile.name,
                    Style::default()
                        .fg(if is_selected {
                            COLOR_CYAN
                        } else {
                            COLOR_TEXT_PRIMARY
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                pill(
                    format!("{} pkgs", profile.mod_count),
                    COLOR_TEXT_SECONDARY,
                    COLOR_KEYCAP_BG,
                    false,
                ),
                Span::raw(" "),
                pill(
                    &profile.proton_version,
                    COLOR_INDIGO,
                    COLOR_INDIGO_BG,
                    false,
                ),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(&profile.desc, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), p_inner);
    }
}

fn render_profile_inspector_pane(f: &mut Frame, app: &App, area: Rect) {
    let inspector_focused = app.profiles_focus == ProfilesFocus::Inspector;
    let border_col = if inspector_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let right_title = if inspector_focused {
        Line::from(vec![
            pill_solid("FOCUS: INSPECTOR", COLOR_CYAN),
            Span::raw(" "),
        ])
    } else {
        Line::from(Span::styled(
            " [Tab] Focus Inspector ",
            Style::default().fg(COLOR_TEXT_MUTED),
        ))
    };

    let block = card_block(border_col)
        .title(
            Line::from(Span::styled(
                " ◆ Target & Profile Inspector (Prototype Model) ",
                Style::default()
                    .fg(if inspector_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(right_title.alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(prof) = app.selected_profile() {
        let rule_str = "─".repeat(inner.width as usize);

        let status_pill = if prof.is_active {
            pill_solid("● MOUNTED WORKSPACE", COLOR_GREEN)
        } else {
            pill(
                "○ INACTIVE PROFILE",
                COLOR_TEXT_MUTED,
                COLOR_KEYCAP_BG,
                false,
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
                pill(&prof.game_id, COLOR_CYAN, COLOR_CYAN_BG, true),
                Span::raw(" "),
                pill(
                    format!("Steam AppID {}", prof.steam_app_id),
                    COLOR_BLUE,
                    COLOR_BLUE_BG,
                    true,
                ),
                Span::raw(" "),
                pill(&prof.proton_version, COLOR_INDIGO, COLOR_INDIGO_BG, true),
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
                    Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
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
                    Style::default().fg(COLOR_GREEN),
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
                Span::styled(connector, Style::default().fg(COLOR_CYAN)),
                Span::styled(
                    format!("{:<8}", format!("{}/*", m.from_prefix)),
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ──▸ ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    format!("{}/*", m.to_prefix),
                    Style::default().fg(COLOR_GREEN),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        )));
        lines.push(Line::from(Span::styled(
            "ENVIRONMENT & RUNTIME DIAGNOSTICS (SIMULATED PREVIEW)",
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let diag = &prof.diagnostic;
        let (diag_fg, diag_bg) = if diag.resolved {
            (COLOR_GREEN, COLOR_GREEN_BG)
        } else {
            (COLOR_AMBER, COLOR_AMBER_BG)
        };

        let action_badge = if diag.resolved {
            pill(&diag.action_text, COLOR_GREEN, COLOR_GREEN_BG, true)
        } else if inspector_focused {
            pill_solid(">> [Enter / f] Apply Fix", COLOR_CYAN)
        } else {
            pill(&diag.action_text, COLOR_CYAN, COLOR_CYAN_BG, true)
        };

        lines.push(Line::from(vec![
            pill(&diag.level, diag_fg, diag_bg, true),
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
        lines.push(Line::from(vec![Span::raw("Action: "), action_badge]));

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    }
}
