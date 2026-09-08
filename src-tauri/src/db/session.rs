//! Persist the logged-in staff id to disk so rebuilds / restarts keep the session
//! until the user explicitly logs out.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::connection::database_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFile {
    staff_id: String,
}

fn session_path() -> Result<PathBuf> {
    let db = database_path()?;
    let dir = db
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(dir.join("session.json"))
}

/// Load the persisted staff id, if any.
pub fn load_session_staff_id() -> Option<String> {
    let path = session_path().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let parsed: SessionFile = serde_json::from_str(&raw).ok()?;
    let id = parsed.staff_id.trim().to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// Persist the current staff id (login / setup).
pub fn save_session_staff_id(staff_id: &str) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create session dir")?;
    }
    let payload = SessionFile {
        staff_id: staff_id.to_string(),
    };
    let raw = serde_json::to_string_pretty(&payload).context("serialize session")?;
    fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Clear the persisted session (logout).
pub fn clear_session_file() {
    if let Ok(path) = session_path() {
        let _ = fs::remove_file(path);
    }
}
