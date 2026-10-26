use peri_core::manifest::Priority;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use crate::{
    action::{InputValidation, ModsFocus},
    app::App,
    theme::{
        box_title_label, card_block, input_block, keycap, panel_block, style_block_right_label,
        style_ghost_text, style_placeholder, style_selected_row, style_selected_row_secondary,
        COLOR_AMBER, COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL, COLOR_FOCUS, COLOR_RED,
        COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

fn priority_label(priority: Priority) -> &'static str {
    match priority {
        Priority::Lowest => "= Lowest",
        Priority::Low => "v Low",
        Priority::Default => "* Default",
        Priority::High => "^ High",
        Priority::Highest => "^^ Highest",
        Priority::Absolute => "!! Absolute",
    }
}

pub fn render_mods_view(f: &mut Frame, app: &App, area: Rect) {
    if app.show_inspector {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(68),
                Constraint::Length(1), // Breathing gap between panes
                Constraint::Percentage(32),
            ])
            .split(area);

        render_table_pane(f, app, chunks[0]);
        render_inspector_pane(f, app, chunks[2]);
    } else {
        render_table_pane(f, app, area);
    }
}

fn render_table_pane(f: &mut Frame, app: &App, area: Rect) {
    let active_count = app.mods.iter().filter(|m| m.active).count();
    let table_focused = app.mods_focus == ModsFocus::Table;

    let block = panel_block(false)
        .title(
            Line::from(Span::styled(
                " Mod Load Order & Priority ",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::styled(
                    format!(" {} Active ", active_count),
                    style_block_right_label(table_focused),
                ),
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input Box & Category Filter Toolbar
            Constraint::Min(5),    // Mods Table (or Empty Filter State)
            Constraint::Length(1), // Bottom Divider Line
            Constraint::Length(1), // Live OverlayFS Stack Preview
        ])
        .split(inner);

    // 1. Search Input Box (left) + Category Filter Box (right)
    let filter_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(32),
            Constraint::Length(2),
            Constraint::Length(46),
        ])
        .split(vertical_layout[0]);

    let validation = app.search_validation();
    let has_err = matches!(
        validation,
        InputValidation::Invalid(_) | InputValidation::Warning(_)
    );
    let is_valid = matches!(validation, InputValidation::Valid(_));
    let search_focused = app.search_active || app.mods_focus == ModsFocus::Search;

    let validation_badge = match &validation {
        InputValidation::Idle => {
            if search_focused {
                Span::styled("[Esc] Done", style_block_right_label(true))
            } else {
                Span::styled("[/] Filter", style_block_right_label(false))
            }
        }
        InputValidation::Valid(msg) => Span::styled(
            format!("[OK] {}", msg),
            style_block_right_label(search_focused),
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

    let suggestion = if search_focused {
        crate::autocomplete::Autocomplete::suggest_mod_search(&app.search_query, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            box_title_label("[/] Filter Input", search_focused),
            Span::raw(" "),
            Span::styled(
                format!("Tab/-> Complete: {}", s.completed_text),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(box_title_label("[/] Filter Input", search_focused))
    };

    let search_box = input_block(search_focused, has_err, is_valid)
        .title(title_line.alignment(Alignment::Left))
        .title(
            Line::from(vec![Span::raw(" "), validation_badge, Span::raw(" ")])
                .alignment(Alignment::Right),
        );

    let search_inner = search_box.inner(filter_chunks[0]);
    f.render_widget(search_box, filter_chunks[0]);

    let search_content = if search_focused {
        let mut spans = vec![
            Span::styled(
                "> ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(COLOR_FOCUS)),
        ];
        if let Some(s) = &suggestion {
            spans.push(Span::styled(&s.ghost_suffix, style_ghost_text()));
        }
        Line::from(spans)
    } else if app.search_query.is_empty() {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Type query (press '/' or Tab to focus)...",
                style_placeholder(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (Esc to clear)", style_placeholder()),
        ])
    };
    f.render_widget(Paragraph::new(search_content), search_inner);

    // Category Filter Box on Right
    let cats_focused = app.mods_focus == ModsFocus::Categories;
    let cat_border_col = if cats_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };
    let cat_block = card_block(cat_border_col)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
        .title(
            Line::from(box_title_label(
                if cats_focused {
                    "Categories [<-/->/Space]"
                } else {
                    "Categories [c]"
                },
                cats_focused,
            ))
            .alignment(Alignment::Right),
        );
    let cat_inner = cat_block.inner(filter_chunks[2]);
    f.render_widget(cat_block, filter_chunks[2]);

    let categories = ["All", "Fixes", "UI", "Gameplay", "Visuals"];
    let mut category_spans = Vec::new();
    for cat in categories {
        let is_selected = match &app.category_filter {
            None => cat == "All",
            Some(c) => c.eq_ignore_ascii_case(cat),
        };

        if is_selected {
            category_spans.push(Span::styled(
                format!(" [{}] ", cat),
                Style::default()
                    .fg(if cats_focused {
                        COLOR_FOCUS
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            category_spans.push(Span::styled(
                format!(" {} ", cat),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        category_spans.push(Span::raw(" "));
    }
    let cat_p = Paragraph::new(Line::from(category_spans)).alignment(Alignment::Right);
    f.render_widget(cat_p, cat_inner);

    // 2. Mods Table (or Empty Filter Notice)
    let filtered = app.filtered_mods();
    if filtered.is_empty() {
        let empty_lines = if app.mods.is_empty() {
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "No packages installed in this workspace yet.",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                    keycap("4"),
                    Span::styled(
                        " to open Downloads, or type a package URL / path in ",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                    keycap("/"),
                    Span::styled(" Quick Command.", Style::default().fg(COLOR_TEXT_SECONDARY)),
                ]),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "! NO MATCHING PACKAGES",
                        Style::default()
                            .fg(COLOR_AMBER)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!(
                            "No mods matched query '{}' in category '{}'.",
                            if app.search_query.is_empty() {
                                "*"
                            } else {
                                &app.search_query
                            },
                            app.category_filter.as_deref().unwrap_or("All")
                        ),
                        Style::default()
                            .fg(COLOR_TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                    keycap("Esc"),
                    Span::styled(" or ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                    keycap("Ctrl+L"),
                    Span::styled(
                        " to clear search and category filters.",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                ]),
            ]
        };
        f.render_widget(
            Paragraph::new(empty_lines).alignment(Alignment::Center),
            vertical_layout[1],
        );
    } else {
        let header_style = Style::default()
            .fg(COLOR_TEXT_SECONDARY)
            .add_modifier(Modifier::BOLD);

        let header = Row::new(vec![
            Cell::from("  STATE").style(header_style),
            Cell::from("ORDER").style(header_style),
            Cell::from("PACKAGE NAME").style(header_style),
            Cell::from("CATEGORY").style(header_style),
            Cell::from("PRIORITY TIER").style(header_style),
            Cell::from("VERSION").style(header_style),
        ])
        .height(1);

        let rows = filtered.iter().enumerate().map(|(ui_idx, (_, mod_item))| {
            let is_selected = table_focused && ui_idx == app.cursor_index;

            let (cursor_bar, status_badge, idx_span, name_spans, cat_span, prio_span, ver_span) =
                if is_selected {
                    let sel_bold = style_selected_row();
                    let sel_dim = style_selected_row_secondary();

                    let cursor = Span::styled("> ", sel_bold);
                    let status = if mod_item.active {
                        Span::styled("[x] on ", sel_bold)
                    } else {
                        Span::styled("[ ] off", sel_dim)
                    };
                    let idx = Span::styled(format!("#{:02}", ui_idx + 1), sel_bold);
                    let mut name_spans = vec![Span::styled(&mod_item.name, sel_bold)];
                    if mod_item.has_conflict {
                        name_spans.push(Span::raw("  "));
                        name_spans.push(Span::styled(
                            "! CONFLICT",
                            Style::default()
                                .bg(COLOR_RED)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    let cat = Span::styled(&mod_item.category, sel_dim);
                    let prio = Span::styled(priority_label(mod_item.priority), sel_dim);
                    let ver = Span::styled(format!("v{}", mod_item.version), sel_dim);
                    (cursor, status, idx, name_spans, cat, prio, ver)
                } else {
                    let cursor_bar = Span::raw("  ");
                    let status_badge = if mod_item.active {
                        Span::styled(
                            "[x] on ",
                            Style::default()
                                .fg(COLOR_TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled("[ ] off", Style::default().fg(COLOR_TEXT_MUTED))
                    };
                    let idx_str = format!("#{:02}", ui_idx + 1);
                    let idx_span = Span::styled(idx_str, Style::default().fg(COLOR_TEXT_MUTED));
                    let name_style = if mod_item.active {
                        Style::default().fg(COLOR_TEXT_PRIMARY)
                    } else {
                        Style::default().fg(COLOR_TEXT_MUTED)
                    };
                    let mut name_spans = vec![Span::styled(&mod_item.name, name_style)];
                    if mod_item.has_conflict {
                        name_spans.push(Span::raw("  "));
                        name_spans.push(Span::styled(
                            "! CONFLICT",
                            Style::default()
                                .fg(COLOR_AMBER)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    let cat_span = Span::styled(
                        &mod_item.category,
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    );
                    let prio_span = Span::styled(
                        priority_label(mod_item.priority),
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    );
                    let ver_span = Span::styled(
                        format!("v{}", mod_item.version),
                        Style::default().fg(COLOR_TEXT_MUTED),
                    );
                    (
                        cursor_bar,
                        status_badge,
                        idx_span,
                        name_spans,
                        cat_span,
                        prio_span,
                        ver_span,
                    )
                };

            let mut row = Row::new(vec![
                Cell::from(Line::from(vec![cursor_bar, status_badge])),
                Cell::from(Line::from(idx_span)),
                Cell::from(Line::from(name_spans)),
                Cell::from(Line::from(cat_span)),
                Cell::from(Line::from(prio_span)),
                Cell::from(Line::from(ver_span)),
            ])
            .height(1);

            if is_selected {
                row = row.style(style_selected_row());
            }

            row
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Min(28),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(9),
        ];

        let table = Table::new(rows, widths).header(header).column_spacing(2);

        let mut state = TableState::default();
        state.select(Some(app.cursor_index));
        f.render_stateful_widget(table, vertical_layout[1], &mut state);
    }

    // 3. Bottom OverlayFS Stack Preview Bar
    let divider = Line::from(Span::styled(
        "─".repeat(vertical_layout[2].width as usize),
        Style::default().fg(COLOR_SEPARATOR),
    ));
    f.render_widget(Paragraph::new(divider), vertical_layout[2]);

    let mut active_layers: Vec<_> = app.mods.iter().filter(|m| m.active).collect();
    // OverlayFS lowerdir order: highest priority first (topmost layer) -> lowest priority -> base_game
    active_layers.sort_by_key(|a| std::cmp::Reverse(a.priority));

    let selected_id = app.selected_mod().map(|m| m.id.as_str());
    let mut stack_spans = vec![
        Span::styled(
            "LOWERDIR",
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

    for (idx, layer) in active_layers.iter().enumerate() {
        let is_focused_layer = selected_id == Some(layer.id.as_str());
        if is_focused_layer {
            stack_spans.push(Span::styled(
                &layer.id,
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            stack_spans.push(Span::styled(
                &layer.id,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ));
        }
        if idx + 1 < active_layers.len() {
            stack_spans.push(Span::styled(" -> ", Style::default().fg(COLOR_TEXT_MUTED)));
        }
    }
    stack_spans.push(Span::styled(" -> ", Style::default().fg(COLOR_TEXT_MUTED)));
    stack_spans.push(Span::styled(
        "base_game (ro)",
        Style::default().fg(COLOR_TEXT_MUTED),
    ));

    f.render_widget(Paragraph::new(Line::from(stack_spans)), vertical_layout[3]);
}

fn render_inspector_pane(f: &mut Frame, app: &App, area: Rect) {
    let inspector_focused = app.mods_focus == ModsFocus::Inspector;
    let border_color = if inspector_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let block = card_block(border_color)
        .title(
            Line::from(box_title_label("Package Inspector", inspector_focused))
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

    if let Some(selected) = app.selected_mod() {
        let rule_str = "─".repeat(inner.width as usize);

        let status_pill = if selected.active {
            Span::styled(
                "[x] Active Layer",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("[ ] Disabled", Style::default().fg(COLOR_TEXT_MUTED))
        };

        let mut lines = vec![
            Line::from(Span::styled(
                &selected.name,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                status_pill,
                Span::raw("  "),
                Span::styled(
                    format!("v{}", selected.version),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::raw("  "),
                Span::styled(
                    priority_label(selected.priority),
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                &selected.description,
                Style::default().fg(COLOR_TEXT_SECONDARY),
            )),
            Line::from(""),
            Line::from(Span::styled(
                rule_str.clone(),
                Style::default().fg(COLOR_SEPARATOR),
            )),
            Line::from(Span::styled(
                "PACKAGE METADATA",
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Package ID  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &selected.id,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Category    ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &selected.category,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Author      ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(&selected.author, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
            Line::from(vec![
                Span::styled("Mount Mode  ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "OverlayFS Read-Only Lowerdir",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
        ];

        if selected.has_conflict {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "! VFI COLLISION",
                    Style::default()
                        .fg(COLOR_AMBER)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  Press [3] to resolve", Style::default().fg(COLOR_AMBER)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            rule_str,
            Style::default().fg(COLOR_SEPARATOR),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "VIRTUAL FILE INDEX ({} PATHS)",
                selected.affected_files.len()
            ),
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let total_files = selected.affected_files.len();
        for (idx, file) in selected.affected_files.iter().enumerate() {
            let is_file_selected = inspector_focused && idx == app.inspector_file_cursor;
            let connector = if idx + 1 == total_files {
                "╰─ "
            } else {
                "├─ "
            };

            let (path, is_override) = if let Some(stripped) = file.strip_suffix(" (override)") {
                (stripped, true)
            } else {
                (file.as_str(), false)
            };

            let mut file_spans = vec![
                Span::styled(
                    if is_file_selected { "> " } else { connector },
                    if is_file_selected {
                        style_selected_row()
                    } else {
                        Style::default().fg(COLOR_TEXT_MUTED)
                    },
                ),
                Span::styled(
                    path,
                    if is_file_selected {
                        style_selected_row()
                    } else {
                        Style::default().fg(COLOR_TEXT_PRIMARY)
                    },
                ),
            ];
            if is_override {
                file_spans.push(Span::raw(" "));
                file_spans.push(Span::styled(
                    "[OVERRIDE]",
                    if is_file_selected {
                        style_selected_row_secondary()
                    } else {
                        Style::default()
                            .fg(COLOR_AMBER)
                            .add_modifier(Modifier::BOLD)
                    },
                ));
            }
            lines.push(Line::from(file_spans));
        }

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    } else {
        let p = Paragraph::new("No package selected")
            .style(Style::default().fg(COLOR_TEXT_MUTED))
            .block(
                ratatui::widgets::Block::default()
                    .borders(Borders::NONE)
                    .border_type(BorderType::Rounded),
            );
        f.render_widget(p, inner);
    }
}
