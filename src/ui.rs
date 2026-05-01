use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem,
        Paragraph, Row, Table, TableState, Tabs, Wrap,
    },
};

use crate::app::{ActivePanel, App, AppMode, Tab};

// ── Color palette ───────────────────────────────────────────────────────────
const C_BG:          Color = Color::Rgb(18, 18, 28);
const C_PANEL_BG:    Color = Color::Rgb(24, 24, 38);
const C_ACTIVE:      Color = Color::Rgb(130, 190, 255);
const C_INACTIVE:    Color = Color::Rgb(80, 90, 110);
const C_HEADER:      Color = Color::Rgb(255, 195, 100);
const C_ROW_SEL:     Color = Color::Rgb(38, 50, 75);
const C_ROW_ALT:     Color = Color::Rgb(22, 26, 40);
const C_TEXT:        Color = Color::Rgb(210, 215, 230);
const C_DIM:         Color = Color::Rgb(100, 108, 130);
const C_ERROR:       Color = Color::Rgb(255, 100, 100);
const C_SUCCESS:     Color = Color::Rgb(100, 230, 140);
const C_TABLE_NAME:  Color = Color::Rgb(180, 160, 255);
const C_VIEW_NAME:   Color = Color::Rgb(100, 210, 200);
const C_COUNT:       Color = Color::Rgb(100, 140, 200);
const C_TAB_ACTIVE:  Color = Color::Rgb(130, 190, 255);
const C_NULL:        Color = Color::Rgb(100, 100, 130);

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(C_BG)),
        area,
    );

    // Top-level layout: sidebar | main
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(area);

    // Main vertical: tabs | content | status
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // title bar
            Constraint::Length(3),  // tabs
            Constraint::Min(0),     // content
            Constraint::Length(3),  // query input (always shown)
            Constraint::Length(1),  // status bar
        ])
        .split(h_chunks[1]);

    draw_titlebar(f, app, area);
    draw_sidebar(f, app, h_chunks[0]);
    draw_tabs(f, app, v_chunks[1]);

    match app.active_tab {
        Tab::Data   => draw_table(f, app, v_chunks[2]),
        Tab::Schema => draw_schema(f, app, v_chunks[2]),
        Tab::Query  => draw_query_results(f, app, v_chunks[2]),
    }

    draw_query_bar(f, app, v_chunks[3]);
    draw_statusbar(f, app, v_chunks[4]);

    if app.open_prompt {
        draw_open_dialog(f, app, area);
    }
}

fn draw_titlebar(f: &mut Frame, app: &App, area: Rect) {
    let title_area = Rect { height: 1, ..area };
    let db_name = app.db_path.as_deref().unwrap_or("No database");
    let title = Line::from(vec![
        Span::styled(" sqltui ", Style::default().fg(C_ACTIVE).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(C_INACTIVE)),
        Span::styled(db_name, Style::default().fg(C_TEXT)),
    ]);
    f.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::Rgb(14, 14, 22))),
        title_area,
    );
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Sidebar;
    let border_style = if is_active {
        Style::default().fg(C_ACTIVE)
    } else {
        Style::default().fg(C_INACTIVE)
    };

    let inner_height = area.height.saturating_sub(4) as usize; // account for borders + section headers
    // Clamp scroll
    if app.sidebar_index >= app.sidebar_scroll + inner_height {
        app.sidebar_scroll = app.sidebar_index - inner_height + 1;
    }
    if app.sidebar_index < app.sidebar_scroll {
        app.sidebar_scroll = app.sidebar_index;
    }

    let mut items: Vec<ListItem> = Vec::new();

    // Tables section header
    items.push(ListItem::new(Line::from(vec![
        Span::styled("  TABLES ", Style::default().fg(C_DIM).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("({})", app.tables.len()),
            Style::default().fg(C_COUNT),
        ),
    ])));

    for (i, t) in app.tables.iter().enumerate() {
        let is_sel = i == app.sidebar_index;
        let prefix = if is_sel { "▶ " } else { "  " };
        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(C_ACTIVE)),
            Span::styled(t.name.clone(), Style::default().fg(
                if is_sel { C_TABLE_NAME } else { C_TEXT }
            ).add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() })),
            Span::styled(
                format!(" ({})", t.row_count),
                Style::default().fg(C_COUNT),
            ),
        ]);
        items.push(ListItem::new(line));
    }

    // Views section header
    if !app.views.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  VIEWS ", Style::default().fg(C_DIM).add_modifier(Modifier::BOLD)),
            Span::styled(format!("({})", app.views.len()), Style::default().fg(C_COUNT)),
        ])));
        for (i, v) in app.views.iter().enumerate() {
            let abs_idx = app.tables.len() + i;
            let is_sel = abs_idx == app.sidebar_index;
            let prefix = if is_sel { "▶ " } else { "  " };
            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(C_ACTIVE)),
                Span::styled(v.clone(), Style::default().fg(
                    if is_sel { C_VIEW_NAME } else { C_TEXT }
                ).add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() })),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let block = Block::default()
        .title(Span::styled(" 󰆼 Tables ", Style::default().fg(C_ACTIVE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(C_PANEL_BG));

    // We use a plain List (not stateful) because we manually build the items with selection markers
    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(C_TEXT));

    f.render_widget(list, area);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![" Data ", " Schema ", " Query "];
    let selected = match app.active_tab {
        Tab::Data   => 0,
        Tab::Schema => 1,
        Tab::Query  => 2,
    };

    let table_name = app.current_table.as_deref().unwrap_or("—");

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(C_INACTIVE))
                .title(Span::styled(
                    format!(" {} ", table_name),
                    Style::default().fg(C_TABLE_NAME).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(C_PANEL_BG)),
        )
        .style(Style::default().fg(C_INACTIVE))
        .highlight_style(
            Style::default()
                .fg(C_TAB_ACTIVE)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled("│", Style::default().fg(C_INACTIVE)));

    f.render_widget(tabs, area);
}

fn draw_table(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Table;
    let border_style = if is_active {
        Style::default().fg(C_ACTIVE)
    } else {
        Style::default().fg(C_INACTIVE)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(C_PANEL_BG));

    let Some(result) = &app.query_result else {
        let msg = if app.db.is_some() {
            "Select a table from the sidebar"
        } else {
            "No database open. Press 'o' to open one."
        };
        f.render_widget(
            Paragraph::new(msg)
                .block(block)
                .style(Style::default().fg(C_DIM)),
            area,
        );
        return;
    };

    if let Some(err) = &result.error.clone() {
        f.render_widget(
            Paragraph::new(format!("Error: {}", err))
                .block(block)
                .style(Style::default().fg(C_ERROR))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    if result.columns.is_empty() {
        f.render_widget(
            Paragraph::new("Query executed successfully (no rows returned)")
                .block(block)
                .style(Style::default().fg(C_SUCCESS)),
            area,
        );
        return;
    }

    let scroll_x = app.table_scroll_x;
    let visible_cols = &result.columns[scroll_x..];

    // Build header
    let header_cells: Vec<Cell> = visible_cols
        .iter()
        .map(|h| {
            Cell::from(h.clone()).style(
                Style::default()
                    .fg(C_HEADER)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(30, 30, 50)))
        .height(1);

    // Inner height for scroll clamping
    let inner_h = area.height.saturating_sub(4) as usize;
    let selected = app.table_selected_row;

    // Build rows
    let rows: Vec<Row> = result
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_selected = i == selected;
            let bg = if is_selected {
                C_ROW_SEL
            } else if i % 2 == 0 {
                C_PANEL_BG
            } else {
                C_ROW_ALT
            };

            let cells: Vec<Cell> = row[scroll_x..]
                .iter()
                .map(|val| {
                    let style = if val == "NULL" {
                        Style::default().fg(C_NULL).add_modifier(Modifier::ITALIC)
                    } else if is_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(C_TEXT)
                    };
                    Cell::from(val.clone()).style(style)
                })
                .collect();

            Row::new(cells).style(Style::default().bg(bg)).height(1)
        })
        .collect();

    // Column widths
    let col_count = visible_cols.len();
    let constraints: Vec<Constraint> = visible_cols
        .iter()
        .map(|_| Constraint::Min(12))
        .collect();

    let page_info = format!(
        " rows {}-{} | col {}/{} ",
        app.table_row_offset,
        app.table_row_offset + result.rows.len(),
        scroll_x + 1,
        result.columns.len()
    );

    let table = Table::new(rows, constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(page_info, Style::default().fg(C_DIM)))
                .style(Style::default().bg(C_PANEL_BG)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    let mut state = TableState::default();
    state.select(Some(selected));

    f.render_stateful_widget(table, area, &mut state);
}

fn draw_schema(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Schema;
    let border_style = if is_active {
        Style::default().fg(C_ACTIVE)
    } else {
        Style::default().fg(C_INACTIVE)
    };

    let text = if app.schema_text.is_empty() {
        Text::from(Span::styled(
            "No schema available",
            Style::default().fg(C_DIM),
        ))
    } else {
        // Very simple SQL keyword highlighting
        let mut lines: Vec<Line> = Vec::new();
        for line_str in app.schema_text.lines() {
            lines.push(highlight_sql_line(line_str));
        }
        Text::from(lines)
    };

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(" Schema ", Style::default().fg(C_ACTIVE)))
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(C_PANEL_BG)),
        )
        .scroll((app.schema_scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn highlight_sql_line(line: &str) -> Line<'static> {
    let keywords = [
        "CREATE", "TABLE", "VIEW", "INDEX", "PRIMARY", "KEY", "NOT", "NULL",
        "UNIQUE", "INTEGER", "TEXT", "REAL", "BLOB", "REFERENCES", "ON",
        "SELECT", "FROM", "WHERE", "AND", "OR", "INSERT", "UPDATE", "DELETE",
        "AUTOINCREMENT", "DEFAULT", "CHECK", "FOREIGN",
    ];

    let mut spans: Vec<Span> = Vec::new();
    let upper = line.to_uppercase();
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        let rest: String = upper[i..].to_string();
        let mut matched = false;
        for kw in &keywords {
            if rest.starts_with(kw) {
                let end = i + kw.len();
                let boundary = end >= chars.len() || !chars[end].is_alphanumeric();
                if boundary {
                    spans.push(Span::styled(
                        line[i..end].to_string(),
                        Style::default().fg(Color::Rgb(130, 190, 255)).add_modifier(Modifier::BOLD),
                    ));
                    i = end;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            // collect non-keyword chars
            let start = i;
            while i < chars.len() {
                let r: String = upper[i..].to_string();
                let kw_here = keywords.iter().any(|kw| {
                    if r.starts_with(kw) {
                        let end = i + kw.len();
                        end >= chars.len() || !chars[end].is_alphanumeric()
                    } else {
                        false
                    }
                });
                if kw_here { break; }
                i += chars[i].len_utf8();
            }
            spans.push(Span::styled(
                line[start..i].to_string(),
                Style::default().fg(C_TEXT),
            ));
        }
    }

    Line::from(spans)
}

fn draw_query_results(f: &mut Frame, app: &mut App, area: Rect) {
    // Reuse the table renderer for query results
    draw_table(f, app, area);
}

fn draw_query_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.mode == AppMode::Query;
    let border_style = if is_active {
        Style::default().fg(C_ACTIVE)
    } else {
        Style::default().fg(C_INACTIVE)
    };

    let hint = if is_active {
        " Ctrl+Enter to run · Esc to cancel "
    } else {
        " Press / or e to edit query "
    };

    let display = if app.query_input.is_empty() && !is_active {
        Span::styled("Enter SQL query...", Style::default().fg(C_DIM).add_modifier(Modifier::ITALIC))
    } else {
        Span::styled(app.query_input.clone(), Style::default().fg(C_TEXT))
    };

    let para = Paragraph::new(Line::from(vec![
        Span::styled(" ⌨ ", Style::default().fg(C_DIM)),
        display,
    ]))
    .block(
        Block::default()
            .title(Span::styled(hint, Style::default().fg(C_DIM)))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(C_PANEL_BG)),
    );

    f.render_widget(para, area);

    // Show cursor in query bar when active
    if is_active {
        let cursor_x = area.x + 4 + app.query_cursor as u16; // 4 for border + " ⌨ " (3 chars)
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
    }
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let msg_style = if app.status_message.starts_with("Error") {
        Style::default().fg(C_ERROR)
    } else {
        Style::default().fg(C_DIM)
    };

    let panel_hint = match app.active_panel {
        ActivePanel::Sidebar => " [Sidebar] ↑↓ navigate · Enter select · o open ",
        ActivePanel::Table   => " [Table] ↑↓ row · ←→ col · n/p page · r refresh ",
        ActivePanel::Schema  => " [Schema] ↑↓ scroll ",
        ActivePanel::Query   => " [Query] Ctrl+Enter run · Esc cancel ",
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.status_message),
            msg_style,
        ),
        Span::styled("│", Style::default().fg(C_INACTIVE)),
        Span::styled(panel_hint, Style::default().fg(Color::Rgb(80, 90, 120))),
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(14, 14, 22))),
        area,
    );
}

fn draw_open_dialog(f: &mut Frame, app: &App, area: Rect) {
    let popup_w = 60u16;
    let popup_h = 5u16;
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w.min(area.width), popup_h.min(area.height));

    f.render_widget(Clear, popup_area);

    let para = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(&app.open_input, Style::default().fg(C_TEXT)),
    ]))
    .block(
        Block::default()
            .title(Span::styled(
                " Open SQLite File (Enter to confirm, Esc to cancel) ",
                Style::default().fg(C_ACTIVE).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_ACTIVE))
            .style(Style::default().bg(Color::Rgb(20, 20, 36))),
    );

    f.render_widget(para, popup_area);

    // Cursor in dialog
    let cx = popup_area.x + 2 + app.open_input.len() as u16;
    let cy = popup_area.y + 1;
    f.set_cursor_position((cx.min(popup_area.x + popup_area.width - 2), cy));
}
