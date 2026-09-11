//! Gzipped SQLite file snapshots for push/pull.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rusqlite::{backup::Backup, Connection};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::{database_path, open_database, run_migrations};

pub fn db_data_version(conn: &Connection) -> Result<i64> {
    let v: i64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
    Ok(v)
}

fn temp_snapshot_path() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("ndimbelente");
    fs::create_dir_all(&dir)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(dir.join(format!("snap-{nanos}.db")))
}

/// Checkpoint + online backup → gzip bytes.
pub fn make_snapshot_bytes(conn: &Connection) -> Result<Vec<u8>> {
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
    let tmp = temp_snapshot_path()?;
    {
        let mut dst = Connection::open(&tmp)
            .with_context(|| format!("Impossible de créer {}", tmp.display()))?;
        {
            let backup = Backup::new(conn, &mut dst)?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .context("Échec de la copie SQLite")?;
        }
    }
    let raw = fs::read(&tmp).context("Lecture snapshot échouée")?;
    let _ = fs::remove_file(&tmp);

    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw)?;
    Ok(enc.finish()?)
}

/// Replace the live DB file with a gzipped snapshot (caller holds AppState lock pattern).
pub fn restore_snapshot_bytes(gzipped: &[u8], live_conn: &mut Connection) -> Result<()> {
    let mut decoder = GzDecoder::new(gzipped);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw)?;

    let tmp = temp_snapshot_path()?;
    fs::write(&tmp, &raw)?;

    let db_path = database_path()?;
    live_conn.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
    let old = std::mem::replace(
        live_conn,
        Connection::open_in_memory().context("mémoire SQLite")?,
    );
    drop(old);

    remove_wal_files(&db_path);
    fs::copy(&tmp, &db_path).context("Remplacement de la base locale")?;
    remove_wal_files(&db_path);
    let _ = fs::remove_file(&tmp);

    let new_conn = open_database()?;
    run_migrations(&new_conn)?;
    *live_conn = new_conn;
    Ok(())
}

fn remove_wal_files(db: &Path) {
    let _ = fs::remove_file(Path::new(&format!("{}-wal", db.display())));
    let _ = fs::remove_file(Path::new(&format!("{}-shm", db.display())));
}
