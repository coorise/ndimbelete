//! Application state shared across Tauri commands.
//!
//! Holds the SQLite connection behind a `Mutex` so commands can
//! safely borrow it one at a time (offline desktop app — simple is fine).

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::db::{load_session_staff_id, save_session_staff_id};

/// Global app state managed by Tauri.
pub struct AppState {
    pub db: Mutex<Connection>,
    /// Currently logged-in staff id (None = logged out).
    pub session_staff_id: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        // Restore last session from disk so rebuild / restart stays logged in.
        let restored = load_session_staff_id().and_then(|id| {
            let active: Result<i64, _> = conn.query_row(
                "SELECT is_active FROM staff WHERE id = ?1",
                [&id],
                |r| r.get(0),
            );
            match active {
                Ok(1) => Some(id),
                _ => {
                    crate::db::clear_session_file();
                    None
                }
            }
        });

        Self {
            db: Mutex::new(conn),
            session_staff_id: Mutex::new(restored),
        }
    }

    /// Set in-memory session and persist to disk.
    pub fn set_session(&self, staff_id: Option<String>) {
        match &staff_id {
            Some(id) => {
                let _ = save_session_staff_id(id);
            }
            None => crate::db::clear_session_file(),
        }
        *self.session_staff_id.lock() = staff_id;
    }
}
