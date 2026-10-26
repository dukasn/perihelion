use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use crate::{
    action::{DownloadsFocus, InputValidation},
    app::App,
    theme::{
        box_title_label, card_block, input_block, keycap, panel_block, render_segmented_bar,
        style_block_right_label, style_ghost_text, style_placeholder, style_selected_row,
        style_selected_row_secondary, COLOR_AMBER, COLOR_BORDER_NORMAL, COLOR_FOCUS, COLOR_RED,
        COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_downloads_view(f: &mut Frame, app: &App, area: Rect) {
    if app.show_inspector {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(62),
                Constraint::Length(1), // Breathing gap between panes
                Constraint::Percentage(38),
            ])
            .split(area);

        render_transfers_table_pane(f, app, chunks[0]);
        render_downloads_inspector_pane(f, app, chunks[2]);
    } else {
        render_transfers_table_pane(f, app, area);
    }
}

fn render_transfers_table_pane(f: &mut Frame, app: &App, area: Rect) {
    let queue_focused = !app.download_input_active && app.download_focus == DownloadsFocus::Queue;
    let curated_focused =
        !app.download_input_active && app.download_focus == DownloadsFocus::Curated;

    let main_block = panel_block(false)
        .title(
            Line::from(Span::styled(
                " Package Transport & Downloads ",
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
                        " {} Transfers | {} Curated ",
                        app.downloads.len(),
                        app.curated_packages.len()
                    ),
                    style_block_right_label(queue_focused || curated_focused),
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
            Constraint::Length(3), // Top Toolbar: Input Well & View Filter Box
            Constraint::Min(5),    // Unified Transfers Table
            Constraint::Length(1), // Divider
            Constraint::Length(1), // Bottom Preview / Swarm Status Bar
        ])
        .split(inner);

    // 1. Top Input & Filter Toolbar
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(36),
            Constraint::Length(2),
            Constraint::Length(34),
        ])
        .split(vertical_layout[0]);

    let validation = app.validate_download_input();
    let has_error = matches!(validation, InputValidation::Invalid(_));
    let is_valid = matches!(validation, InputValidation::Valid(_));
    let input_focused = app.download_input_active || app.download_focus == DownloadsFocus::Input;

    let status_badge = match &validation {
        InputValidation::Idle => {
            if app.download_input_active {
                Span::styled(
                    "[Esc] Cancel  [Enter] Submit",
                    style_block_right_label(true),
                )
            } else if input_focused {
                Span::styled("[u] Edit URL", style_block_right_label(true))
            } else {
                Span::styled("[u] Edit URL", style_block_right_label(false))
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

    let suggestion = if app.download_input_active {
        crate::autocomplete::Autocomplete::suggest_download_uri(&app.download_input, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            box_title_label("> Direct URI / Magnet / Slug", input_focused),
            Span::raw(" "),
            Span::styled(
                format!("Tab/-> Complete: {}", s.completed_text),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(box_title_label(
            "> Direct URI / Magnet / Slug",
            input_focused,
        ))
    };

    let url_box = input_block(input_focused, has_error, is_valid)
        .title(title_line.alignment(Alignment::Left))
        .title(
            Line::from(vec![Span::raw(" "), status_badge, Span::raw(" ")])
                .alignment(Alignment::Right),
        );

    let url_inner = url_box.inner(top_chunks[0]);
    f.render_widget(url_box, top_chunks[0]);

    let input_line = if app.download_input_active {
        let mut spans = vec![
            Span::styled(
                "> ",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &app.download_input,
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
    } else if app.download_input.is_empty() {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "https://mods.perihelion.io/pack.peca or magnet:?xt=urn:btih:...",
                style_placeholder(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.download_input,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    f.render_widget(Paragraph::new(input_line), url_inner);

    // View filter box on right
    let filter_card = card_block(COLOR_BORDER_NORMAL)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
        .title(
            Line::from(Span::styled(
                " View Filter ",
                style_block_right_label(false),
            ))
            .alignment(Alignment::Right),
        );
    let filter_text = Line::from(vec![
        Span::styled(
            " [All Transfers] ",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Queue ({}) ", app.downloads.len()),
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
        Span::styled(
            format!("Curated ({})", app.curated_packages.len()),
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]);
    f.render_widget(
        Paragraph::new(filter_text)
            .alignment(Alignment::Right)
            .block(filter_card),
        top_chunks[2],
    );

    // 2. Unified Transfers Table
    let header_style = Style::default()
        .fg(COLOR_TEXT_SECONDARY)
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from("  STATE").style(header_style),
        Cell::from("PACKAGE / ARCHIVE").style(header_style),
        Cell::from("PROTOCOL / TYPE").style(header_style),
        Cell::from("THROUGHPUT").style(header_style),
        Cell::from("TRANSPORT / SOURCE").style(header_style),
        Cell::from("ACTION / STATUS").style(header_style),
    ])
    .height(1);

    let mut rows = Vec::new();

    // Active download transfers
    for (idx, item) in app.downloads.iter().enumerate() {
        let is_selected = queue_focused && idx == app.download_cursor;

        let (cursor_bar, state_span, name_span, proto_span, speed_span, peers_span, status_span) =
            if is_selected {
                let sel_bold = style_selected_row();
                let sel_dim = style_selected_row_secondary();

                let cursor = Span::styled("> ", sel_bold);
                let state = if item.paused {
                    Span::styled(
                        "[PAUSED]",
                        Style::default()
                            .bg(COLOR_FOCUS)
                            .fg(COLOR_AMBER)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if item.progress >= 100.0 {
                    Span::styled("[DONE]", sel_dim)
                } else {
                    Span::styled(format!("v {:>3.0}%", item.progress), sel_bold)
                };
                let name = Span::styled(&item.name, sel_bold);
                let proto = Span::styled("P2P BitTorrent (.peca)", sel_dim);
                let speed = Span::styled(&item.speed, sel_bold);
                let peers = Span::styled(&item.peers_info, sel_dim);
                let status = Span::styled(&item.status, sel_dim);
                (cursor, state, name, proto, speed, peers, status)
            } else {
                let cursor_bar = Span::raw("  ");
                let state_span = if item.paused {
                    Span::styled("[PAUSED]", Style::default().fg(COLOR_AMBER))
                } else if item.progress >= 100.0 {
                    Span::styled("[DONE]", Style::default().fg(COLOR_TEXT_MUTED))
                } else {
                    Span::styled(
                        format!("v {:>3.0}%", item.progress),
                        Style::default()
                            .fg(COLOR_TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )
                };
                let name_style = Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD);
                let status_style = if item.paused {
                    Style::default().fg(COLOR_AMBER)
                } else if item.progress >= 100.0 {
                    Style::default().fg(COLOR_TEXT_MUTED)
                } else {
                    Style::default().fg(COLOR_TEXT_SECONDARY)
                };
                (
                    cursor_bar,
                    state_span,
                    Span::styled(&item.name, name_style),
                    Span::styled(
                        "P2P BitTorrent (.peca)",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                    Span::styled(&item.speed, Style::default().fg(COLOR_TEXT_SECONDARY)),
                    Span::styled(&item.peers_info, Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(&item.status, status_style),
                )
            };

        let mut row = Row::new(vec![
            Cell::from(Line::from(vec![cursor_bar, state_span])),
            Cell::from(Line::from(name_span)),
            Cell::from(Line::from(proto_span)),
            Cell::from(Line::from(speed_span)),
            Cell::from(Line::from(peers_span)),
            Cell::from(Line::from(status_span)),
        ])
        .height(1);

        if is_selected {
            row = row.style(style_selected_row());
        }

        rows.push(row);
    }

    // Curated packages section
    for (idx, pkg) in app.curated_packages.iter().enumerate() {
        let is_selected = curated_focused && idx == app.curated_cursor;

        let (cursor_bar, state_span, title_span, type_span, speed_span, host_span, action_span) =
            if is_selected {
                let sel_bold = style_selected_row();
                let sel_dim = style_selected_row_secondary();

                let cursor = Span::styled("> ", sel_bold);
                let state = if pkg.is_installed {
                    Span::styled("[INSTALLED]", sel_dim)
                } else {
                    Span::styled("+ REGISTRY", sel_bold)
                };
                let title = Span::styled(&pkg.title, sel_bold);
                let typ = Span::styled("Curated Registry (.peca)", sel_dim);
                let speed = Span::styled("-", sel_dim);
                let host = Span::styled("mods.perihelion.io", sel_dim);
                let action = if pkg.is_installed {
                    Span::styled("Active in VFS", sel_dim)
                } else {
                    Span::styled("> [Enter] Install", sel_bold)
                };
                (cursor, state, title, typ, speed, host, action)
            } else {
                let cursor_bar = Span::raw("  ");
                let state_span = if pkg.is_installed {
                    Span::styled("[INSTALLED]", Style::default().fg(COLOR_TEXT_MUTED))
                } else {
                    Span::styled("+ REGISTRY", Style::default().fg(COLOR_TEXT_SECONDARY))
                };
                let name_style = Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD);
                let action_span = if pkg.is_installed {
                    Span::styled("Active in VFS", Style::default().fg(COLOR_TEXT_MUTED))
                } else {
                    Span::styled("+ 1-Click Install", Style::default().fg(COLOR_TEXT_MUTED))
                };
                (
                    cursor_bar,
                    state_span,
                    Span::styled(&pkg.title, name_style),
                    Span::styled(
                        "Curated Registry (.peca)",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                    Span::styled("-", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled("mods.perihelion.io", Style::default().fg(COLOR_TEXT_MUTED)),
                    action_span,
                )
            };

        let mut row = Row::new(vec![
            Cell::from(Line::from(vec![cursor_bar, state_span])),
            Cell::from(Line::from(title_span)),
            Cell::from(Line::from(type_span)),
            Cell::from(Line::from(speed_span)),
            Cell::from(Line::from(host_span)),
            Cell::from(Line::from(action_span)),
        ])
        .height(1);

        if is_selected {
            row = row.style(style_selected_row());
        }

        rows.push(row);
    }

    let widths = [
        Constraint::Length(12),
        Constraint::Percentage(32),
        Constraint::Percentage(22),
        Constraint::Percentage(14),
        Constraint::Percentage(18),
        Constraint::Length(18),
    ];

    if rows.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No active downloads or packages in queue.",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                keycap("u"),
                Span::styled(
                    " to enter a package URL (HTTP/HTTPS), magnet link, or local .peca path.",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(empty_lines).alignment(Alignment::Center),
            vertical_layout[1],
        );
    } else {
        let table = Table::new(rows, widths).header(header).column_spacing(2);

        let mut state = TableState::default();
        let selected_index = if queue_focused {
            Some(app.download_cursor)
        } else if curated_focused {
            Some(app.downloads.len() + app.curated_cursor)
        } else {
            None
        };
        state.select(selected_index);
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
            "SWARM TRANSPORT",
            Style::default()
                .fg(COLOR_TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "DHT Online -> 14 Active Peers -> Multi-Threaded Zstandard (.peca) Stream -> Verified TLS Mirrors",
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ),
        Span::raw("   |   "),
        Span::styled(
            "[u] Input URI   [p] Pause/Resume   [i] Toggle Inspector",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(bottom_spans)), vertical_layout[3]);
}

fn render_downloads_inspector_pane(f: &mut Frame, app: &App, area: Rect) {
    let curated_focused =
        !app.download_input_active && app.download_focus == DownloadsFocus::Curated;

    let inspector_focused = app.show_inspector && !app.download_input_active;
    let block = card_block(COLOR_BORDER_NORMAL)
        .title(
            Line::from(Span::styled(
                " Swarm & Package Inspector ",
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

    let rule_str = "─".repeat(inner.width as usize);

    if curated_focused {
        // Curated Package Inspector Details
        if let Some(pkg) = app.curated_packages.get(app.curated_cursor) {
            let status_span = if pkg.is_installed {
                Span::styled(
                    "[x] Mounted in Lowerdir Plan",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "[ ] Ready for 1-Click Install",
                    Style::default().fg(COLOR_TEXT_MUTED),
                )
            };

            let mut lines = vec![
                Line::from(Span::styled(
                    &pkg.title,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(status_span),
                Line::from(""),
                Line::from(Span::styled(
                    &pkg.desc,
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    rule_str.clone(),
                    Style::default().fg(COLOR_SEPARATOR),
                )),
                Line::from(Span::styled(
                    "PACKAGE & TRANSPORT SPECIFICATION",
                    Style::default()
                        .fg(COLOR_TEXT_MUTED)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Registry    ", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(
                        "mods.perihelion.io (Official)",
                        Style::default().fg(COLOR_TEXT_PRIMARY),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Archive     ", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(
                        ".peca (Multi-Threaded Zstd Tarball)",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Mount Mode  ", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(
                        "OverlayFS Non-Destructive Lowerdir",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Integrity   ", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(
                        "SHA-256 Checksum Verified",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(rule_str, Style::default().fg(COLOR_SEPARATOR))),
                Line::from(""),
            ];

            if pkg.is_installed {
                lines.push(Line::from(Span::styled(
                    "Package is already installed and active in the current workspace profile.",
                    Style::default().fg(COLOR_TEXT_MUTED),
                )));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        ">> Press [Enter / Space] ",
                        Style::default()
                            .fg(COLOR_FOCUS)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "to stream & install package into workspace lowerdir.",
                        Style::default().fg(COLOR_TEXT_SECONDARY),
                    ),
                ]));
            }

            let p = Paragraph::new(lines).wrap(Wrap { trim: false });
            f.render_widget(p, inner);
            return;
        }
    }

    // Default: Download Queue Item Inspector Details
    if let Some(item) = app.downloads.get(app.download_cursor) {
        let status_span = if item.paused {
            Span::styled("[PAUSED]", Style::default().fg(COLOR_AMBER))
        } else if item.progress >= 100.0 {
            Span::styled(
                "[OK] Transfer Complete & Verified",
                Style::default().fg(COLOR_TEXT_MUTED),
            )
        } else {
            Span::styled(
                format!("* Downloading ({:>3.0}%)", item.progress),
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        };

        // Segmented Progress Bar
        let bar_width: usize = (inner.width as usize).saturating_sub(14).clamp(16, 50);
        let norm_progress = if item.progress > 1.0 {
            item.progress / 100.0
        } else {
            item.progress
        };
        let mut bar_spans = vec![Span::raw("  ")];
        bar_spans.extend(render_segmented_bar(norm_progress, bar_width).spans);
        let bar_line = Line::from(bar_spans);

        let lines = vec![
            Line::from(Span::styled(
                &item.name,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(status_span),
            Line::from(""),
            bar_line,
            Line::from(""),
            Line::from(Span::styled(
                rule_str.clone(),
                Style::default().fg(COLOR_SEPARATOR),
            )),
            Line::from(Span::styled(
                "SWARM TRANSPORT & PEER TELEMETRY",
                Style::default()
                    .fg(COLOR_TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Throughput   : ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    &item.speed,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Peers & Seeds: ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(&item.peers_info, Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
            Line::from(vec![
                Span::styled("Archive Type : ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    ".peca (Multi-Threaded Zstd Tarball)",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("Verification : ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "SHA-256 Streaming Verified",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("Mount Mode   : ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "OverlayFS Lowerdir Auto-Mount",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(rule_str, Style::default().fg(COLOR_SEPARATOR))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Shortcuts: ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    "[p]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " Pause/Resume   ",
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
                Span::styled(
                    "[u]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" New URL   ", Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::styled(
                    "[Tab]",
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Next Area", Style::default().fg(COLOR_TEXT_SECONDARY)),
            ]),
        ];

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    } else {
        let p = Paragraph::new("No active transfer selected")
            .style(Style::default().fg(COLOR_TEXT_MUTED));
        f.render_widget(p, inner);
    }
}
