use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use miette::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::panic;
use std::time::Duration;

mod action;
mod app;
pub mod autocomplete;
mod event;
mod theme;
mod ui;
mod views;

use action::{Action, ModsFocus, ProfilesFocus, Tab};
use app::App;
use event::{Event, EventHandler};

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook for clean terminal restoration
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode().map_err(|e| miette::miette!("Failed to enable raw mode: {}", e))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| miette::miette!("Failed to enter alternate screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| miette::miette!("Failed to initialize terminal: {}", e))?;

    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_millis(50));

    // Main TUI Event Loop
    while !app.should_quit {
        terminal
            .draw(|f| ui::render_ui(f, &app))
            .map_err(|e| miette::miette!("Failed to draw frame: {}", e))?;

        if let Some(event) = events.next().await {
            match event {
                Event::Tick => {
                    app.handle_action(Action::Tick);
                }
                Event::Resize(_, _) => {
                    app.handle_action(Action::Render);
                }
                Event::Key(key) => {
                    handle_key_event(&mut app, key);
                }
            }
        }
    }

    // Teardown terminal cleanly
    disable_raw_mode().map_err(|e| miette::miette!("Failed to disable raw mode: {}", e))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| miette::miette!("Failed to leave alternate screen: {}", e))?;
    terminal
        .show_cursor()
        .map_err(|e| miette::miette!("Failed to show cursor: {}", e))?;

    Ok(())
}

fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) {
    // Always allow Ctrl+C to quit cleanly
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.handle_action(Action::Quit);
        return;
    }

    // 1. Modal Dialog Handling
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                app.handle_action(Action::CloseModal);
            }
            _ => {}
        }
        return;
    }

    // 2. Active Text Input Mode (Readline / CUA Standard Shortcuts + Tab Focus Cycling)
    if app.is_any_input_active() {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => app.handle_action(Action::InputClear),
                KeyCode::Char('w') => app.handle_action(Action::InputDeleteWord),
                KeyCode::Char('h') => app.handle_action(Action::InputBackspace),
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab | KeyCode::Right | KeyCode::End => {
                if app.accept_autocomplete() {
                    return;
                }
                if key.code == KeyCode::Tab {
                    app.handle_action(Action::FocusNext);
                }
            }
            KeyCode::BackTab => {
                app.handle_action(Action::FocusPrev);
            }
            KeyCode::Esc => {
                app.handle_action(Action::CancelInput);
            }
            KeyCode::Enter => {
                app.handle_action(Action::SubmitInput);
            }
            KeyCode::Backspace => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    app.handle_action(Action::InputDeleteWord);
                } else {
                    app.handle_action(Action::InputBackspace);
                }
            }
            KeyCode::Delete => {
                app.handle_action(Action::InputClear);
            }
            KeyCode::Up => {
                if app.overview_input_active {
                    app.handle_action(Action::CursorUp);
                }
            }
            KeyCode::Down => {
                if app.overview_input_active {
                    app.handle_action(Action::CursorDown);
                }
            }
            KeyCode::Char(c) => {
                app.handle_action(Action::InputChar(c));
            }
            _ => {}
        }
        return;
    }

    // 3. Control-Modified Navigation Shortcuts (Ctrl+D / Ctrl+U / Ctrl+L)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => app.handle_action(Action::CursorPageDown),
            KeyCode::Char('u') => app.handle_action(Action::CursorPageUp),
            KeyCode::Char('l') => {
                if app.active_tab == Tab::Mods {
                    app.handle_action(Action::ClearSearchAndFilters);
                }
            }
            KeyCode::Char('w') => app.handle_action(Action::FocusNext),
            _ => {}
        }
        return;
    }

    // 4. Alt-Modified Shortcuts (Alt+Up/Down to reorder, Alt+Left/Right to switch page)
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Up => {
                if app.active_tab == Tab::Mods {
                    app.handle_action(Action::MoveModUp);
                }
                return;
            }
            KeyCode::Down => {
                if app.active_tab == Tab::Mods {
                    app.handle_action(Action::MoveModDown);
                }
                return;
            }
            KeyCode::Left => {
                app.handle_action(Action::PrevTab);
                return;
            }
            KeyCode::Right => {
                app.handle_action(Action::NextTab);
                return;
            }
            _ => {}
        }
    }

    // 5. Standard Global & View-Specific Keyboard Navigation
    match key.code {
        // App lifecycle
        KeyCode::Char('q') => app.handle_action(Action::Quit),
        KeyCode::Esc => {
            if app.active_tab == Tab::Mods
                && (!app.search_query.is_empty() || app.category_filter.is_some())
            {
                app.handle_action(Action::ClearSearchAndFilters);
            } else if app.active_tab != Tab::Overview {
                app.handle_action(Action::SetTab(Tab::Overview));
            } else {
                app.handle_action(Action::Quit);
            }
        }
        KeyCode::Char('?') | KeyCode::F(1) => app.handle_action(Action::ToggleHelp),

        // Selectable Area Focus Cycling within the Current Page (Tab / Shift+Tab)
        KeyCode::Tab => app.handle_action(Action::FocusNext),
        KeyCode::BackTab => app.handle_action(Action::FocusPrev),

        // Top-level Page Switching (1-5, 0, or [ / ])
        KeyCode::Char('1') => app.handle_action(Action::SetTab(Tab::Overview)),
        KeyCode::Char('2') => app.handle_action(Action::SetTab(Tab::Mods)),
        KeyCode::Char('3') => app.handle_action(Action::SetTab(Tab::Conflicts)),
        KeyCode::Char('4') => app.handle_action(Action::SetTab(Tab::Downloads)),
        KeyCode::Char('5') => app.handle_action(Action::SetTab(Tab::Profiles)),
        KeyCode::Char('0') => app.handle_action(Action::SetTab(Tab::Overview)),
        KeyCode::Char('[') => app.handle_action(Action::PrevTab),
        KeyCode::Char(']') => app.handle_action(Action::NextTab),

        // Vertical List Navigation (Arrows & Vim j/k/g/G)
        KeyCode::Up | KeyCode::Char('k') => app.handle_action(Action::CursorUp),
        KeyCode::Down | KeyCode::Char('j') => app.handle_action(Action::CursorDown),
        KeyCode::PageUp => app.handle_action(Action::CursorPageUp),
        KeyCode::PageDown => app.handle_action(Action::CursorPageDown),
        KeyCode::Home | KeyCode::Char('g') => app.handle_action(Action::CursorHome),
        KeyCode::End | KeyCode::Char('G') => app.handle_action(Action::CursorEnd),

        // Reorder Mod Load Order with Shift+K / Shift+J
        KeyCode::Char('K') => {
            if app.active_tab == Tab::Mods {
                app.handle_action(Action::MoveModUp);
            }
        }
        KeyCode::Char('J') => {
            if app.active_tab == Tab::Mods {
                app.handle_action(Action::MoveModDown);
            }
        }

        // Horizontal Navigation (Left/Right & h/l)
        KeyCode::Left | KeyCode::Char('h') => match app.active_tab {
            Tab::Mods if app.mods_focus == ModsFocus::Categories => {
                app.handle_action(Action::CycleCategoryReverse);
            }
            Tab::Profiles if app.profiles_focus == ProfilesFocus::GameSelector => {
                app.handle_action(Action::CycleTargetGameReverse);
            }
            Tab::Conflicts => {
                if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
                    let id = card.option_a.mod_id.clone();
                    app.handle_action(Action::ResolveConflictChoice {
                        index: app.active_conflict_cursor,
                        winner_id: id,
                    });
                }
            }
            _ => app.handle_action(Action::FocusPrev),
        },
        KeyCode::Right | KeyCode::Char('l') => match app.active_tab {
            Tab::Mods if app.mods_focus == ModsFocus::Categories => {
                app.handle_action(Action::CycleCategory);
            }
            Tab::Profiles if app.profiles_focus == ProfilesFocus::GameSelector => {
                app.handle_action(Action::CycleTargetGame);
            }
            Tab::Conflicts => {
                if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
                    let id = card.option_b.mod_id.clone();
                    app.handle_action(Action::ResolveConflictChoice {
                        index: app.active_conflict_cursor,
                        winner_id: id,
                    });
                }
            }
            _ => app.handle_action(Action::FocusNext),
        },

        // Spacebar & Enter Context Activation on Focused Area
        KeyCode::Char(' ') | KeyCode::Enter => {
            app.handle_action(Action::ActivateFocused);
        }

        // Priority & Pause Cycling
        KeyCode::Char('p') | KeyCode::Char('+') | KeyCode::Char('=') => match app.active_tab {
            Tab::Overview => app.handle_action(Action::SetTab(Tab::Profiles)),
            Tab::Mods => app.handle_action(Action::CyclePriority),
            Tab::Downloads => app.handle_action(Action::TogglePauseDownload),
            _ => {}
        },
        KeyCode::Char('P') | KeyCode::Char('-') => {
            if app.active_tab == Tab::Mods {
                app.handle_action(Action::CyclePriorityReverse);
            }
        }

        // Category / Target Game Cycling / Overview Clone Input
        KeyCode::Char('c') => match app.active_tab {
            Tab::Overview => app.handle_action(Action::StartOverviewInput),
            Tab::Mods => app.handle_action(Action::CycleCategory),
            Tab::Profiles => app.handle_action(Action::CycleTargetGame),
            _ => {}
        },
        KeyCode::Char('C') => match app.active_tab {
            Tab::Mods => app.handle_action(Action::CycleCategoryReverse),
            Tab::Profiles => app.handle_action(Action::CycleTargetGameReverse),
            _ => {}
        },
        KeyCode::Char('s') => {
            if app.active_tab == Tab::Overview {
                app.handle_action(Action::SetTab(Tab::Mods));
                app.handle_action(Action::StartSearch);
            }
        }
        KeyCode::Char('t') => {
            if app.active_tab == Tab::Profiles {
                app.handle_action(Action::CycleTargetGame);
            }
        }
        KeyCode::Char('T') => {
            if app.active_tab == Tab::Profiles {
                app.handle_action(Action::CycleTargetGameReverse);
            }
        }

        // Search / Input Focus ('/')
        KeyCode::Char('/') => match app.active_tab {
            Tab::Overview => app.handle_action(Action::StartOverviewInput),
            Tab::Mods => app.handle_action(Action::StartSearch),
            Tab::Downloads => app.handle_action(Action::StartDownloadInput),
            Tab::Profiles => app.handle_action(Action::StartProfileInput),
            Tab::Conflicts => {}
        },

        // Inspector / Input Shortcuts
        KeyCode::Char('i') => match app.active_tab {
            Tab::Mods | Tab::Conflicts | Tab::Downloads => {
                app.handle_action(Action::ToggleInspector)
            }
            _ => {}
        },
        KeyCode::Char('u') => {
            if app.active_tab == Tab::Downloads {
                app.handle_action(Action::StartDownloadInput);
            }
        }
        KeyCode::Char('n') => match app.active_tab {
            Tab::Profiles => app.handle_action(Action::StartProfileInput),
            Tab::Downloads => app.handle_action(Action::StartDownloadInput),
            _ => {}
        },
        KeyCode::Char('f') | KeyCode::Char('x') => {
            if app.active_tab == Tab::Profiles {
                app.handle_action(Action::FixDiagnostic);
            }
        }

        // Conflict Choices & Toggle All Mods
        KeyCode::Char('a') => match app.active_tab {
            Tab::Conflicts => {
                if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
                    let id = card.option_a.mod_id.clone();
                    app.handle_action(Action::ResolveConflictChoice {
                        index: app.active_conflict_cursor,
                        winner_id: id,
                    });
                }
            }
            Tab::Mods => app.handle_action(Action::ToggleAllMods),
            _ => {}
        },
        KeyCode::Char('b') => {
            if app.active_tab == Tab::Conflicts {
                if let Some(card) = app.conflicts.get(app.active_conflict_cursor) {
                    let id = card.option_b.mod_id.clone();
                    app.handle_action(Action::ResolveConflictChoice {
                        index: app.active_conflict_cursor,
                        winner_id: id,
                    });
                }
            }
        }
        KeyCode::Char('r') => {
            if app.active_tab == Tab::Conflicts {
                app.handle_action(Action::AutoResolveConflicts);
            }
        }

        // Launch Game (F9 or Shift+L)
        KeyCode::F(9) | KeyCode::Char('L') => {
            app.handle_action(Action::LaunchGame);
        }

        _ => {}
    }
}
