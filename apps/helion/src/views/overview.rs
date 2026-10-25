use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::{
    action::{InputValidation, OverviewFocus},
    app::App,
    theme::{
        card_block, input_block, pill, pill_solid, style_ghost_text, style_placeholder,
        COLOR_AMBER, COLOR_BLUE, COLOR_BORDER_FOCUSED, COLOR_BORDER_NORMAL, COLOR_BORDER_SUBTLE,
        COLOR_CYAN, COLOR_CYAN_BG, COLOR_GREEN, COLOR_GREEN_BG, COLOR_RED, COLOR_TEXT_MUTED,
        COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_overview_view(f: &mut Frame, app: &App, area: Rect) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Hero Banner (SVG logo + brand tagline)
            Constraint::Length(3), // Interactive Action / Profile Input Bar
            Constraint::Length(1), // Validation & suggestion strip
            Constraint::Min(8),    // Two-Column Status & Profiles Dashboard
            Constraint::Length(1), // Bottom quick navigation bar
        ])
        .split(area);

    render_hero_banner(f, app, vertical_chunks[0]);
    render_action_bar(f, app, vertical_chunks[1], vertical_chunks[2]);
    render_dashboard(f, app, vertical_chunks[3]);
    render_quick_nav(f, vertical_chunks[4]);
}

fn render_hero_banner(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 75 {
        // Compact fallback for narrow screens
        let compact_text = vec![
            Line::from(vec![
                Span::styled(
                    "◆ PERIHELION ",
                    Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                ),
                Span::styled("v0.1.0-alpha", Style::default().fg(COLOR_TEXT_MUTED)),
            ]),
            Line::from(Span::styled(
                "Mod Fearlessly. Share Instantly.",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                pill("Engineered Differently", COLOR_CYAN, COLOR_CYAN_BG, true),
                Span::raw("  "),
                pill("Zero-Copy", COLOR_GREEN, COLOR_GREEN_BG, false),
                Span::raw("  "),
                pill("P2P Swarm", COLOR_BLUE, COLOR_CYAN_BG, false),
            ]),
        ];
        f.render_widget(Paragraph::new(compact_text), area);
        return;
    }

    let hero_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // SVG logo block art
            Constraint::Length(2),  // Spacer
            Constraint::Min(40),    // Brand title, description, and architectural badges
        ])
        .split(area);

    // Exact geometric representation of assets/perihelion-vector.svg
    let logo_lines = [
        "    ██      ██    ",
        "  ██████  ██████  ",
        "██████  ██████    ",
        "  ██      ██      ",
        "    ████    ████  ",
        "      ██      ██  ",
    ];

    let mut logo_spans = Vec::new();
    for line in logo_lines {
        logo_spans.push(Line::from(Span::styled(
            line,
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        )));
    }
    f.render_widget(Paragraph::new(logo_spans), hero_cols[0]);

    // Brand metadata and empowering value proposition
    let brand_lines = vec![
        Line::from(vec![
            Span::styled(
                "P E R I H E L I O N",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            pill_solid("v0.1.0-alpha", COLOR_BORDER_SUBTLE),
            Span::raw("   "),
            Span::styled(
                format!("● READY :: {}", app.game_title),
                Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "Mod Fearlessly. Share Instantly.",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "No 40-step guides. No throttled downloads. Sync and play entire setups in one command.",
            Style::default().fg(COLOR_TEXT_MUTED),
        )),
        Line::from(Span::raw("")),
        Line::from(vec![
            pill("ENGINEERED DIFFERENTLY", COLOR_CYAN, COLOR_CYAN_BG, true),
            Span::raw("  "),
            pill("KERNEL OVERLAYFS", COLOR_BORDER_SUBTLE, COLOR_CYAN_BG, false),
            Span::raw("  "),
            pill("UNPRIVILEGED USER NS", COLOR_BORDER_SUBTLE, COLOR_CYAN_BG, false),
            Span::raw("  "),
            pill("ZERO-COPY ISOLATION", COLOR_GREEN, COLOR_GREEN_BG, false),
            Span::raw("  "),
            pill("P2P SWARM STREAMING", COLOR_BLUE, COLOR_CYAN_BG, false),
        ]),
    ];

    f.render_widget(
        Paragraph::new(brand_lines).wrap(Wrap { trim: false }),
        hero_cols[2],
    );
}

fn render_action_bar(f: &mut Frame, app: &App, input_area: Rect, strip_area: Rect) {
    let is_focused = app.overview_focus == OverviewFocus::CommandInput;
    let is_active = app.overview_input_active;
    let validation = app.validate_overview_input();

    let has_error = matches!(validation, InputValidation::Invalid(_));
    let is_valid = matches!(validation, InputValidation::Valid(_));

    let suggestion = if is_active {
        crate::autocomplete::Autocomplete::suggest_overview(&app.overview_input, app)
    } else {
        None
    };

    let title_line = if let Some(s) = &suggestion {
        Line::from(vec![
            Span::raw(" Quick Action / Profile Bar "),
            pill(
                format!("Tab/→ Complete: {}", s.completed_text),
                COLOR_CYAN,
                COLOR_CYAN_BG,
                true,
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(" Quick Action / Profile Bar ")
    };

    let block = input_block(is_focused || is_active, has_error, is_valid)
        .title(title_line)
        .title_alignment(Alignment::Left);

    let prompt_prefix = Span::styled(
        "> ",
        Style::default()
            .fg(if is_active {
                COLOR_CYAN
            } else {
                COLOR_TEXT_MUTED
            })
            .add_modifier(Modifier::BOLD),
    );

    let mut line_spans = vec![prompt_prefix];

    if is_active {
        if app.overview_input.is_empty() {
            line_spans.push(Span::styled(
                "█",
                Style::default()
                    .fg(COLOR_CYAN)
                    .add_modifier(Modifier::RAPID_BLINK),
            ));
            line_spans.push(Span::raw(" "));
            line_spans.push(Span::styled(
                "clone <url-or-slug> | launch | switch <profile> | search <pkg>",
                style_placeholder(),
            ));
        } else {
            line_spans.push(Span::styled(
                &app.overview_input,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ));
            line_spans.push(Span::styled("█", Style::default().fg(COLOR_CYAN)));
            if let Some(s) = &suggestion {
                line_spans.push(Span::styled(&s.ghost_suffix, style_ghost_text()));
            }
        }
    } else if app.overview_input.is_empty() {
        line_spans.push(Span::styled(
            "clone <url-or-slug> | launch | switch <profile> | search <pkg>  [press '/' to focus]",
            style_placeholder(),
        ));
    } else {
        line_spans.push(Span::styled(
            &app.overview_input,
            Style::default().fg(COLOR_TEXT_PRIMARY),
        ));
    }

    f.render_widget(
        Paragraph::new(Line::from(line_spans)).block(block),
        input_area,
    );

    // Suggestion / Validation status strip below
    let strip_line = if let Some(s) = &suggestion {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Autocomplete: ",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[Tab] or [→] to complete '",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
            Span::styled(
                &s.completed_text,
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("'", Style::default().fg(COLOR_TEXT_SECONDARY)),
        ])
    } else {
        match validation {
            InputValidation::Valid(msg) => Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "✓ ",
                    Style::default()
                        .fg(COLOR_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg, Style::default().fg(COLOR_GREEN)),
                Span::raw("   "),
                Span::styled(
                    "[Press Enter to Execute]",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ]),
            InputValidation::Warning(msg) => Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "! ",
                    Style::default()
                        .fg(COLOR_AMBER)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg, Style::default().fg(COLOR_AMBER)),
            ]),
            InputValidation::Invalid(msg) => Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "✗ ",
                    Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg, Style::default().fg(COLOR_RED)),
            ]),
            InputValidation::Idle => Line::from(vec![
                Span::raw("  "),
                Span::styled("Commands: ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled("clone <uri>", Style::default().fg(COLOR_CYAN)),
                Span::styled(" · ", Style::default().fg(COLOR_BORDER_SUBTLE)),
                Span::styled("launch", Style::default().fg(COLOR_GREEN)),
                Span::styled(" · ", Style::default().fg(COLOR_BORDER_SUBTLE)),
                Span::styled("switch <game>", Style::default().fg(COLOR_BLUE)),
                Span::styled(" · ", Style::default().fg(COLOR_BORDER_SUBTLE)),
                Span::styled("search <mod>", Style::default().fg(COLOR_AMBER)),
                Span::styled(
                    "   │   [Tab] Switch Area  [Enter] Select",
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
            ]),
        }
    };

    f.render_widget(Paragraph::new(strip_line), strip_area);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // VFS Sandbox & Engine Status
            Constraint::Percentage(50), // Workspaces & Fast Profiles Switcher
        ])
        .split(area);

    // 1. Left Card: VFS Sandbox Telemetry & Engineering Credibility
    let left_block = card_block(COLOR_BORDER_NORMAL)
        .title(" ◆ Under the Hood: Kernel VFS Engine ")
        .title_alignment(Alignment::Left);

    let active_profile = app.profiles.iter().find(|p| p.is_active);
    let profile_name = active_profile
        .map(|p| p.name.as_str())
        .unwrap_or("Default Profile");
    let mod_count = app.mods.len();

    let telemetry_lines = vec![
        Line::from(vec![
            Span::styled("Target Title    : ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.game_title,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [Prototype Preview]", Style::default().fg(COLOR_AMBER)),
        ]),
        Line::from(vec![
            Span::styled("Active Blueprint: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                profile_name,
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            pill_solid("PREVIEW", COLOR_CYAN),
        ]),
        Line::from(vec![
            Span::styled("Base Game Safety: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Immutable & Pristine",
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Read-Only Lowerdir)",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("Zero-Copy Engine: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "CLONE_NEWNS | CLONE_NEWUSER",
                Style::default().fg(COLOR_CYAN),
            ),
            Span::styled(
                " (No root / sudo required)",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("Upperdir (RW)   : ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "~/.local/share/perihelion/profiles/skyrim_requiem/upper",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("Live Layer Stack: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!(
                    "{} isolated OverlayFS layers (SkyUI ‣ Iron ‣ Ordinator ‣ ...)",
                    mod_count
                ),
                Style::default().fg(COLOR_CYAN),
            ),
        ]),
        Line::from(vec![
            Span::styled("Transport & VFS : ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.proton_version,
                Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " · P2P Swarm Active (14 Peers)",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(telemetry_lines).block(left_block), cols[0]);

    // 2. Right Card: Fast Workspaces / Profiles Switcher
    let is_profiles_focused = app.overview_focus == OverviewFocus::RecentProfiles;
    let right_border = if is_profiles_focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_NORMAL
    };

    let right_block = card_block(right_border)
        .title(" ◆ Workspaces & Profiles (Development Preview) ")
        .title_alignment(Alignment::Left);

    let mut profile_lines = Vec::new();
    profile_lines.push(Line::from(vec![Span::styled(
        "   NAME                     TARGET GAME      MODS    STATUS",
        Style::default()
            .fg(COLOR_TEXT_MUTED)
            .add_modifier(Modifier::BOLD),
    )]));

    for (idx, profile) in app.profiles.iter().enumerate() {
        let is_cursor = idx == app.overview_profile_cursor && is_profiles_focused;
        let prefix = if is_cursor { " ▸ " } else { "   " };

        let name_padded = format!(
            "{:<24}",
            if profile.name.len() > 23 {
                format!("{}…", &profile.name[..22])
            } else {
                profile.name.clone()
            }
        );

        let game_padded = format!(
            "{:<16}",
            if profile.game_short.len() > 15 {
                format!("{}…", &profile.game_short[..14])
            } else {
                profile.game_short.clone()
            }
        );

        let mods_padded = format!("{:>4} mods", profile.mod_count);

        let row_style = if is_cursor {
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)
        } else if profile.is_active {
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT_SECONDARY)
        };

        let status_span = if profile.is_active {
            pill_solid("ACTIVE", COLOR_GREEN)
        } else {
            Span::styled("  Ready", Style::default().fg(COLOR_TEXT_MUTED))
        };

        profile_lines.push(Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(if is_cursor {
                    COLOR_CYAN
                } else {
                    COLOR_TEXT_MUTED
                }),
            ),
            Span::styled(name_padded, row_style),
            Span::styled(game_padded, Style::default().fg(COLOR_BLUE)),
            Span::raw(" "),
            Span::styled(mods_padded, Style::default().fg(COLOR_TEXT_MUTED)),
            Span::raw("   "),
            status_span,
        ]));
    }

    if is_profiles_focused {
        profile_lines.push(Line::from(Span::raw("")));
        profile_lines.push(Line::from(vec![
            Span::styled(
                "  [↑/↓]",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Select   ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Activate Profile   ",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::styled(
                "[5]",
                Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Full Inspector", Style::default().fg(COLOR_TEXT_MUTED)),
        ]));
    }

    f.render_widget(Paragraph::new(profile_lines).block(right_block), cols[1]);
}

fn render_quick_nav(f: &mut Frame, area: Rect) {
    let nav_line = Line::from(vec![
        Span::raw(" "),
        Span::styled("Quick Jump: ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(
            "[1]",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Overview  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(
            "[2]",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Mods  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(
            "[3]",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Conflicts  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(
            "[4]",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Downloads  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(
            "[5]",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Profiles  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled("│  ", Style::default().fg(COLOR_BORDER_SUBTLE)),
        Span::styled(
            "[F9]",
            Style::default()
                .fg(COLOR_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Launch Game  ", Style::default().fg(COLOR_TEXT_SECONDARY)),
        Span::styled(
            "[?]",
            Style::default()
                .fg(COLOR_AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cheatsheet", Style::default().fg(COLOR_TEXT_SECONDARY)),
    ]);

    f.render_widget(Paragraph::new(nav_line), area);
}
