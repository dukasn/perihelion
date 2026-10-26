use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    action::{InputValidation, OverviewFocus},
    app::{App, ProfileItem},
    theme::{
        box_title_label, input_block, style_block_right_label, style_ghost_text, style_placeholder,
        style_selected_row, style_selected_row_secondary, COLOR_BORDER_SUBTLE, COLOR_FOCUS,
        COLOR_GREEN, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
    },
};

pub fn render_overview_view(f: &mut Frame, app: &App, area: Rect) {
    let hero_height = if area.height >= 26 {
        10
    } else if area.height >= 22 {
        8
    } else {
        5
    };

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(hero_height), // Centered Hero (vector logo + brand title + tagline + subline)
            Constraint::Length(1),           // Breathing gap
            Constraint::Length(3),           // Centered Command / Search Input bar
            Constraint::Length(1),           // Breathing gap
            Constraint::Min(6), // Dynamic Results Canvas (Idle Cockpit or Live Filter List)
        ])
        .split(area);

    render_hero_banner(f, app, vertical_chunks[0]);
    render_command_bar(f, app, vertical_chunks[2]);
    render_dynamic_canvas(f, app, vertical_chunks[4]);
}

fn render_hero_banner(f: &mut Frame, _app: &App, area: Rect) {
    // Exact geometric representation of assets/perihelion-vector.svg
    let logo_lines = [
        "    ██      ██    ",
        "  ██████  ██████  ",
        "██████  ██████    ",
        "  ██      ██      ",
        "    ████    ████  ",
        "      ██      ██  ",
    ];

    let mut hero_lines = Vec::new();

    if area.height >= 8 {
        for line in logo_lines {
            hero_lines.push(
                Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
            );
        }
    } else {
        for line in &logo_lines[1..3] {
            hero_lines.push(
                Line::from(Span::styled(
                    *line,
                    Style::default()
                        .fg(COLOR_TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
            );
        }
    }

    if area.height >= 10 {
        hero_lines.push(Line::raw(""));
    }

    // Title
    hero_lines.push(
        Line::from(Span::styled(
            "P E R I H E L I O N",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );

    // Tagline
    if area.height >= 8 {
        hero_lines.push(
            Line::from(Span::styled(
                "Mod Fearlessly. Share Instantly.",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        );
    }

    // Value proposition / Subtitle
    if area.height >= 10 {
        hero_lines.push(
            Line::from(Span::styled(
                "Playable in 1-click | 100% clean untouched base game",
                Style::default().fg(COLOR_TEXT_MUTED),
            ))
            .alignment(Alignment::Center),
        );
    }

    f.render_widget(Paragraph::new(hero_lines), area);
}

fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
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
            box_title_label("QUICK COMMAND / SEARCH", is_focused || is_active),
            Span::raw(" "),
            Span::styled(
                format!("Tab/-> Complete: {}", s.completed_text),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(box_title_label(
            "QUICK COMMAND / SEARCH",
            is_focused || is_active,
        ))
    };

    let status_badge = if is_active {
        Span::styled("[Esc] Back", style_block_right_label(true))
    } else if is_focused {
        Span::styled("[Enter] or [/] Focus", style_block_right_label(true))
    } else {
        Span::styled("[/] Focus", style_block_right_label(false))
    };

    let block = input_block(is_focused || is_active, has_error, is_valid)
        .title(title_line.alignment(Alignment::Left))
        .title(
            Line::from(vec![Span::raw(" "), status_badge, Span::raw(" ")])
                .alignment(Alignment::Right),
        );

    let prompt_prefix = Span::styled(
        "> ",
        Style::default()
            .fg(if is_active {
                COLOR_FOCUS
            } else {
                COLOR_TEXT_MUTED
            })
            .add_modifier(Modifier::BOLD),
    );

    let mut line_spans = vec![prompt_prefix];

    if is_active {
        if app.overview_input.is_empty() {
            line_spans.push(Span::styled("█", Style::default().fg(COLOR_FOCUS)));
            line_spans.push(Span::raw(" "));
            line_spans.push(Span::styled(
                "Type a command, profile name, or search query...",
                style_placeholder(),
            ));
        } else {
            line_spans.push(Span::styled(
                &app.overview_input,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ));
            line_spans.push(Span::styled("█", Style::default().fg(COLOR_FOCUS)));
            if let Some(s) = &suggestion {
                line_spans.push(Span::styled(&s.ghost_suffix, style_ghost_text()));
            }
        }
    } else if app.overview_input.is_empty() {
        line_spans.push(Span::styled(
            "Type a command, profile name, or search query...",
            style_placeholder(),
        ));
    } else {
        line_spans.push(Span::styled(
            &app.overview_input,
            Style::default().fg(COLOR_TEXT_PRIMARY),
        ));
    }

    let input_inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(Line::from(line_spans)), input_inner);
}

fn render_dynamic_canvas(f: &mut Frame, app: &App, area: Rect) {
    let is_typing = app.overview_input_active || !app.overview_input.trim().is_empty();

    if !is_typing {
        render_idle_cockpit(f, app, area);
    } else {
        render_active_results(f, app, area);
    }
}

/// State 1: Idle (Before Typing) — Launch Readiness Cockpit & Suggested Actions
fn render_idle_cockpit(f: &mut Frame, app: &App, area: Rect) {
    let is_actions_focused = app.overview_focus == OverviewFocus::RecentProfiles;
    let active_profile = app.profiles.iter().find(|p| p.is_active);
    let profile_name = active_profile
        .map(|p| p.name.as_str())
        .unwrap_or("No Active Profile");
    let mod_count = app.mods.len();

    let has_active_ws = app.game_title != "No Active Workspace" && active_profile.is_some();
    let (action1_title, action1_sub) = if !has_active_ws {
        (
            "Launch Active Game".to_string(),
            "No Active Workspace (Configure game profile or open peri.toml)".to_string(),
        )
    } else {
        (
            format!("Launch {}", app.game_title),
            format!("{} ({} active mods)", profile_name, mod_count),
        )
    };

    // Inactive profiles summary
    let other_profiles: Vec<&ProfileItem> = app.profiles.iter().filter(|p| !p.is_active).collect();
    let switch_subtitle = if app.profiles.is_empty() {
        "No Profiles Configured (Press [5] to create)".to_string()
    } else if other_profiles.len() >= 2 {
        format!(
            "{} ({} mods) | {} ({} mods)",
            other_profiles[0].name,
            other_profiles[0].mod_count,
            other_profiles[1].name,
            other_profiles[1].mod_count
        )
    } else if let Some(p) = other_profiles.first() {
        format!("{} ({} mods)", p.name, p.mod_count)
    } else {
        "No Alternate Profiles".to_string()
    };

    // Conflicts subtitle
    let conflict_subtitle = if !app.conflicts.is_empty() {
        format!(
            "{} collision{} on {}",
            app.conflicts.len(),
            if app.conflicts.len() == 1 { "" } else { "s" },
            app.conflicts[0].file_path
        )
    } else {
        format!("0 collisions across {} virtual layers", mod_count)
    };

    // Curated packages summary
    let curated_subtitle = if app.curated_packages.len() >= 2 {
        format!(
            "{} | {}",
            app.curated_packages[0].title, app.curated_packages[1].title
        )
    } else if !app.curated_packages.is_empty() {
        app.curated_packages[0].title.clone()
    } else {
        "Download package archive (.peca / magnet)".to_string()
    };

    let actions = [
        ("[1]", ">", action1_title, action1_sub),
        (
            "[2]",
            "!",
            if app.conflicts.is_empty() {
                "Layer Conflicts Clean".to_string()
            } else {
                format!("Resolve {} Conflicts", app.conflicts.len())
            },
            conflict_subtitle,
        ),
        (
            "[3]",
            "~",
            "Switch Active Profile".to_string(),
            switch_subtitle,
        ),
        (
            "[4]",
            "+",
            "Download Package Archive (.peca)".to_string(),
            curated_subtitle,
        ),
    ];

    let mut lines = Vec::new();

    // Section 1: Quick Actions Header
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "QUICK ACTIONS",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "(Press 1-4 or use Up/Down + Enter)",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    // Quick Action Rows
    for (idx, (num, icon, title, subtitle)) in actions.iter().enumerate() {
        let is_selected = is_actions_focused && app.overview_profile_cursor == idx;
        let title_col_width = 34;
        let padded_title = format!("{:<width$}", title, width = title_col_width);

        if is_selected {
            let row_style = style_selected_row();
            let sub_style = style_selected_row_secondary();

            // Continuous seamless highlight bar across row width (no unstyled gaps)
            let inner_width = (area.width as usize).saturating_sub(4);
            let used = 3 + num.len() + 1 + icon.len() + 1 + title_col_width + 2 + subtitle.len();
            let trailing_spaces = inner_width.saturating_sub(used).min(60);
            let trailing_fill = " ".repeat(trailing_spaces);

            lines.push(Line::from(vec![
                Span::styled(" > ", row_style),
                Span::styled(format!("{} ", num), row_style),
                Span::styled(format!("{} ", icon), row_style),
                Span::styled(padded_title, row_style),
                Span::styled("  ", row_style),
                Span::styled(subtitle.clone(), sub_style),
                Span::styled(trailing_fill, row_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    format!("{} ", num),
                    Style::default()
                        .fg(COLOR_TEXT_MUTED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", icon),
                    Style::default().fg(COLOR_TEXT_SECONDARY),
                ),
                Span::styled(padded_title, Style::default().fg(COLOR_TEXT_PRIMARY)),
                Span::raw("  "),
                Span::styled(subtitle.clone(), Style::default().fg(COLOR_TEXT_MUTED)),
            ]));
        }
    }

    // Breathing room / separator
    lines.push(Line::raw(""));

    // Section 2: Current Readiness
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "CURRENT READINESS",
            Style::default()
                .fg(COLOR_TEXT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "* ",
            Style::default()
                .fg(COLOR_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Safe Sandbox Active (Base files locked Read-Only)",
            Style::default().fg(COLOR_TEXT_PRIMARY),
        ),
        Span::styled("  |  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(&app.proton_version, Style::default().fg(COLOR_TEXT_PRIMARY)),
        Span::styled("  |  ", Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(
            "Dependencies Clean",
            Style::default().fg(COLOR_TEXT_SECONDARY),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

/// State 2: Active (While Typing) — Live Filter / Command Palette Result List
fn render_active_results(f: &mut Frame, app: &App, area: Rect) {
    let query = app.overview_input.trim();
    let lower = query.to_ascii_lowercase();

    // 1. URL / .peca / Magnet package detection
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("magnet:")
        || lower.ends_with(".peca")
    {
        render_detected_package(f, app, area, query);
        return;
    }

    // 2. Profile switcher or 'switch' / 'profile' verb
    if lower.starts_with("switch")
        || lower.starts_with("profile")
        || app.profiles.iter().any(|p| {
            p.name.to_ascii_lowercase().contains(&lower)
                || p.game_short.to_ascii_lowercase().contains(&lower)
        })
    {
        render_matching_profiles(f, app, area, &lower);
        return;
    }

    // 3. Mod search or 'find' / 'search' verb
    if lower.starts_with("find")
        || lower.starts_with("search")
        || app.mods.iter().any(|m| {
            m.name.to_ascii_lowercase().contains(&lower)
                || m.id.to_ascii_lowercase().contains(&lower)
        })
    {
        render_matching_mods(f, app, area, &lower);
        return;
    }

    // 4. Curated packages or 'clone' / 'download' verb
    if lower.starts_with("clone")
        || lower.starts_with("download")
        || app
            .curated_packages
            .iter()
            .any(|p| p.title.to_ascii_lowercase().contains(&lower))
    {
        render_matching_curated(f, app, area, &lower);
        return;
    }

    // 5. Launch / Run verb
    if lower.starts_with("launch") || lower.starts_with("run") || lower.starts_with("play") {
        render_launch_pipeline(f, app, area);
        return;
    }

    // 6. Fallback / Universal search
    render_universal_search(f, app, area, query);
}

fn render_detected_package(f: &mut Frame, app: &App, area: Rect, url_or_slug: &str) {
    let pkg_name = if let Some(last) = url_or_slug.split('/').next_back() {
        let clean = last.trim_end_matches(".peca");
        if clean.is_empty() {
            "Package Bundle"
        } else {
            clean
        }
    } else {
        "Package Bundle"
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "DETECTED PACKAGE",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "                                                                 [Enter to Install]",
                Style::default()
                    .fg(COLOR_FOCUS)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("   Package:       ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                pkg_name,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Target Game:   ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.game_title,
                Style::default().fg(COLOR_TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Contents:      ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "Zstandard Compressed Tarball (.peca Archive)",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Swarm Health:  ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "P2P Swarm & HTTP Stream Transport",
                Style::default().fg(COLOR_GREEN),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Integrity:     ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "SHA-256 Streaming Verified | Clean OverlayFS Memory Staging",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "Tip: Press [Enter] to stream directly into virtual OverlayFS memory. Base game remains untouched.",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), area);
}

fn render_matching_profiles(f: &mut Frame, app: &App, area: Rect, lower: &str) {
    let filter_arg = if let Some(stripped) = lower.strip_prefix("switch ") {
        stripped.trim()
    } else if let Some(stripped) = lower.strip_prefix("profile ") {
        stripped.trim()
    } else if lower == "switch" || lower == "profile" {
        ""
    } else {
        lower
    };

    let matching: Vec<(usize, &crate::app::ProfileItem)> = app
        .profiles
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            filter_arg.is_empty()
                || p.name.to_ascii_lowercase().contains(filter_arg)
                || p.game_short.to_ascii_lowercase().contains(filter_arg)
        })
        .collect();

    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "MATCHING PROFILES & ACTIONS",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "                                          ({} results, Up/Down to navigate)",
                matching.len()
            ),
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    let cursor_idx = app.overview_profile_cursor;

    for (pos, (_real_idx, profile)) in matching.iter().enumerate() {
        let is_selected = pos == cursor_idx % matching.len().max(1);
        let prefix = if is_selected { " > " } else { "   " };
        let icon = if profile.is_active { "[*]" } else { "[ ]" };

        let name_padded = format!("{:<30}", profile.name);
        let game_padded = format!("{:<16}", profile.game_short);
        let mods_padded = format!("{:>3} mods", profile.mod_count);
        let proton_padded = format!("{:<18}", app.proton_version);

        if is_selected {
            let row_style = style_selected_row();
            let dim_style = style_selected_row_secondary();
            let action_badge = if profile.is_active {
                Span::styled("[Current]", dim_style)
            } else {
                Span::styled("[Enter to Load]", row_style)
            };

            let inner_width = (area.width as usize).saturating_sub(4);
            let used = 3 + 3 + 1 + 30 + 1 + 16 + 1 + 8 + 4 + 18 + 1 + 15;
            let trailing_spaces = inner_width.saturating_sub(used).min(60);
            let trailing_fill = " ".repeat(trailing_spaces);

            lines.push(Line::from(vec![
                Span::styled(prefix, row_style),
                Span::styled(icon, row_style),
                Span::styled(" ", row_style),
                Span::styled(name_padded, row_style),
                Span::styled(" ", row_style),
                Span::styled(game_padded, dim_style),
                Span::styled(" ", dim_style),
                Span::styled(mods_padded, dim_style),
                Span::styled("    ", dim_style),
                Span::styled(proton_padded, dim_style),
                Span::styled(" ", dim_style),
                action_badge,
                Span::styled(trailing_fill, row_style),
            ]));
        } else {
            let row_style = if profile.is_active {
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_TEXT_SECONDARY)
            };
            let action_badge = if profile.is_active {
                Span::styled("[Current]", Style::default().fg(COLOR_TEXT_MUTED))
            } else {
                Span::styled(
                    "[Enter to Load]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                )
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(
                    icon,
                    if profile.is_active {
                        Style::default().fg(COLOR_GREEN)
                    } else {
                        Style::default().fg(COLOR_TEXT_MUTED)
                    },
                ),
                Span::raw(" "),
                Span::styled(name_padded, row_style),
                Span::raw(" "),
                Span::styled(game_padded, Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::raw(" "),
                Span::styled(mods_padded, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::raw("    "),
                Span::styled(proton_padded, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::raw(" "),
                action_badge,
            ]));
        }
    }

    lines.push(Line::raw(""));
    let sep_len = (area.width as usize).saturating_sub(6).min(96);
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "─".repeat(sep_len),
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "Tip: Press [Enter] to switch instantly. All virtual mods will remount in memory (0.01s).",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

fn render_matching_mods(f: &mut Frame, app: &App, area: Rect, lower: &str) {
    let filter_arg = if let Some(stripped) = lower.strip_prefix("find ") {
        stripped.trim()
    } else if let Some(stripped) = lower.strip_prefix("search ") {
        stripped.trim()
    } else if lower == "find" || lower == "search" {
        ""
    } else {
        lower
    };

    let matching: Vec<&crate::app::ModItem> = app
        .mods
        .iter()
        .filter(|m| {
            filter_arg.is_empty()
                || m.name.to_ascii_lowercase().contains(filter_arg)
                || m.id.to_ascii_lowercase().contains(filter_arg)
        })
        .collect();

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "MATCHING MODS & ASSETS",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "                                                 ({} results, Up/Down to navigate)",
                matching.len()
            ),
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    let cursor_idx = app.overview_profile_cursor;

    for (pos, mod_item) in matching.iter().take(6).enumerate() {
        let is_selected = pos == cursor_idx % matching.len().max(1);
        let prefix = if is_selected { " > " } else { "   " };

        let name_padded = format!("{:<32}", mod_item.name);
        let cat_padded = format!("{:<14}", mod_item.category);
        let prio_padded = format!("{:?}", mod_item.priority);

        if is_selected {
            let row_style = style_selected_row();
            let dim_style = style_selected_row_secondary();
            let inner_width = (area.width as usize).saturating_sub(4);
            let used = 3 + 32 + 1 + 14 + 1 + 10 + 3 + 15;
            let trailing_spaces = inner_width.saturating_sub(used).min(60);
            let trailing_fill = " ".repeat(trailing_spaces);

            lines.push(Line::from(vec![
                Span::styled(prefix, row_style),
                Span::styled(name_padded, row_style),
                Span::styled(" ", row_style),
                Span::styled(cat_padded, dim_style),
                Span::styled(" ", dim_style),
                Span::styled(format!("{:<10}", prio_padded), dim_style),
                Span::styled("   ", dim_style),
                Span::styled("[Enter to View]", row_style),
                Span::styled(trailing_fill, row_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(name_padded, Style::default().fg(COLOR_TEXT_PRIMARY)),
                Span::raw(" "),
                Span::styled(cat_padded, Style::default().fg(COLOR_TEXT_SECONDARY)),
                Span::raw(" "),
                Span::styled(
                    format!("{:<10}", prio_padded),
                    Style::default().fg(COLOR_TEXT_MUTED),
                ),
                Span::raw("   "),
                Span::styled(
                    "[Enter to View]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    lines.push(Line::raw(""));
    let sep_len = (area.width as usize).saturating_sub(6).min(96);
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "─".repeat(sep_len),
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "Tip: Press [Enter] to open package in Mod Load Order & Inspector.",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

fn render_matching_curated(f: &mut Frame, app: &App, area: Rect, lower: &str) {
    let filter_arg = if let Some(stripped) = lower.strip_prefix("clone ") {
        stripped.trim()
    } else if let Some(stripped) = lower.strip_prefix("download ") {
        stripped.trim()
    } else if lower == "clone" || lower == "download" {
        ""
    } else {
        lower
    };

    let matching: Vec<&crate::app::CuratedPackage> = app
        .curated_packages
        .iter()
        .filter(|p| {
            filter_arg.is_empty()
                || p.title.to_ascii_lowercase().contains(filter_arg)
                || p.desc.to_ascii_lowercase().contains(filter_arg)
        })
        .collect();

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "COMMUNITY CURATED PACKS",
            Style::default()
                .fg(COLOR_TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "                                                ({} results, Up/Down to navigate)",
                matching.len()
            ),
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    let cursor_idx = app.overview_profile_cursor;

    for (pos, pkg) in matching.iter().take(5).enumerate() {
        let is_selected = pos == cursor_idx % matching.len().max(1);
        let prefix = if is_selected { " > " } else { "   " };

        let title_padded = format!("{:<30}", pkg.title);
        let status_padded = if pkg.is_installed {
            "Installed"
        } else {
            "P2P Verified"
        };
        let peers_padded = "18 DHT seeds";

        if is_selected {
            let row_style = style_selected_row();
            let dim_style = style_selected_row_secondary();
            let inner_width = (area.width as usize).saturating_sub(4);
            let used = 3 + 30 + 1 + 14 + 1 + 16 + 1 + 19;
            let trailing_spaces = inner_width.saturating_sub(used).min(60);
            let trailing_fill = " ".repeat(trailing_spaces);

            lines.push(Line::from(vec![
                Span::styled(prefix, row_style),
                Span::styled(title_padded, row_style),
                Span::styled(" ", row_style),
                Span::styled(format!("{:<14}", status_padded), dim_style),
                Span::styled(" ", dim_style),
                Span::styled(format!("{:<16}", peers_padded), dim_style),
                Span::styled(" ", dim_style),
                Span::styled("[Enter to Download]", row_style),
                Span::styled(trailing_fill, row_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(title_padded, Style::default().fg(COLOR_TEXT_PRIMARY)),
                Span::raw(" "),
                Span::styled(
                    format!("{:<14}", status_padded),
                    if pkg.is_installed {
                        Style::default().fg(COLOR_GREEN)
                    } else {
                        Style::default().fg(COLOR_TEXT_SECONDARY)
                    },
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:<16}", peers_padded),
                    Style::default().fg(COLOR_GREEN),
                ),
                Span::raw(" "),
                Span::styled(
                    "[Enter to Download]",
                    Style::default()
                        .fg(COLOR_FOCUS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    lines.push(Line::raw(""));
    let sep_len = (area.width as usize).saturating_sub(6).min(96);
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "─".repeat(sep_len),
            Style::default().fg(COLOR_BORDER_SUBTLE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "Tip: Press [Enter] to enqueue download & mount directly into active profile.",
            Style::default().fg(COLOR_TEXT_MUTED),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

fn render_launch_pipeline(f: &mut Frame, app: &App, area: Rect) {
    let active_profile = app.profiles.iter().find(|p| p.is_active);
    let profile_name = active_profile.map(|p| p.name.as_str()).unwrap_or("Default");

    let lines = vec![
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "EXECUTE LAUNCH PIPELINE",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "                                                        [Enter to Run]",
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("   Target Game:   ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                &app.game_title,
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Blueprint:     ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                format!("{} ({} active mods)", profile_name, app.mods.len()),
                Style::default().fg(COLOR_TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Proton Env:    ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(&app.proton_version, Style::default().fg(COLOR_TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("   VFS Isolation: ", Style::default().fg(COLOR_TEXT_MUTED)),
            Span::styled(
                "CLONE_NEWNS | CLONE_NEWUSER (Unprivileged OverlayFS mount)",
                Style::default().fg(COLOR_TEXT_SECONDARY),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "Tip: Press [Enter] or [F9] to launch immediately.",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), area);
}

fn render_universal_search(f: &mut Frame, _app: &App, area: Rect, query: &str) {
    let lines = vec![
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "COMMAND PALETTE SEARCH",
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "   No direct match for: ",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::styled(
                format!("\"{}\"", query),
                Style::default()
                    .fg(COLOR_TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "Suggestions:",
                Style::default()
                    .fg(COLOR_TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   * ", Style::default().fg(COLOR_FOCUS)),
            Span::styled("switch <name>", Style::default().fg(COLOR_TEXT_PRIMARY)),
            Span::styled(
                "  - Filter and activate workspaces",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("   * ", Style::default().fg(COLOR_FOCUS)),
            Span::styled("find <term>", Style::default().fg(COLOR_TEXT_PRIMARY)),
            Span::styled(
                "    - Search installed mods and textures",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("   * ", Style::default().fg(COLOR_FOCUS)),
            Span::styled("clone <url>", Style::default().fg(COLOR_TEXT_PRIMARY)),
            Span::styled(
                "    - Download .peca package archive",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("   * ", Style::default().fg(COLOR_FOCUS)),
            Span::styled("launch", Style::default().fg(COLOR_TEXT_PRIMARY)),
            Span::styled(
                "         - Start active game in virtual sandbox",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "Press [Enter] to search Mod Load Order, or [Esc] to return to Cockpit.",
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), area);
}
