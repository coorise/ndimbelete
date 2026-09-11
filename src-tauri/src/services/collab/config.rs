//! Local collaboration config + optional `.env` default URI.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_KEEP_COMMITS: u32 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabConfig {
    /// Remote PostgreSQL URI (any host that speaks Postgres).
    #[serde(default)]
    pub uri: Option<String>,
    /// Whether the user chose to stay connected.
    #[serde(default)]
    pub connected: bool,
    /// Last commit id applied locally (after push or pull).
    #[serde(default)]
    pub last_commit_id: Option<String>,
    /// SQLite `PRAGMA data_version` at last sync — dirty if current differs.
    #[serde(default)]
    pub last_synced_data_version: Option<i64>,
    /// ISO timestamp of last successful push/pull — used for local change badges.
    #[serde(default)]
    pub last_synced_at: Option<String>,
    /// Keep at most this many remote data versions (oldest deleted).
    #[serde(default = "default_keep")]
    pub keep_commits: u32,
    /// Role ids allowed to push (empty = any logged-in staff, unless staff list set).
    #[serde(default)]
    pub push_role_ids: Vec<String>,
    /// Staff ids allowed to push.
    #[serde(default)]
    pub push_staff_ids: Vec<String>,
}

fn default_keep() -> u32 {
    DEFAULT_KEEP_COMMITS
}

impl Default for CollabConfig {
    fn default() -> Self {
        Self {
            uri: None,
            connected: false,
            last_commit_id: None,
            last_synced_data_version: None,
            last_synced_at: None,
            keep_commits: DEFAULT_KEEP_COMMITS,
            push_role_ids: Vec::new(),
            push_staff_ids: Vec::new(),
        }
    }
}

pub fn collab_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    let dir = home.join("ndimbelente").join("data");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("collab.json"))
}

pub fn load_config() -> CollabConfig {
    let Ok(path) = collab_config_path() else {
        return CollabConfig::default();
    };
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => CollabConfig::default(),
    }
}

pub fn save_config(cfg: &CollabConfig) -> Result<()> {
    let path = collab_config_path()?;
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load optional `.env` from common locations and return `DATABASE_URL`
/// (or `NDIMBELENTE_DATABASE_URL`).
pub fn default_database_url() -> Option<String> {
    load_dotenv_files();
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NDIMBELENTE_DATABASE_URL"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn load_dotenv_files() {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
        candidates.push(cwd.join("..").join(".env"));
        candidates.push(cwd.join("..").join("..").join(".env"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(".env"));
            candidates.push(dir.join("..").join(".env"));
        }
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("ndimbelente").join(".env"));
    }

    for path in candidates {
        if path.is_file() {
            let _ = dotenvy::from_path(&path);
        }
    }
    // Also try default dotenv discovery (cwd).
    let _ = dotenvy::dotenv();
}
