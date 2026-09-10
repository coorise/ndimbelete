//! Member role commands (association members — not staff).

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::models::{CreateMemberRoleInput, MemberRole, UpdateMemberRoleInput};
use crate::state::AppState;

fn map_role(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemberRole> {
    Ok(MemberRole {
        id: row.get(0)?,
        name: row.get(1)?,
        permissions_json: row.get(2)?,
        created_at: row.get(3)?,
    })
}

#[tauri::command]
pub fn list_member_roles(state: State<'_, AppState>) -> Result<Vec<MemberRole>, String> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, name, permissions_json, created_at FROM member_roles ORDER BY name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_role)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_member_role(
    state: State<'_, AppState>,
    input: CreateMemberRoleInput,
) -> Result<MemberRole, String> {
    let conn = state.db.lock();
    let id = Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let perms = serde_json::to_string(&input.permissions).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO member_roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, input.name.trim(), perms, now],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ce rôle membre existe déjà".into()
        } else {
            e.to_string()
        }
    })?;

    conn.query_row(
        "SELECT id, name, permissions_json, created_at FROM member_roles WHERE id = ?1",
        [&id],
        map_role,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_member_role(
    state: State<'_, AppState>,
    input: UpdateMemberRoleInput,
) -> Result<MemberRole, String> {
    let conn = state.db.lock();
    let perms = serde_json::to_string(&input.permissions).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE member_roles SET name=?1, permissions_json=?2 WHERE id=?3",
        params![input.name.trim(), perms, input.id],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, name, permissions_json, created_at FROM member_roles WHERE id = ?1",
        [&input.id],
        map_role,
    )
    .map_err(|e| e.to_string())
}
