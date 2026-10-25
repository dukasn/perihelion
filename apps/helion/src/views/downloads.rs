use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::{
    action::{DownloadsFocus, InputValidation},
    app::App,
    theme::{
        card_block, input_block, panel_block, pill, pill_solid, style_ghost_text,
        style_placeholder, COLOR_AMBER, COLOR_AMBER_BG, COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL,
        COLOR_CYAN, COLOR_CYAN_BG, COLOR_GREEN, COLOR_GREEN_BG, COLOR_KEYCAP_BG, COLOR_RED,
        COLOR_RED_BG, COLOR_SEPARATOR, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_downloads_view(f: &mut Frame, app: &App, area: Rect) {
    let main_block = panel_block(true)
        .title(
            Line::from(Span::styled(
                " ◆ Package Transport & P2P Swarm Queue ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                " .PECA (TAR.ZST) STREAMING ",
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        );
    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // High-Contrast URL / Magnet Input Well
            Constraint::Length(1), // Spacing
            Constraint::Length(9), // Download Queue Card
            Constraint::Length(1), // Spacing
            Constraint::Min(6),    // Curated Packages Card
        ])
        .split(inner);

    // 1. High-Contrast URL / Magnet / Slug Input Well
    let validation = app.validate_download_input();
    let has_error = matches!(validation, InputValidation::Invalid(_));
    let is_valid = matches!(validation, InputValidation::Valid(_));
    let input_focused = app.download_input_active || app.download_focus == DownloadsFocus::Input;

    let status_badge = match &validation {
        InputValidation::Idle => {
            if app.download_input_active {
                pill_solid("EDITING -- ENTER: SUBMIT · ESC: CANCEL", COLOR_CYAN)
            } else if input_focused {
                pill_solid("FOCUSED -- PRESS [ENTER/SPACE] TO EDIT", COLOR_CYAN)
            } else {
                pill(
                    "Press [Tab] or [u] to focus",
                    COLOR_TEXT_SECONDARY,
                    COLOR_KEYCAP_BG,
                    false,
                )
            }
        }
        InputValidation::Valid(msg) => pill(
            format!("+ {} · Press Enter", msg),
            COLOR_GREEN,
            COLOR_GREEN_BG,
            true,
        ),
        InputValidation::Warning(msg) => {
            pill(format!("! {}", msg), COLOR_AMBER, COLOR_AMBER_BG, true)
        }
        InputValidation::Invalid(msg) => pill(format!("x {}", msg), COLOR_RED, COLOR_RED_BG, true),
    };

    let suggestion = if app.download_input_active {
        crate::autocomplete::Autocomplete::suggest_download_uri(&app.download_input, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            Span::styled(
                " >> Direct Package URI / BitTorrent Magnet / Registry Slug ",
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
        Line::from(vec![Span::styled(
            if app.download_input_active {
                " >> Direct Package URI / BitTorrent Magnet / Registry Slug (Editing) "
            } else if input_focused {
                " >> Direct Package URI / BitTorrent Magnet / Registry Slug (Focused) "
            } else {
                " >> Direct Package URI / BitTorrent Magnet / Registry Slug "
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
        )])
    };

    let url_box = input_block(input_focused, has_error, is_valid)
        .title(title_line.alignment(Alignment::Left))
        .title(
            Line::from(vec![Span::raw(" "), status_badge, Span::raw(" ")])
                .alignment(Alignment::Right),
        );

    let url_inner = url_box.inner(chunks[0]);
    f.render_widget(url_box, chunks[0]);

    let input_line = if app.download_input_active {
        let mut spans = vec![
            Span::styled(
                "> ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
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
            Span::styled("█", Style::default().fg(COLOR_CYAN)),
        ];
        if let Some(s) = &suggestion {
            spans.push(Span::styled(&s.ghost_suffix, style_ghost_text()));
        }
        Line::from(spans)
    } else if app.download_input.is_empty() {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "https://mods.perihelion.io/pack.peca  or  magnet:?xt=urn:btih:...  or  skyui/next",
                style_placeholder(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_CYAN)),
            Span::styled(
                &app.download_input,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    f.render_widget(Paragraph::new(input_line), url_inner);

    // 2. Download Queue Card
    let queue_focused = !app.download_input_active && app.download_focus == DownloadsFocus::Queue;
    let queue_border = if queue_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let queue_block = card_block(queue_border)
        .title(
            Line::from(Span::styled(
                format!(" ◆ Active Transfer Queue ({}) ", app.downloads.len()),
                Style::default()
                    .fg(if queue_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::raw(" "),
                if queue_focused {
                    pill_solid("FOCUSED -- [Space/Enter] Pause/Resume", COLOR_CYAN)
                } else {
                    pill("● DHT + WEB SEEDS READY", COLOR_GREEN, COLOR_GREEN_BG, true)
                },
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );
    let queue_inner = queue_block.inner(chunks[2]);
    f.render_widget(queue_block, chunks[2]);

    if let Some(item) = app.downloads.get(app.download_cursor) {
        let q_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title + Status
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Progress bar
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Speed + seeds info
            ])
            .split(queue_inner);

        let (st_fg, st_bg) = if item.paused {
            (COLOR_AMBER, COLOR_AMBER_BG)
        } else if item.progress >= 100.0 {
            (COLOR_GREEN, COLOR_GREEN_BG)
        } else {
            (COLOR_CYAN, COLOR_CYAN_BG)
        };

        let title_line = Line::from(vec![
            Span::styled(
                if queue_focused { "┃ " } else { "  " },
                Style::default().fg(COLOR_CYAN),
            ),
            Span::styled(
                &item.name,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            pill(&item.status, st_fg, st_bg, true),
            Span::styled(
                format!(
                    "   (Item {}/{})",
                    app.download_cursor + 1,
                    app.downloads.len()
                ),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]);
        f.render_widget(Paragraph::new(title_line), q_chunks[0]);

        // Smooth Block Progress Bar
        let bar_width: usize = (q_chunks[2].width as usize)
            .saturating_sub(14)
            .clamp(20, 68);
        let filled_count = ((item.progress / 100.0) * bar_width as f32).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);
        let filled_str = "━".repeat(filled_count);
        let empty_str = "─".repeat(empty_count);
        let bar_color = if item.paused {
            COLOR_AMBER
        } else {
            COLOR_GREEN
        };
        let bar_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(filled_str, Style::default().fg(bar_color)),
            Span::styled(empty_str, Style::default().fg(COLOR_BORDER_NORMAL)),
            Span::raw("  "),
            pill(
                format!("{:>3.0}%", item.progress),
                COLOR_TEXT_PRIMARY,
                COLOR_KEYCAP_BG,
                true,
            ),
        ]);
        f.render_widget(Paragraph::new(bar_line), q_chunks[2]);

        let speed_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(q_chunks[4]);

        let speed_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("Throughput: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &item.speed,
                Style::default()
                    .fg(COLOR_TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(speed_line), speed_chunks[0]);

        let peers_line = Paragraph::new(Line::from(vec![
            Span::styled("Transport: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(&item.peers_info, Style::default().fg(COLOR_TEXT_SECONDARY)),
        ]))
        .alignment(Alignment::Right);
        f.render_widget(peers_line, speed_chunks[1]);
    }

    // 3. Curated Community Packages Card
    let curated_focused =
        !app.download_input_active && app.download_focus == DownloadsFocus::Curated;
    let curated_border = if curated_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let curated_block = card_block(curated_border)
        .title(
            Line::from(Span::styled(
                " ◆ Curated .peca Bundles ",
                Style::default()
                    .fg(if curated_focused {
                        COLOR_CYAN
                    } else {
                        COLOR_TEXT_PRIMARY
                    })
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(vec![
                Span::raw(" "),
                if curated_focused {
                    pill_solid("FOCUSED -- [Enter/Space] Install", COLOR_CYAN)
                } else {
                    Span::styled(
                        " [Tab] Focus Section ",
                        Style::default().fg(COLOR_TEXT_MUTED),
                    )
                },
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );
    let curated_inner = curated_block.inner(chunks[4]);
    f.render_widget(curated_block, chunks[4]);

    let rule_width = curated_inner.width as usize;
    let mut pkg_lines = Vec::new();
    for (idx, pkg) in app.curated_packages.iter().enumerate() {
        let is_selected = curated_focused && idx == app.curated_cursor;

        let badge = if pkg.is_installed {
            pill(
                "+ INSTALLED IN WORKSPACE",
                COLOR_GREEN,
                COLOR_GREEN_BG,
                true,
            )
        } else if is_selected {
            pill_solid(">> PRESS [ENTER/SPACE] TO INSTALL", COLOR_CYAN)
        } else {
            pill(
                "+ 1-CLICK INSTALL",
                COLOR_TEXT_SECONDARY,
                COLOR_KEYCAP_BG,
                false,
            )
        };

        let row_prefix = if is_selected {
            Span::styled("┃ ◆ ", Style::default().fg(COLOR_CYAN))
        } else {
            Span::styled("  ◆ ", Style::default().fg(COLOR_TEXT_MUTED))
        };

        let title_style = if is_selected {
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD)
        };

        pkg_lines.push(Line::from(vec![
            row_prefix,
            Span::styled(&pkg.title, title_style),
            Span::raw("   "),
            badge,
        ]));
        pkg_lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(&pkg.desc, Style::default().fg(COLOR_TEXT_SECONDARY)),
        ]));
        if idx + 1 < app.curated_packages.len() {
            pkg_lines.push(Line::from(Span::styled(
                "─".repeat(rule_width),
                Style::default().fg(COLOR_SEPARATOR),
            )));
        }
    }
    let curated_widget = Paragraph::new(pkg_lines).wrap(Wrap { trim: false });
    f.render_widget(curated_widget, curated_inner);
}
