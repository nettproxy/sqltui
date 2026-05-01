# sqltui

A terminal UI for browsing SQLite databases. Navigate tables, inspect schemas, and run queries without leaving the keyboard.

Built with Rust using [ratatui](https://github.com/ratatui-org/ratatui) and [rusqlite](https://github.com/rusqlite/rusqlite).

---

## Install

**Arch Linux (AUR)**

```bash
yay -S sqltui
```

**From source**

```bash
git clone https://github.com/yourusername/sqltui
cd sqltui
cargo build --release
sudo install -Dm755 target/release/sqltui /usr/local/bin/sqltui
```

Requires Rust 1.70+ and a C compiler for the bundled SQLite.

---

## Usage

```bash
sqltui path/to/database.db
```

You can also launch without a file and open one from inside the app with `o`.

---

## Interface

The layout has three areas: a sidebar listing tables and views, a main panel showing data, schema, or query results, and a query bar at the bottom.

Switch between tabs with `d` (data), `s` (schema), and `/` (query). Switch panels with `Tab` or jump directly with `1`, `2`, `3`.

---

## Keybindings

**Global**

| Key | Action |
|-----|--------|
| `q` | Quit |
| `o` | Open a database file |
| `Tab` | Cycle panels forward |
| `Shift+Tab` | Cycle panels backward |
| `1` / `2` / `3` | Jump to sidebar / table / query |
| `?` | Print keybind hint to status bar |

**Sidebar**

| Key | Action |
|-----|--------|
| `j` `↓` | Move down |
| `k` `↑` | Move up |
| `Enter` | Load selected table |

**Data view**

| Key | Action |
|-----|--------|
| `j` `↓` | Next row |
| `k` `↑` | Previous row |
| `h` `←` | Scroll columns left |
| `l` `→` | Scroll columns right |
| `n` | Next page (100 rows) |
| `p` | Previous page |
| `r` | Refresh current table |

**Query editor**

| Key | Action |
|-----|--------|
| `/` or `e` | Enter query mode |
| `Ctrl+Enter` or `F5` | Execute query |
| `↑` `↓` | Navigate query history |
| `Esc` | Exit query mode |

**Tab switching**

| Key | Action |
|-----|--------|
| `d` | Data tab |
| `s` | Schema tab |
| `/` | Query tab |

---

## Notes

- Pagination is 100 rows per page. Large tables are never loaded fully into memory.
- Query history persists for the session and is navigable with arrow keys while in query mode.
- The schema view includes index definitions below the table DDL.
- Views are listed separately in the sidebar below tables.
- `rusqlite` is compiled with the `bundled` feature, so no system SQLite installation is required at build time.

---

## License

MIT
