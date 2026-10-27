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
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::{
    action::{InputValidation, ProfilesFocus},
    app::{App, GameWizardField},
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
                    "[g] Define Game  [s] Scan Steam  | ",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::styled(
                    format!("{} Profiles ", app.profiles.len()),
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
                "No workspace profiles created yet.",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("1. Target Game:  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Use ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[t]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" or ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[< / >]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to select target game above.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("2. Steam Import: ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[s]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to auto-detect and import installed Steam apps.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("3. Custom Game:  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("Press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[g]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to define a new game profile from scratch.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("4. New Profile:  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "Type a name above and press ",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
                Span::styled(
                    "[Enter]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to create a workspace profile.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
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
    } else if let Some(target_game) = app.selected_target_game() {
        let rule_str = "─".repeat(inner.width as usize);
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    &target_game.short_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    "[Target Specification]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    &target_game.game_id,
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
                Span::styled("  |  ", Style::default().fg(COLOR_BORDER_NORMAL)),
                Span::styled(
                    if target_game.steam_app_id > 0 {
                        format!("Steam AppID {}", target_game.steam_app_id)
                    } else {
                        "Native / Custom Game".to_string()
                    },
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::styled("  |  ", Style::default().fg(COLOR_BORDER_NORMAL)),
                Span::styled(
                    &target_game.default_proton,
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
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
                    &target_game.full_name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Entry Binary ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &target_game.executable,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Base Game RO ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &target_game.install_dir,
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
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
                format!(
                    "VFS LAYOUT PREFIX MAPPINGS ({} RULES)",
                    target_game.mappings.len()
                ),
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if target_game.mappings.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Direct 1:1 overlay without path prefix translation.",
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        } else {
            let total_maps = target_game.mappings.len();
            for (idx, m) in target_game.mappings.iter().enumerate() {
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

        let diag = &target_game.default_diagnostic;
        lines.push(Line::from(vec![
            Span::styled(
                format!("! {}", diag.level),
                Style::default().fg(COLOR_TEXT_MUTED),
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
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Hint: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Press [n] above to create a workspace profile for this game target.",
                Style::default().fg(COLOR_FOCUS),
            ),
        ]));

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    } else {
        let empty_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No profile or target game selected",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Select a target game or press [s] to scan Steam / [g] to define a new game profile.",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )]),
        ];
        f.render_widget(
            Paragraph::new(empty_lines).wrap(Wrap { trim: false }),
            inner,
        );
    }
}

pub fn render_steam_importer_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(76, 75, area);
    f.render_widget(Clear, popup_area);

    let loc_text = if app.steam_importer_is_global {
        " Target: [*] Global (~/.config) [Tab/Space to toggle] "
    } else {
        " Target: [*] Workspace (.perihelion) [Tab/Space to toggle] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(
            Style::default()
                .fg(COLOR_BORDER_FOCUSED)
                .add_modifier(Modifier::BOLD),
        )
        .title(
            Line::from(Span::styled(
                " Scan & Import Steam Apps [s] ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                loc_text,
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        )
        .padding(Padding::new(2, 2, 1, 1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rule_str = "─".repeat(inner.width as usize);

    if app.steam_games.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No installed Steam apps or content detected across configured library folders.",
                Style::default()
                    .fg(COLOR_AMBER)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Perihelion probed default Linux Steam roots (~/.local/share/Steam, Flatpak, $STEAM_DIR).",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
            Line::from(Span::styled(
                "Make sure Steam is running or apps are installed, or press [g] to define a custom game manually.",
                Style::default().fg(COLOR_TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(rule_str, Style::default().fg(COLOR_SEPARATOR))),
            Line::from(Span::styled(
                "[Esc] Close Dialog    [g] Define Game Profile Manually",
                Style::default().fg(COLOR_FOCUS),
            )),
        ];
        f.render_widget(
            Paragraph::new(empty_lines).wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Top description
            Constraint::Length(1), // Separator
            Constraint::Min(6),    // List of games
            Constraint::Length(1), // Separator
            Constraint::Length(2), // Bottom action bar
        ])
        .split(inner);

    let desc = Line::from(vec![
        Span::styled(
            format!(
                "Found {} installed Steam app{} / content.",
                app.steam_games.len(),
                if app.steam_games.len() == 1 { "" } else { "s" }
            ),
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "Select an app or binary entry and press [Enter] to auto-scaffold a declarative profile.",
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ),
    ]);
    f.render_widget(Paragraph::new(desc), chunks[0]);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        ))),
        chunks[1],
    );

    // List of games (scrollable window)
    let list_area = chunks[2];
    let row_height = 2; // 2 lines per game
    let visible_capacity = ((list_area.height as usize) / row_height).max(1);

    let scroll_offset = if app.steam_importer_cursor >= visible_capacity {
        app.steam_importer_cursor - visible_capacity + 1
    } else {
        0
    };

    let mut game_rows = Vec::new();
    for (idx, game) in app
        .steam_games
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_capacity)
    {
        let is_selected = idx == app.steam_importer_cursor;
        let prefix = if is_selected { "> " } else { "  " };

        let exe_str = game
            .executable
            .as_deref()
            .unwrap_or("No entry binary detected");

        if is_selected {
            game_rows.push(Line::from(vec![
                Span::styled(prefix, style_selected_row()),
                Span::styled(&game.name, style_selected_row()),
                Span::styled("  ", style_selected_row()),
                Span::styled(
                    format!("(AppID {})", game.app_id),
                    style_selected_row_secondary(),
                ),
                Span::styled("  ::  ", style_selected_row_secondary()),
                Span::styled(exe_str, style_selected_row_secondary()),
            ]));
            game_rows.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("Path: {}", game.install_path.display()),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ]));
        } else {
            game_rows.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    &game.name,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("(AppID {})", game.app_id),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::raw("  ::  "),
                Span::styled(exe_str, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]));
            game_rows.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("Path: {}", game.install_path.display()),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ]));
        }
    }

    f.render_widget(Paragraph::new(game_rows), list_area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        ))),
        chunks[3],
    );

    let bottom_actions = Line::from(vec![
        Span::styled(
            "[Enter]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Import Selected App   ",
            Style::default().fg(COLOR_TEXT_PRIMARY),
        ),
        Span::styled(
            "[Tab/Space]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Toggle Target Location   ",
            Style::default().fg(COLOR_TEXT_PRIMARY),
        ),
        Span::styled(
            "[Up/Down or j/k]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Navigate   ", Style::default().fg(COLOR_TEXT_PRIMARY)),
        Span::styled(
            "[Esc/q]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel", Style::default().fg(COLOR_TEXT_PRIMARY)),
    ]);
    f.render_widget(Paragraph::new(bottom_actions), chunks[4]);
}

pub fn render_game_wizard_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(74, 82, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(
            Style::default()
                .fg(COLOR_BORDER_FOCUSED)
                .add_modifier(Modifier::BOLD),
        )
        .title(
            Line::from(Span::styled(
                " Define Game Profile Wizard [g] ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " [Ctrl+S / Enter] Save  [Esc] Cancel ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        )
        .padding(Padding::new(2, 2, 1, 1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rule_str = "─".repeat(inner.width as usize);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Subtitle
            Constraint::Length(1), // Separator
            Constraint::Length(3), // Field 1: Game ID
            Constraint::Length(3), // Field 2: Name
            Constraint::Length(3), // Field 3: Steam App ID
            Constraint::Length(3), // Field 4: Executable
            Constraint::Length(3), // Field 5: Mappings
            Constraint::Length(3), // Field 6: Target Destination
            Constraint::Length(2), // Error or status hint
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Footer hint
        ])
        .split(inner);

    let subtitle = Line::from(Span::styled(
        "Define declarative profile rules for a native Linux or Wine/Proton game title.",
        Style::default().fg(COLOR_TEXT_SECONDARY),
    ));
    f.render_widget(Paragraph::new(subtitle), chunks[0]);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        ))),
        chunks[1],
    );

    let render_field =
        |f: &mut Frame, area: Rect, title: &str, val: &str, is_active: bool, placeholder: &str| {
            let border_col = if is_active {
                COLOR_BORDER_FOCUSED
            } else {
                COLOR_BORDER_NORMAL
            };
            let b = card_block(border_col)
                .title(Line::from(box_title_label(title, is_active)).alignment(Alignment::Left));
            let inner_b = b.inner(area);
            f.render_widget(b, area);

            let line = if val.is_empty() {
                if is_active {
                    Line::from(vec![
                        Span::styled("> ", Style::default().fg(COLOR_FOCUS)),
                        Span::styled("█", Style::default().fg(COLOR_FOCUS)),
                        Span::styled(placeholder, style_placeholder()),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
                        Span::styled(placeholder, style_placeholder()),
                    ])
                }
            } else {
                let mut spans = vec![
                    Span::styled(
                        "> ",
                        Style::default().fg(if is_active {
                            COLOR_FOCUS
                        } else {
                            COLOR_TEXT_MUTED
                        }),
                    ),
                    Span::styled(
                        val,
                        Style::default()
                            .fg(COLOR_TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if is_active {
                    spans.push(Span::styled("█", Style::default().fg(COLOR_FOCUS)));
                }
                Line::from(spans)
            };
            f.render_widget(Paragraph::new(line), inner_b);
        };

    // 1. Game ID
    render_field(
        f,
        chunks[2],
        "Game Identifier (slug, e.g. witcher3)",
        &app.game_wizard.game_id,
        app.game_wizard.active_field == GameWizardField::GameId,
        "Enter slugified game ID (required)...",
    );

    // 2. Name
    render_field(
        f,
        chunks[3],
        "Display Title (e.g. The Witcher 3: Wild Hunt)",
        &app.game_wizard.name,
        app.game_wizard.active_field == GameWizardField::Name,
        "Enter full human-readable game title...",
    );

    // 3. Steam App ID
    render_field(
        f,
        chunks[4],
        "Steam App ID (optional, e.g. 292030)",
        &app.game_wizard.steam_app_id,
        app.game_wizard.active_field == GameWizardField::SteamAppId,
        "Enter numerical Steam App ID or leave empty for native...",
    );

    // 4. Executable
    render_field(
        f,
        chunks[5],
        "Default Executable (e.g. bin/x64/witcher3.exe)",
        &app.game_wizard.executable,
        app.game_wizard.active_field == GameWizardField::Executable,
        "Enter binary name or relative path...",
    );

    // 5. Mappings
    render_field(
        f,
        chunks[6],
        "VFS Prefix Mappings (e.g. mods=mods, bin=bin)",
        &app.game_wizard.mappings,
        app.game_wizard.active_field == GameWizardField::Mappings,
        "Comma-separated prefix translations (leave empty for 1:1)...",
    );

    // 6. Target Destination
    let is_global_active = app.game_wizard.active_field == GameWizardField::IsGlobal;
    let dest_block = card_block(if is_global_active {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    })
    .title(
        Line::from(box_title_label(
            "Save Target Storage [Space / Left / Right]",
            is_global_active,
        ))
        .alignment(Alignment::Left),
    );
    let dest_inner = dest_block.inner(chunks[7]);
    f.render_widget(dest_block, chunks[7]);

    let dest_spans = if app.game_wizard.is_global {
        vec![
            Span::styled(
                " [*] Global Config ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "(~/.config/perihelion/profiles)  ",
                Style::default().fg(COLOR_TEXT_PRIMARY),
            ),
            Span::styled(" [ ] Workspace ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "(.perihelion/profiles)",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]
    } else {
        vec![
            Span::styled(" [ ] Global Config ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "(~/.config/perihelion/profiles)  ",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::styled(
                " [*] Workspace ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "(.perihelion/profiles)",
                Style::default().fg(COLOR_TEXT_PRIMARY),
            ),
        ]
    };
    f.render_widget(Paragraph::new(Line::from(dest_spans)), dest_inner);

    // Error or status hint
    if let Some(ref err) = app.game_wizard.error {
        let err_line = Line::from(vec![
            Span::styled(
                "! Error: ",
                Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(COLOR_RED)),
        ]);
        f.render_widget(Paragraph::new(err_line), chunks[8]);
    } else {
        let info_line = Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Global profiles are available across all workspaces on this system.",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]);
        f.render_widget(Paragraph::new(info_line), chunks[8]);
    }

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        ))),
        chunks[9],
    );

    let footer_hints = Line::from(vec![
        Span::styled(
            "[Tab/Down]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Next Field   ", Style::default().fg(COLOR_TEXT_PRIMARY)),
        Span::styled(
            "[BackTab/Up]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Prev Field   ", Style::default().fg(COLOR_TEXT_PRIMARY)),
        Span::styled(
            "[Ctrl+S]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Save Profile   ", Style::default().fg(COLOR_TEXT_PRIMARY)),
        Span::styled(
            "[Esc]",
            Style::default()
                .fg(COLOR_FOCUS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel", Style::default().fg(COLOR_TEXT_PRIMARY)),
    ]);
    f.render_widget(Paragraph::new(footer_hints), chunks[10]);
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
