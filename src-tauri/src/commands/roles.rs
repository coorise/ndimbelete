//! Role commands.

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::models::{CreateRoleInput, Role, UpdateRoleInput};
use crate::state::AppState;

fn map_role(row: &rusqlite::Row<'_>) -> rusqlite::Result<Role> {
    Ok(Role {
        id: row.get(0)?,
        name: row.get(1)?,
        permissions_json: row.get(2)?,
        created_at: row.get(3)?,
    })
}

#[tauri::command]
pub fn list_roles(state: State<'_, AppState>) -> Result<Vec<Role>, String> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare("SELECT id, name, permissions_json, created_at FROM roles ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_role)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_role(state: State<'_, AppState>, input: CreateRoleInput) -> Result<Role, String> {
    let conn = state.db.lock();
    let id = Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let perms = serde_json::to_string(&input.permissions).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, input.name, perms, now],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, name, permissions_json, created_at FROM roles WHERE id = ?1",
        [&id],
        map_role,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_role(state: State<'_, AppState>, input: UpdateRoleInput) -> Result<Role, String> {
    let conn = state.db.lock();
    let perms = serde_json::to_string(&input.permissions).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE roles SET name=?1, permissions_json=?2 WHERE id=?3",
        params![input.name, perms, input.id],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, name, permissions_json, created_at FROM roles WHERE id = ?1",
        [&input.id],
        map_role,
    )
    .map_err(|e| e.to_string())
}
