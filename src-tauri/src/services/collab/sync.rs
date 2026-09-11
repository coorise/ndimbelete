//! Push / pull / rollback / cleanup against remote collab_* tables.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use uuid::Uuid;

use super::config::{load_config, save_config, CollabConfig};
use super::remote::{ensure_remote_schema, open_remote};
use super::snapshot::{db_data_version, make_snapshot_bytes, restore_snapshot_bytes};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollabCommitInfo {
    pub id: String,
    pub message: String,
    pub author_name: Option<String>,
    pub author_staff_id: Option<String>,
    pub created_at: String,
    pub parent_id: Option<String>,
    pub is_head: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollabStatus {
    pub connected: bool,
    pub uri_masked: Option<String>,
    pub default_uri_masked: Option<String>,
    pub has_default_uri: bool,
    pub last_commit_id: Option<String>,
    pub dirty: bool,
    pub keep_commits: u32,
    pub remote_commit_count: Option<i64>,
    pub remote_head_id: Option<String>,
    pub pending_remote_push: bool,
    pub can_push: bool,
    pub push_role_ids: Vec<String>,
    pub push_staff_ids: Vec<String>,
    /// Local root (first staff by created_at) — always allowed to push / manage ACL.
    pub root_staff_id: Option<String>,
    /// Local management changes since last sync (for Envoyer badge).
    pub local_change_count: u32,
    /// Remote commits ahead of local head (for Recevoir badge).
    pub remote_ahead_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub commit_id: String,
    pub cleaned: u64,
}

/// Result of connect: may ask for remote credentials before completing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollabConnectResult {
    /// Remote already has a data head — caller must supply username/password and retry.
    pub needs_remote_login: bool,
    pub status: CollabStatus,
}

pub fn mask_uri(uri: &str) -> String {
    // Hide password between ://user:PASSWORD@host
    if let Some(scheme_end) = uri.find("://") {
        let rest = &uri[scheme_end + 3..];
        if let Some(at) = rest.find('@') {
            let creds = &rest[..at];
            let host = &rest[at..];
            if let Some(colon) = creds.find(':') {
                let user = &creds[..colon];
                return format!("{}{}:***{}", &uri[..scheme_end + 3], user, host);
            }
        }
    }
    uri.to_string()
}

fn is_dirty(conn: &Connection, cfg: &CollabConfig) -> bool {
    match (cfg.last_synced_data_version, db_data_version(conn)) {
        (Some(saved), Ok(cur)) => cur != saved,
        _ => false,
    }
}

fn local_change_count(conn: &Connection, cfg: &CollabConfig, dirty: bool) -> u32 {
    if !dirty {
        return 0;
    }
    let count: i64 = if let Some(ts) = cfg.last_synced_at.as_deref() {
        conn.query_row(
            "SELECT COUNT(*) FROM activity_log WHERE created_at > ?1",
            [ts],
            |r| r.get(0),
        )
        .unwrap_or(0)
    } else {
        conn.query_row("SELECT COUNT(*) FROM activity_log", [], |r| r.get(0))
            .unwrap_or(0)
    };
    count.max(1) as u32
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn root_staff_id(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT id FROM staff ORDER BY created_at ASC, id ASC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .ok()
}

fn can_user_push(conn: &Connection, staff_id: Option<&str>, cfg: &CollabConfig) -> bool {
    let Some(sid) = staff_id else {
        return false;
    };
    // Root (first staff) can always push.
    if root_staff_id(conn).as_deref() == Some(sid) {
        return true;
    }
    if cfg.push_role_ids.is_empty() && cfg.push_staff_ids.is_empty() {
        return true;
    }
    if cfg.push_staff_ids.iter().any(|id| id == sid) {
        return true;
    }
    if cfg.push_role_ids.is_empty() {
        return false;
    }
    let role_id: Result<String, _> =
        conn.query_row("SELECT role_id FROM staff WHERE id = ?1", [sid], |r| r.get(0));
    matches!(role_id, Ok(rid) if cfg.push_role_ids.iter().any(|id| id == &rid))
}

pub fn status(conn: &Connection, staff_id: Option<&str>) -> Result<CollabStatus> {
    let cfg = load_config();
    let default_uri = super::config::default_database_url();
    let dirty = is_dirty(conn, &cfg);
    let can_push = can_user_push(conn, staff_id, &cfg);

    let mut remote_commit_count = None;
    let mut remote_head_id = None;
    let mut pending_remote_push = false;
    let mut remote_ahead_count = 0u32;

    if cfg.connected {
        if let Some(uri) = cfg.uri.as_deref() {
            if let Ok(mut client) = open_remote(uri) {
                let _ = ensure_remote_schema(&mut client);
                if let Ok(row) = client.query_one(
                    "SELECT commit_id FROM collab_head WHERE id = 1",
                    &[],
                ) {
                    let head: Option<String> = row.get(0);
                    remote_head_id = head.clone();
                    if let (Some(local), Some(remote)) = (&cfg.last_commit_id, &head) {
                        pending_remote_push = local != remote;
                    } else if head.is_some() && cfg.last_commit_id.is_none() {
                        pending_remote_push = true;
                    }
                }
                if let Ok(row) =
                    client.query_one("SELECT COUNT(*)::bigint FROM collab_commits", &[])
                {
                    remote_commit_count = Some(row.get(0));
                }
                if pending_remote_push {
                    remote_ahead_count = if let Some(local_id) = cfg.last_commit_id.as_deref() {
                        client
                            .query_one(
                                "SELECT COUNT(*)::bigint FROM collab_commits c
                                 WHERE c.created_at > COALESCE(
                                   (SELECT created_at FROM collab_commits WHERE id = $1),
                                   '-infinity'::timestamptz
                                 )",
                                &[&local_id],
                            )
                            .map(|r| r.get::<_, i64>(0))
                            .unwrap_or(1)
                            .max(1) as u32
                    } else {
                        remote_commit_count.unwrap_or(1).max(1) as u32
                    };
                }
            }
        }
    }

    let local_changes = local_change_count(conn, &cfg, dirty);

    Ok(CollabStatus {
        connected: cfg.connected,
        uri_masked: cfg.uri.as_deref().map(mask_uri),
        default_uri_masked: default_uri.as_deref().map(mask_uri),
        has_default_uri: default_uri.is_some(),
        last_commit_id: cfg.last_commit_id.clone(),
        dirty,
        keep_commits: cfg.keep_commits,
        remote_commit_count,
        remote_head_id,
        pending_remote_push,
        can_push,
        push_role_ids: cfg.push_role_ids.clone(),
        push_staff_ids: cfg.push_staff_ids.clone(),
        root_staff_id: root_staff_id(conn),
        local_change_count: local_changes,
        remote_ahead_count,
    })
}

pub fn disconnect() -> Result<()> {
    let mut cfg = load_config();
    cfg.connected = false;
    // Keep URI for next connect convenience.
    save_config(&cfg)?;
    crate::services::collab::listen::stop_listener();
    Ok(())
}

/// Save URI, mark connected, ensure schema. If remote has a head and local is empty staff, pull.
pub fn connect_and_pull_if_needed(
    conn: &mut Connection,
    uri: &str,
    pull_if_remote_has_data: bool,
) -> Result<CollabStatus> {
    let result = connect_remote(conn, uri, None, None, pull_if_remote_has_data)?;
    if result.needs_remote_login {
        return Err(anyhow!(
            "Le distant contient déjà des données — saisissez l'identifiant et le mot de passe du compte distant."
        ));
    }
    Ok(result.status)
}

/// Connect to remote Postgres.
///
/// - Empty remote: mark connected, keep local data (first push establishes root).
/// - Remote has head + no credentials: return `needs_remote_login` (not connected yet).
/// - Remote has head + credentials: pull head; caller must login against local DB after.
pub fn connect_remote(
    conn: &mut Connection,
    uri: &str,
    username: Option<&str>,
    password: Option<&str>,
    _pull_if_remote_has_data: bool,
) -> Result<CollabConnectResult> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Err(anyhow!("URI obligatoire"));
    }

    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;

    let head: Option<String> = client
        .query_opt("SELECT commit_id FROM collab_head WHERE id = 1", &[])?
        .and_then(|r| r.get(0));

    if let Some(commit_id) = head {
        let user = username.map(str::trim).filter(|s| !s.is_empty());
        let pass = password.filter(|s| !s.is_empty());
        match (user, pass) {
            (None, _) | (_, None) => {
                // Remember URI for convenience but do not mark connected until auth.
                let mut cfg = load_config();
                cfg.uri = Some(uri.to_string());
                cfg.connected = false;
                save_config(&cfg)?;
                let st = status(conn, None)?;
                return Ok(CollabConnectResult {
                    needs_remote_login: true,
                    status: st,
                });
            }
            (Some(_), Some(_)) => {
                pull_commit_id(conn, &mut client, &commit_id)?;
                let mut cfg = load_config();
                cfg.uri = Some(uri.to_string());
                // Connected only after successful remote login (command layer).
                cfg.connected = false;
                save_config(&cfg)?;
                return Ok(CollabConnectResult {
                    needs_remote_login: false,
                    status: status(conn, None)?,
                });
            }
        }
    }

    // Empty remote — first-come-first-served: local data (and its root) will become remote root on first push.
    let mut cfg = load_config();
    cfg.uri = Some(uri.to_string());
    cfg.connected = true;
    // Baseline so Envoyer does not blink until the user actually changes local data.
    cfg.last_synced_data_version = db_data_version(conn).ok();
    cfg.last_synced_at = Some(now_iso());
    save_config(&cfg)?;
    super::listen::start_listener(uri.to_string());
    Ok(CollabConnectResult {
        needs_remote_login: false,
        status: status(conn, None)?,
    })
}

fn pull_commit_id(
    conn: &mut Connection,
    client: &mut postgres::Client,
    commit_id: &str,
) -> Result<()> {
    let row = client
        .query_one(
            "SELECT snapshot FROM collab_commits WHERE id = $1",
            &[&commit_id],
        )
        .with_context(|| format!("Commit {commit_id} introuvable"))?;
    let bytes: Vec<u8> = row.get(0);
    restore_snapshot_bytes(&bytes, conn)?;

    let mut cfg = load_config();
    cfg.last_commit_id = Some(commit_id.to_string());
    cfg.last_synced_data_version = Some(db_data_version(conn)?);
    cfg.last_synced_at = Some(now_iso());
    save_config(&cfg)?;
    Ok(())
}

pub fn pull_head(conn: &mut Connection) -> Result<CollabStatus> {
    let cfg = load_config();
    if !cfg.connected {
        return Err(anyhow!("Non connecté au distant"));
    }
    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;

    let head: Option<String> = client
        .query_opt("SELECT commit_id FROM collab_head WHERE id = 1", &[])?
        .and_then(|r| r.get(0));
    let Some(commit_id) = head else {
        return Err(anyhow!("Aucune version distante — envoyez d'abord (Push)."));
    };

    pull_commit_id(conn, &mut client, &commit_id)?;
    status(conn, None)
}

pub fn push_snapshot(
    conn: &Connection,
    message: &str,
    author_name: Option<String>,
    author_staff_id: Option<String>,
) -> Result<PushResult> {
    let cfg = load_config();
    if !cfg.connected {
        return Err(anyhow!("Non connecté au distant"));
    }
    if !can_user_push(conn, author_staff_id.as_deref(), &cfg) {
        return Err(anyhow!(
            "Vous n'êtes pas autorisé à envoyer les données (rôle / utilisateur)."
        ));
    }

    let message = {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            crate::db::suggest_push_message(conn)
        } else {
            trimmed.to_string()
        }
    };

    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let keep = cfg.keep_commits.max(1);

    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;

    // First-come-first-served: never overwrite the remote root staff record.
    preserve_remote_root_staff(conn, &mut client)?;

    let snapshot = make_snapshot_bytes(conn)?;
    let commit_id = Uuid::new_v4().to_string();
    let parent_id = cfg.last_commit_id.clone();

    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO collab_commits (id, message, author_name, author_staff_id, parent_id, snapshot)
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &commit_id,
            &message,
            &author_name,
            &author_staff_id,
            &parent_id,
            &snapshot,
        ],
    )?;
    tx.execute(
        "INSERT INTO collab_head (id, commit_id) VALUES (1, $1)
         ON CONFLICT (id) DO UPDATE SET commit_id = EXCLUDED.commit_id",
        &[&commit_id],
    )?;
    tx.commit()?;

    // Best-effort notify (optional UX).
    let _ = client.execute("NOTIFY ndimbelente_push, $1", &[&commit_id]);

    let cleaned = cleanup_with_client(&mut client, keep)?;

    let mut cfg = load_config();
    cfg.last_commit_id = Some(commit_id.clone());
    cfg.last_synced_data_version = Some(db_data_version(conn)?);
    cfg.last_synced_at = Some(now_iso());
    save_config(&cfg)?;

    Ok(PushResult {
        commit_id,
        cleaned,
    })
}

/// If remote already has a head, copy its root staff (+ role) into the local DB so a later
/// local admin setup cannot overwrite the first-come remote root on push.
fn preserve_remote_root_staff(conn: &Connection, client: &mut postgres::Client) -> Result<()> {
    let head: Option<String> = client
        .query_opt("SELECT commit_id FROM collab_head WHERE id = 1", &[])?
        .and_then(|r| r.get(0));
    let Some(commit_id) = head else {
        return Ok(());
    };

    let row = client.query_one(
        "SELECT snapshot FROM collab_commits WHERE id = $1",
        &[&commit_id],
    )?;
    let bytes: Vec<u8> = row.get(0);

    let remote_db = snapshot_to_temp_connection(&bytes)?;
    let Some(root) = load_root_staff_row(&remote_db)? else {
        return Ok(());
    };

    // Ensure the remote root's role exists locally (by id); copy from remote snapshot if needed.
    let role_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM roles WHERE id = ?1",
            [&root.role_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if role_exists == 0 {
        if let Ok((name, perms, created)) = remote_db.query_row(
            "SELECT name, permissions_json, created_at FROM roles WHERE id = ?1",
            [&root.role_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
        ) {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![root.role_id, name, perms, created],
            );
        }
    }

    // Username unique: if another local staff uses the remote root username, rename them.
    let conflict: Option<String> = conn
        .query_row(
            "SELECT id FROM staff WHERE username = ?1 AND id != ?2",
            rusqlite::params![root.username, root.id],
            |r| r.get(0),
        )
        .ok();
    if let Some(other_id) = conflict {
        let renamed = format!("{}_local", root.username);
        let _ = conn.execute(
            "UPDATE staff SET username = ?1 WHERE id = ?2",
            rusqlite::params![renamed, other_id],
        );
    }

    conn.execute(
        "INSERT INTO staff (
            id, username, password_hash, full_name, first_name, last_name, phone,
            role_id, is_active, created_at, recovery_school_hash, recovery_color_hash
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            username=excluded.username,
            password_hash=excluded.password_hash,
            full_name=excluded.full_name,
            first_name=excluded.first_name,
            last_name=excluded.last_name,
            phone=excluded.phone,
            role_id=excluded.role_id,
            is_active=excluded.is_active,
            created_at=excluded.created_at,
            recovery_school_hash=excluded.recovery_school_hash,
            recovery_color_hash=excluded.recovery_color_hash",
        rusqlite::params![
            root.id,
            root.username,
            root.password_hash,
            root.full_name,
            root.first_name,
            root.last_name,
            root.phone,
            root.role_id,
            root.is_active,
            root.created_at,
            root.recovery_school_hash,
            root.recovery_color_hash,
        ],
    )?;

    // Keep remote root as earliest staff (founder).
    let _ = conn.execute(
        "UPDATE staff SET created_at = created_at || '.1' WHERE id != ?1 AND created_at <= ?2",
        rusqlite::params![root.id, root.created_at],
    );

    Ok(())
}

struct RootStaffRow {
    id: String,
    username: String,
    password_hash: String,
    full_name: String,
    first_name: String,
    last_name: String,
    phone: Option<String>,
    role_id: String,
    is_active: i64,
    created_at: String,
    recovery_school_hash: Option<String>,
    recovery_color_hash: Option<String>,
}

fn load_root_staff_row(conn: &Connection) -> Result<Option<RootStaffRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, full_name, first_name, last_name, phone,
                role_id, is_active, created_at, recovery_school_hash, recovery_color_hash
         FROM staff
         ORDER BY created_at ASC, id ASC
         LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(RootStaffRow {
        id: row.get(0)?,
        username: row.get(1)?,
        password_hash: row.get(2)?,
        full_name: row.get(3)?,
        first_name: row.get(4)?,
        last_name: row.get(5)?,
        phone: row.get(6)?,
        role_id: row.get(7)?,
        is_active: row.get(8)?,
        created_at: row.get(9)?,
        recovery_school_hash: row.get(10)?,
        recovery_color_hash: row.get(11)?,
    }))
}

fn snapshot_to_temp_connection(gzipped: &[u8]) -> Result<Connection> {
    use flate2::read::GzDecoder;
    use rusqlite::backup::Backup;
    use std::io::Read;

    let mut decoder = GzDecoder::new(gzipped);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw)?;

    let dir = std::env::temp_dir().join("ndimbelente");
    std::fs::create_dir_all(&dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("remote-root-{nanos}.db"));
    std::fs::write(&path, &raw)?;

    let file_conn = Connection::open(&path)?;
    let mut mem = Connection::open_in_memory()?;
    {
        let backup = Backup::new(&file_conn, &mut mem)?;
        backup
            .run_to_completion(100, std::time::Duration::from_millis(1), None)
            .context("Lecture snapshot distant échouée")?;
    }
    drop(file_conn);
    let _ = std::fs::remove_file(&path);
    Ok(mem)
}

pub fn list_commits() -> Result<Vec<CollabCommitInfo>> {
    let cfg = load_config();
    if !cfg.connected {
        return Ok(vec![]);
    }
    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;

    let head: Option<String> = client
        .query_opt("SELECT commit_id FROM collab_head WHERE id = 1", &[])?
        .and_then(|r| r.get(0));

    let rows = client.query(
        "SELECT id, message, author_name, author_staff_id, created_at, parent_id
         FROM collab_commits
         ORDER BY created_at DESC
         LIMIT 200",
        &[],
    )?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let id: String = row.get(0);
            let created: DateTime<Utc> = row.get(4);
            CollabCommitInfo {
                is_head: head.as_ref() == Some(&id),
                id,
                message: row.get(1),
                author_name: row.get(2),
                author_staff_id: row.get(3),
                created_at: created.to_rfc3339(),
                parent_id: row.get(5),
            }
        })
        .collect())
}

/// Restore a historical commit onto local DB (does not change remote head unless followed by push).
pub fn rollback_to_commit(conn: &mut Connection, commit_id: &str) -> Result<CollabStatus> {
    let cfg = load_config();
    if !cfg.connected {
        return Err(anyhow!("Non connecté au distant"));
    }
    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let mut client = open_remote(uri)?;
    pull_commit_id(conn, &mut client, commit_id)?;
    // Mark dirty so user is encouraged to push the rollback as a new head if desired.
    let mut cfg = load_config();
    cfg.last_synced_data_version = None;
    save_config(&cfg)?;
    status(conn, None)
}

pub fn cleanup_old_commits(keep: Option<u32>) -> Result<u64> {
    let mut cfg = load_config();
    if let Some(k) = keep {
        cfg.keep_commits = k.max(1);
        save_config(&cfg)?;
    }
    if !cfg.connected {
        return Err(anyhow!("Non connecté au distant"));
    }
    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;
    cleanup_with_client(&mut client, cfg.keep_commits.max(1))
}

fn cleanup_with_client(client: &mut postgres::Client, keep: u32) -> Result<u64> {
    let keep = keep.max(1) as i64;
    // Never delete the current head.
    let deleted = client.execute(
        r#"
        DELETE FROM collab_commits
        WHERE id NOT IN (
            SELECT commit_id FROM collab_head WHERE id = 1 AND commit_id IS NOT NULL
        )
        AND id IN (
            SELECT id FROM collab_commits
            ORDER BY created_at DESC
            OFFSET $1
        )
        "#,
        &[&keep],
    )?;
    Ok(deleted)
}

/// Admin: wipe all remote collaboration data.
pub fn clear_remote_database() -> Result<()> {
    let cfg = load_config();
    if !cfg.connected {
        return Err(anyhow!("Non connecté au distant"));
    }
    let uri = cfg.uri.as_deref().ok_or_else(|| anyhow!("URI manquante"))?;
    let mut client = open_remote(uri)?;
    client.batch_execute(
        "
        UPDATE collab_head SET commit_id = NULL WHERE id = 1;
        DELETE FROM collab_commits;
        ",
    )?;
    let mut cfg = load_config();
    cfg.last_commit_id = None;
    cfg.last_synced_data_version = None;
    save_config(&cfg)?;
    Ok(())
}

pub fn default_uri_for_ui() -> Option<String> {
    let cfg = load_config();
    cfg.uri
        .filter(|s| !s.is_empty())
        .or_else(super::config::default_database_url)
}
