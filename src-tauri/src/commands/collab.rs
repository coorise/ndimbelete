//! Collaboration Tauri commands.

use tauri::State;

use crate::db::{list_activities, ActivityEntry};
use crate::models::SessionInfo;
use crate::services::auth_service;
use crate::services::collab::{self, CollabCommitInfo, CollabStatus, PushResult, RemoteProbe};
use crate::state::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollabConnectResponse {
    pub needs_remote_login: bool,
    pub status: CollabStatus,
    pub session: Option<SessionInfo>,
}

#[tauri::command]
pub fn collab_get_default_uri() -> Option<String> {
    collab::default_uri_for_ui()
}

#[tauri::command]
pub fn collab_probe(uri: String) -> Result<RemoteProbe, String> {
    collab::probe_remote(&uri).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_status(state: State<'_, AppState>) -> Result<CollabStatus, String> {
    let staff_id = state.session_staff_id.lock().clone();
    let conn = state.db.lock();
    collab::status(&conn, staff_id.as_deref()).map_err(|e| e.to_string())
}

/// Connect to remote. If remote already has data, pass `username` + `password` of a remote staff
/// account; local DB is replaced by the remote head and a session is opened.
#[tauri::command]
pub fn collab_connect(
    state: State<'_, AppState>,
    uri: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<CollabConnectResponse, String> {
    let mut conn = state.db.lock();
    let outcome = collab::connect_remote(
        &mut conn,
        &uri,
        username.as_deref(),
        password.as_deref(),
        true,
    )
    .map_err(|e| e.to_string())?;

    if outcome.needs_remote_login {
        return Ok(CollabConnectResponse {
            needs_remote_login: true,
            status: outcome.status,
            session: None,
        });
    }

    let had_credentials = username
        .as_ref()
        .map(|u| !u.trim().is_empty())
        .unwrap_or(false)
        && password.as_ref().map(|p| !p.is_empty()).unwrap_or(false);

    let session = if had_credentials {
        let user = username.as_ref().unwrap().trim();
        let pass = password.as_ref().unwrap();
        match login_on_conn(&conn, user, pass, &state) {
            Ok(s) => Some(s),
            Err(e) => {
                // Pull already applied; keep URI but do not mark connected.
                return Err(e);
            }
        }
    } else {
        None
    };

    if had_credentials || outcome.status.connected {
        let mut cfg = collab::load_config();
        cfg.uri = Some(uri.trim().to_string());
        cfg.connected = true;
        if cfg.last_synced_data_version.is_none() {
            cfg.last_synced_data_version = conn
                .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
                .ok();
            cfg.last_synced_at = Some(chrono::Utc::now().to_rfc3339());
        }
        collab::save_config(&cfg).map_err(|e| e.to_string())?;
        collab::start_listener(uri.trim().to_string());
    }

    let staff_id = state.session_staff_id.lock().clone();
    let status = collab::status(&conn, staff_id.as_deref()).map_err(|e| e.to_string())?;

    Ok(CollabConnectResponse {
        needs_remote_login: false,
        status,
        session,
    })
}

fn login_on_conn(
    conn: &rusqlite::Connection,
    username: &str,
    password: &str,
    state: &State<'_, AppState>,
) -> Result<SessionInfo, String> {
    let (id, hash, is_active): (String, String, i64) = conn
        .query_row(
            "SELECT id, password_hash, is_active FROM staff WHERE username = ?1",
            [username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Identifiant ou mot de passe incorrect (compte distant)".to_string())?;

    if is_active == 0 {
        return Err("Ce compte distant est désactivé".into());
    }
    if !auth_service::verify_password(password, &hash)? {
        return Err("Identifiant ou mot de passe incorrect (compte distant)".into());
    }

    let (staff, permissions) = crate::commands::load_staff_session(conn, &id)?;
    state.set_session(Some(id));
    Ok(SessionInfo { staff, permissions })
}

#[tauri::command]
pub fn collab_disconnect() -> Result<(), String> {
    collab::disconnect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_push(
    state: State<'_, AppState>,
    message: Option<String>,
) -> Result<PushResult, String> {
    let (author_name, author_staff_id) = {
        let staff_id = state.session_staff_id.lock().clone();
        let conn = state.db.lock();
        match staff_id {
            Some(id) => {
                let name: Option<String> = conn
                    .query_row(
                        "SELECT first_name || ' ' || last_name FROM staff WHERE id = ?1",
                        [&id],
                        |r| r.get(0),
                    )
                    .ok();
                (name, Some(id))
            }
            None => (None, None),
        }
    };
    let conn = state.db.lock();
    let msg = message.unwrap_or_default();
    collab::push_snapshot(&conn, &msg, author_name, author_staff_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_pull(state: State<'_, AppState>) -> Result<CollabStatus, String> {
    let mut conn = state.db.lock();
    let status = collab::pull_head(&mut conn).map_err(|e| e.to_string())?;
    state.set_session(None);
    Ok(status)
}

#[tauri::command]
pub fn collab_list_commits() -> Result<Vec<CollabCommitInfo>, String> {
    collab::list_commits().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_list_activities(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<ActivityEntry>, String> {
    let conn = state.db.lock();
    list_activities(&conn, limit.unwrap_or(200)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_rollback(
    state: State<'_, AppState>,
    commit_id: String,
) -> Result<CollabStatus, String> {
    let mut conn = state.db.lock();
    let status = collab::rollback_to_commit(&mut conn, &commit_id).map_err(|e| e.to_string())?;
    state.set_session(None);
    Ok(status)
}

#[tauri::command]
pub fn collab_cleanup(keep: Option<u32>) -> Result<u64, String> {
    collab::cleanup_old_commits(keep).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_set_keep_commits(keep: u32) -> Result<(), String> {
    let mut cfg = collab::load_config();
    cfg.keep_commits = keep.max(1);
    collab::save_config(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_set_push_acl(
    state: State<'_, AppState>,
    role_ids: Vec<String>,
    staff_ids: Vec<String>,
) -> Result<(), String> {
    let conn = state.db.lock();
    let mut staff_ids = staff_ids;
    // Root is always authorized to push.
    if let Ok(root) = conn.query_row(
        "SELECT id FROM staff ORDER BY created_at ASC, id ASC LIMIT 1",
        [],
        |r| r.get::<_, String>(0),
    ) {
        if !staff_ids.iter().any(|id| id == &root) {
            staff_ids.push(root);
        }
    }
    drop(conn);
    let mut cfg = collab::load_config();
    cfg.push_role_ids = role_ids;
    cfg.push_staff_ids = staff_ids;
    collab::save_config(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collab_clear_remote() -> Result<(), String> {
    collab::clear_remote_database().map_err(|e| e.to_string())
}
