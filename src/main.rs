mod app;
mod db;
mod ui;
mod event;

use anyhow::Result;
use clap::Parser;
use app::App;

#[derive(Parser, Debug)]
#[command(name = "sqltui", about = "A TUI browser for SQLite databases")]
struct Args {
    /// Path to the SQLite database file
    #[arg(value_name = "DATABASE")]
    database: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new(args.database)?;
    app.run()
}
