use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use crate::db::{Database, QueryResult, TableInfo};
use crate::event::handle_event;
use crate::ui::draw;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Query,
    Confirm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    Sidebar,
    Table,
    Schema,
    Query,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Data,
    Schema,
    Query,
}

pub struct App {
    pub db: Option<Database>,
    pub db_path: Option<String>,

    // sidebar
    pub tables: Vec<TableInfo>,
    pub views: Vec<String>,
    pub sidebar_index: usize,
    pub sidebar_scroll: usize,

    // active state
    pub active_panel: ActivePanel,
    pub active_tab: Tab,
    pub mode: AppMode,

    // table browser
    pub current_table: Option<String>,
    pub query_result: Option<QueryResult>,
    pub table_row_offset: usize,
    pub table_scroll_x: usize,
    pub table_selected_row: usize,
    pub page_size: usize,

    // schema view
    pub schema_text: String,
    pub schema_scroll: usize,

    // query editor
    pub query_input: String,
    pub query_cursor: usize,
    pub query_history: Vec<String>,
    pub query_history_index: Option<usize>,
    pub query_error: Option<String>,

    // status
    pub status_message: String,
    pub should_quit: bool,

    // open file prompt
    pub open_prompt: bool,
    pub open_input: String,
}

impl App {
    pub fn new(path: Option<String>) -> Result<Self> {
        let mut app = App {
            db: None,
            db_path: None,
            tables: vec![],
            views: vec![],
            sidebar_index: 0,
            sidebar_scroll: 0,
            active_panel: ActivePanel::Sidebar,
            active_tab: Tab::Data,
            mode: AppMode::Normal,
            current_table: None,
            query_result: None,
            table_row_offset: 0,
            table_scroll_x: 0,
            table_selected_row: 0,
            page_size: 100,
            schema_text: String::new(),
            schema_scroll: 0,
            query_input: String::new(),
            query_cursor: 0,
            query_history: vec![],
            query_history_index: None,
            query_error: None,
            status_message: String::from("Press ? for help"),
            should_quit: false,
            open_prompt: false,
            open_input: String::new(),
        };

        if let Some(p) = path {
            app.open_database(&p)?;
        } else {
            app.status_message = "No database loaded. Press 'o' to open a file.".to_string();
        }

        Ok(app)
    }

    pub fn open_database(&mut self, path: &str) -> Result<()> {
        let db = Database::open(path)?;
        self.tables = db.get_tables().unwrap_or_default();
        self.views = db.get_views().unwrap_or_default();
        self.db_path = Some(path.to_string());
        self.db = Some(db);
        self.sidebar_index = 0;
        self.current_table = None;
        self.query_result = None;
        self.status_message = format!("Opened: {}", path);

        if !self.tables.is_empty() {
            self.load_current_selection();
        }
        Ok(())
    }

    pub fn load_current_selection(&mut self) {
        let total = self.tables.len() + self.views.len();
        if total == 0 { return; }
        let idx = self.sidebar_index;
        let name = if idx < self.tables.len() {
            self.tables[idx].name.clone()
        } else {
            let vi = idx - self.tables.len();
            self.views[vi].clone()
        };
        self.load_table(&name);
    }

    pub fn load_table(&mut self, name: &str) {
        let Some(db) = &self.db else { return };
        self.current_table = Some(name.to_string());
        self.table_row_offset = 0;
        self.table_selected_row = 0;
        self.table_scroll_x = 0;
        self.query_result = Some(db.get_table_data(name, self.page_size, 0));
        self.schema_text = db.get_schema(name).unwrap_or_default();
        if let Ok(indexes) = db.get_indexes(name) {
            if !indexes.is_empty() {
                self.schema_text.push_str("\n\n-- Indexes:\n");
                for idx in indexes {
                    self.schema_text.push_str(&idx);
                    self.schema_text.push('\n');
                }
            }
        }
        self.schema_scroll = 0;
    }

    pub fn load_next_page(&mut self) {
        let Some(table) = self.current_table.clone() else { return };
        let Some(db) = &self.db else { return };
        let current_rows = self.query_result.as_ref().map(|r| r.rows.len()).unwrap_or(0);
        if current_rows < self.page_size { return; } // no more pages
        self.table_row_offset += self.page_size;
        self.query_result = Some(db.get_table_data(&table, self.page_size, self.table_row_offset));
        self.table_selected_row = 0;
        self.status_message = format!("Page offset: {}", self.table_row_offset);
    }

    pub fn load_prev_page(&mut self) {
        let Some(table) = self.current_table.clone() else { return };
        let Some(db) = &self.db else { return };
        if self.table_row_offset == 0 { return; }
        self.table_row_offset = self.table_row_offset.saturating_sub(self.page_size);
        self.query_result = Some(db.get_table_data(&table, self.page_size, self.table_row_offset));
        self.table_selected_row = 0;
        self.status_message = format!("Page offset: {}", self.table_row_offset);
    }

    pub fn execute_custom_query(&mut self) {
        let Some(db) = &self.db else { return };
        let sql = self.query_input.trim().to_string();
        if sql.is_empty() { return; }

        let result = db.execute_query(&sql);
        self.query_error = result.error.clone();
        if self.query_error.is_none() {
            let row_count = result.rows.len();
            self.status_message = format!("Query returned {} row(s)", row_count);
            self.query_history.push(sql.clone());
            self.query_history_index = None;
        } else {
            self.status_message = "Query error — see error message below".to_string();
        }
        self.query_result = Some(result);
        self.current_table = None;
        self.table_selected_row = 0;
        self.table_scroll_x = 0;
        self.active_tab = Tab::Data;
        self.active_panel = ActivePanel::Table;
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| draw(f, self))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    handle_event(self, key);
                }
            }

            if self.should_quit { break; }
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn sidebar_total(&self) -> usize {
        self.tables.len() + self.views.len()
    }
}
