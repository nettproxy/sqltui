use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{ActivePanel, App, AppMode, Tab};

pub fn handle_event(app: &mut App, key: KeyEvent) {
    match app.mode {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Query  => handle_query_mode(app, key),
        AppMode::Confirm => handle_confirm(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Open file prompt
    if app.open_prompt {
        match key.code {
            KeyCode::Esc => {
                app.open_prompt = false;
                app.open_input.clear();
            }
            KeyCode::Enter => {
                let path = app.open_input.clone();
                app.open_prompt = false;
                app.open_input.clear();
                if let Err(e) = app.open_database(&path) {
                    app.status_message = format!("Error: {}", e);
                }
            }
            KeyCode::Backspace => { app.open_input.pop(); }
            KeyCode::Char(c) => { app.open_input.push(c); }
            _ => {}
        }
        return;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Open file
        KeyCode::Char('o') => {
            app.open_prompt = true;
            app.open_input.clear();
        }

        // Panel switching
        KeyCode::Tab => cycle_panel(app),
        KeyCode::BackTab => cycle_panel_back(app),
        KeyCode::Char('1') => { app.active_panel = ActivePanel::Sidebar; }
        KeyCode::Char('2') => { app.active_panel = ActivePanel::Table; }
        KeyCode::Char('3') => { app.active_panel = ActivePanel::Query; }

        // Tab switching (data/schema/query)
        KeyCode::Char('d') => { app.active_tab = Tab::Data; }
        KeyCode::Char('s') => { app.active_tab = Tab::Schema; }
        KeyCode::Char('/') | KeyCode::Char('e') => {
            app.active_tab = Tab::Query;
            app.active_panel = ActivePanel::Query;
            app.mode = AppMode::Query;
        }

        // Navigation in active panel
        KeyCode::Up | KeyCode::Char('k') => navigate_up(app),
        KeyCode::Down | KeyCode::Char('j') => navigate_down(app),
        KeyCode::Left | KeyCode::Char('h') => navigate_left(app),
        KeyCode::Right | KeyCode::Char('l') => navigate_right(app),

        // Page navigation
        KeyCode::PageDown | KeyCode::Char('n') => {
            if app.active_panel == ActivePanel::Table {
                app.load_next_page();
            }
        }
        KeyCode::PageUp | KeyCode::Char('p') => {
            if app.active_panel == ActivePanel::Table {
                app.load_prev_page();
            }
        }

        // Enter — load table from sidebar
        KeyCode::Enter => {
            if app.active_panel == ActivePanel::Sidebar {
                app.load_current_selection();
                app.active_panel = ActivePanel::Table;
                app.active_tab = Tab::Data;
            }
        }

        // Refresh
        KeyCode::Char('r') => {
            if let Some(table) = app.current_table.clone() {
                app.load_table(&table);
                app.status_message = "Refreshed".to_string();
            }
        }

        // Help
        KeyCode::Char('?') => {
            app.status_message =
                "q:quit  o:open  Tab:panel  d:data  s:schema  /:query  j/k:move  n/p:page  r:refresh"
                    .to_string();
        }

        _ => {}
    }
}

fn handle_query_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.active_panel = ActivePanel::Table;
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+Enter to execute
            app.execute_custom_query();
            app.mode = AppMode::Normal;
        }
        KeyCode::F(5) => {
            app.execute_custom_query();
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('\n') if key.modifiers.contains(KeyModifiers::ALT) => {
            app.execute_custom_query();
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            if app.query_cursor > 0 {
                app.query_cursor -= 1;
                app.query_input.remove(app.query_cursor);
            }
        }
        KeyCode::Delete => {
            if app.query_cursor < app.query_input.len() {
                app.query_input.remove(app.query_cursor);
            }
        }
        KeyCode::Left => {
            if app.query_cursor > 0 { app.query_cursor -= 1; }
        }
        KeyCode::Right => {
            if app.query_cursor < app.query_input.len() { app.query_cursor += 1; }
        }
        KeyCode::Home => { app.query_cursor = 0; }
        KeyCode::End => { app.query_cursor = app.query_input.len(); }
        KeyCode::Up => {
            // history up
            if app.query_history.is_empty() { return; }
            let new_idx = match app.query_history_index {
                None => app.query_history.len() - 1,
                Some(0) => 0,
                Some(i) => i - 1,
            };
            app.query_history_index = Some(new_idx);
            app.query_input = app.query_history[new_idx].clone();
            app.query_cursor = app.query_input.len();
        }
        KeyCode::Down => {
            // history down
            match app.query_history_index {
                None => {}
                Some(i) if i + 1 >= app.query_history.len() => {
                    app.query_history_index = None;
                    app.query_input.clear();
                    app.query_cursor = 0;
                }
                Some(i) => {
                    app.query_history_index = Some(i + 1);
                    app.query_input = app.query_history[i + 1].clone();
                    app.query_cursor = app.query_input.len();
                }
            }
        }
        KeyCode::Char(c) => {
            app.query_input.insert(app.query_cursor, c);
            app.query_cursor += 1;
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => { app.mode = AppMode::Normal; }
        KeyCode::Char('n') | KeyCode::Esc  => { app.mode = AppMode::Normal; }
        _ => {}
    }
}

fn navigate_up(app: &mut App) {
    match app.active_panel {
        ActivePanel::Sidebar => {
            if app.sidebar_index > 0 {
                app.sidebar_index -= 1;
                clamp_sidebar_scroll(app);
            }
        }
        ActivePanel::Table => {
            let rows = app.query_result.as_ref().map(|r| r.rows.len()).unwrap_or(0);
            if rows > 0 && app.table_selected_row > 0 {
                app.table_selected_row -= 1;
            }
        }
        ActivePanel::Schema => {
            if app.schema_scroll > 0 { app.schema_scroll -= 1; }
        }
        ActivePanel::Query => {}
    }
}

fn navigate_down(app: &mut App) {
    match app.active_panel {
        ActivePanel::Sidebar => {
            let total = app.sidebar_total();
            if total > 0 && app.sidebar_index + 1 < total {
                app.sidebar_index += 1;
                clamp_sidebar_scroll(app);
            }
        }
        ActivePanel::Table => {
            let rows = app.query_result.as_ref().map(|r| r.rows.len()).unwrap_or(0);
            if rows > 0 && app.table_selected_row + 1 < rows {
                app.table_selected_row += 1;
            }
        }
        ActivePanel::Schema => {
            app.schema_scroll += 1;
        }
        ActivePanel::Query => {}
    }
}

fn navigate_left(app: &mut App) {
    if app.active_panel == ActivePanel::Table && app.table_scroll_x > 0 {
        app.table_scroll_x -= 1;
    }
}

fn navigate_right(app: &mut App) {
    if app.active_panel == ActivePanel::Table {
        let col_count = app.query_result.as_ref().map(|r| r.columns.len()).unwrap_or(0);
        if app.table_scroll_x + 1 < col_count {
            app.table_scroll_x += 1;
        }
    }
}

fn cycle_panel(app: &mut App) {
    app.active_panel = match app.active_panel {
        ActivePanel::Sidebar => ActivePanel::Table,
        ActivePanel::Table   => {
            if app.active_tab == Tab::Schema { ActivePanel::Schema }
            else if app.active_tab == Tab::Query { ActivePanel::Query }
            else { ActivePanel::Sidebar }
        }
        ActivePanel::Schema  => ActivePanel::Sidebar,
        ActivePanel::Query   => ActivePanel::Sidebar,
    };
}

fn cycle_panel_back(app: &mut App) {
    app.active_panel = match app.active_panel {
        ActivePanel::Sidebar => ActivePanel::Table,
        ActivePanel::Table   => ActivePanel::Sidebar,
        ActivePanel::Schema  => ActivePanel::Table,
        ActivePanel::Query   => ActivePanel::Table,
    };
}

fn clamp_sidebar_scroll(app: &mut App) {
    // Will be adjusted in draw based on viewport height
    // Simple clamp: ensure scroll doesn't go past selected
    if app.sidebar_index < app.sidebar_scroll {
        app.sidebar_scroll = app.sidebar_index;
    }
}
