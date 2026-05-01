use anyhow::{Context, Result};
use rusqlite::{Connection, types::Value};

pub struct Database {
    pub conn: Connection,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub row_count: i64,
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    pub col_type: String,
    pub notnull: bool,
    pub pk: bool,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub error: Option<String>,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path))?;
        Ok(Database {
            conn,
            path: path.to_string(),
        })
    }

    pub fn get_tables(&self) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut tables = Vec::new();
        for name in names {
            let count: i64 = self
                .conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{}\"", name), [], |r| r.get(0))
                .unwrap_or(0);
            tables.push(TableInfo { name, row_count: count });
        }
        Ok(tables)
    }

    pub fn get_views(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name"
        )?;
        let views = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(views)
    }

    pub fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let mut stmt = self.conn.prepare(
            &format!("PRAGMA table_info(\"{}\")", table)
        )?;
        let cols = stmt.query_map([], |row| {
            Ok(ColumnInfo {
                cid: row.get(0)?,
                name: row.get(1)?,
                col_type: row.get(2).unwrap_or_default(),
                notnull: row.get::<_, i64>(3).unwrap_or(0) != 0,
                pk: row.get::<_, i64>(5).unwrap_or(0) != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(cols)
    }

    pub fn get_table_data(&self, table: &str, limit: usize, offset: usize) -> QueryResult {
        self.execute_query(&format!(
            "SELECT * FROM \"{}\" LIMIT {} OFFSET {}",
            table, limit, offset
        ))
    }

    pub fn execute_query(&self, sql: &str) -> QueryResult {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return QueryResult {
                columns: vec![],
                rows: vec![],
                error: None,
            };
        }

        match self.conn.prepare(trimmed) {
            Err(e) => QueryResult {
                columns: vec![],
                rows: vec![],
                error: Some(e.to_string()),
            },
            Ok(mut stmt) => {
                let col_names: Vec<String> = stmt
                    .column_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();

                let rows_result: Result<Vec<Vec<String>>, _> = stmt
                    .query_map([], |row| {
                        let count = row.as_ref().column_count();
                        let mut cells = Vec::new();
                        for i in 0..count {
                            let val = match row.get::<_, Value>(i) {
                                Ok(Value::Null)    => "NULL".to_string(),
                                Ok(Value::Integer(n)) => n.to_string(),
                                Ok(Value::Real(f))    => format!("{:.6}", f),
                                Ok(Value::Text(s))    => s,
                                Ok(Value::Blob(b))    => format!("<BLOB {} bytes>", b.len()),
                                Err(_) => "?".to_string(),
                            };
                            cells.push(val);
                        }
                        Ok(cells)
                    })
                    .map(|iter| iter.filter_map(|r| r.ok()).collect());

                match rows_result {
                    Ok(rows) => QueryResult { columns: col_names, rows, error: None },
                    Err(e)   => QueryResult { columns: vec![], rows: vec![], error: Some(e.to_string()) },
                }
            }
        }
    }

    pub fn get_schema(&self, table: &str) -> Result<String> {
        let sql: Option<String> = self.conn.query_row(
            "SELECT sql FROM sqlite_master WHERE (type='table' OR type='view') AND name=?",
            [table],
            |row| row.get(0),
        ).ok();
        Ok(sql.unwrap_or_else(|| "No schema found".to_string()))
    }

    pub fn get_indexes(&self, table: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=? AND sql IS NOT NULL"
        )?;
        let indexes = stmt
            .query_map([table], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(indexes)
    }
}
