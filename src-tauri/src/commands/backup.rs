//! Backup / restore / restart commands.

use std::fs;
use std::path::Path;

use rusqlite::Connection;
use tauri::{AppHandle, State};

use crate::db::{database_path, open_database, run_migrations};
use crate::state::AppState;

fn remove_wal_files(db: &Path) {
    let wal = Path::new(&format!("{}-wal", db.display())).to_path_buf();
    let shm = Path::new(&format!("{}-shm", db.display())).to_path_buf();
    let _ = fs::remove_file(&wal);
    let _ = fs::remove_file(&shm);
}

/// Copy the live SQLite database to `path` (after WAL checkpoint).
#[tauri::command]
pub fn export_backup(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let db_path = database_path().map_err(|e| e.to_string())?;
    {
        let conn = state.db.lock();
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|e| e.to_string())?;
    }
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(&db_path, &path).map_err(|e| format!("Échec de la sauvegarde : {e}"))?;
    Ok(())
}

/// Replace the app database with the file at `path`, then reopen.
/// Intended for first-run restore or full replace after closing the connection.
#[tauri::command]
pub fn import_backup(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let src = Path::new(&path);
    if !src.is_file() {
        return Err(format!("Fichier introuvable : {path}"));
    }

    let db_path = database_path().map_err(|e| e.to_string())?;

    {
        let mut guard = state.db.lock();
        let _ = guard.execute_batch("PRAGMA wal_checkpoint(FULL);");
        // Swap out the live connection so the file can be overwritten on Windows.
        let old = std::mem::replace(
            &mut *guard,
            Connection::open_in_memory().map_err(|e| e.to_string())?,
        );
        drop(old);

        remove_wal_files(&db_path);
        fs::copy(src, &db_path).map_err(|e| format!("Échec de l'import : {e}"))?;
        remove_wal_files(&db_path);

        let new_conn = open_database().map_err(|e| e.to_string())?;
        run_migrations(&new_conn).map_err(|e| e.to_string())?;
        *guard = new_conn;
    }

    // Clear any in-memory session after a restore.
    state.set_session(None);
    Ok(())
}

/// Restart the desktop application.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
