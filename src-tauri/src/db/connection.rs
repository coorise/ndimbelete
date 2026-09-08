//! Open (or create) the SQLite database at `~/ndimbelente/data/ndimbelente.db`.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// Resolve `~/ndimbelente/data/ndimbelente.db` and ensure parent dirs exist.
pub fn database_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    let dir = home.join("ndimbelente").join("data");
    fs::create_dir_all(&dir).context("Failed to create ndimbelente data directory")?;
    Ok(dir.join("ndimbelente.db"))
}

/// Open the database with sensible pragmas for a desktop app.
pub fn open_database() -> Result<Connection> {
    let path = database_path()?;
    let conn = Connection::open(&path)
        .with_context(|| format!("Failed to open database at {}", path.display()))?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ",
    )?;

    Ok(conn)
}
