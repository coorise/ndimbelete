//! Staff CRUD commands.

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::models::{
    compose_full_name, split_full_name, CreateStaffInput, Staff, UpdateStaffInput,
};
use crate::services::auth_service;
use crate::state::AppState;

fn hints_present(school: &Option<String>, color: &Option<String>) -> bool {
    school.as_ref().is_some_and(|s| !s.is_empty()) && color.as_ref().is_some_and(|c| !c.is_empty())
}

fn founder_id(conn: &rusqlite::Connection) -> Option<String> {
    conn.query_row(
        "SELECT id FROM staff ORDER BY created_at ASC, id ASC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .ok()
}

fn map_staff(row: &rusqlite::Row<'_>, founder: &Option<String>) -> rusqlite::Result<Staff> {
    let id: String = row.get(0)?;
    let full_name: String = row.get(2)?;
    let mut first_name: String = row.get(3)?;
    let mut last_name: String = row.get(4)?;
    if first_name.trim().is_empty() && last_name.trim().is_empty() && !full_name.trim().is_empty() {
        let (l, f) = split_full_name(&full_name);
        last_name = l;
        first_name = f;
    }
    let school: Option<String> = row.get(10)?;
    let color: Option<String> = row.get(11)?;
    Ok(Staff {
        id: id.clone(),
        username: row.get(1)?,
        full_name,
        first_name,
        last_name,
        phone: row.get(5)?,
        role_id: row.get(6)?,
        role_name: row.get(7)?,
        is_active: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        has_recovery_hints: hints_present(&school, &color),
        is_founder: founder.as_ref() == Some(&id),
    })
}

const STAFF_SELECT: &str = "
SELECT s.id, s.username, s.full_name, s.first_name, s.last_name, s.phone, s.role_id, r.name,
       s.is_active, s.created_at, s.recovery_school_hash, s.recovery_color_hash
FROM staff s JOIN roles r ON r.id = s.role_id
";

#[tauri::command]
pub fn list_staff(state: State<'_, AppState>) -> Result<Vec<Staff>, String> {
    let conn = state.db.lock();
    let founder = founder_id(&conn);
    let mut stmt = conn
        .prepare(&format!("{STAFF_SELECT} ORDER BY s.last_name, s.first_name, s.full_name"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| map_staff(row, &founder))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_staff(state: State<'_, AppState>, input: CreateStaffInput) -> Result<Staff, String> {
    if input.username.trim().is_empty() || input.password.len() < 8 {
        return Err("Username requis et mot de passe ≥ 8 caractères".into());
    }
    let last_name = input.last_name.trim().to_string();
    let first_name = input.first_name.trim().to_string();
    if last_name.is_empty() && first_name.is_empty() {
        return Err("Nom et prénom requis".into());
    }
    let full_name = compose_full_name(&last_name, &first_name);

    let conn = state.db.lock();
    let id = Uuid::new_v4().to_string();
    let hash = auth_service::hash_password(&input.password)?;
    let school_hash =
        auth_service::optional_recovery_hash(input.recovery_school.as_deref())?;
    let color_hash = auth_service::optional_recovery_hash(input.recovery_color.as_deref())?;
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO staff (id, username, password_hash, full_name, first_name, last_name, phone, role_id, is_active, created_at, recovery_school_hash, recovery_color_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11)",
        params![
            id,
            input.username.trim(),
            hash,
            full_name,
            first_name,
            last_name,
            input.phone,
            input.role_id,
            now,
            school_hash,
            color_hash,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ce nom d'utilisateur existe déjà".into()
        } else {
            e.to_string()
        }
    })?;

    let founder = founder_id(&conn);
    conn.query_row(
        &format!("{STAFF_SELECT} WHERE s.id = ?1"),
        [&id],
        |row| map_staff(row, &founder),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_staff(state: State<'_, AppState>, input: UpdateStaffInput) -> Result<Staff, String> {
    let last_name = input.last_name.trim().to_string();
    let first_name = input.first_name.trim().to_string();
    if last_name.is_empty() && first_name.is_empty() {
        return Err("Nom et prénom requis".into());
    }
    let full_name = compose_full_name(&last_name, &first_name);
    let conn = state.db.lock();

    conn.execute(
        "UPDATE staff SET username=?1, full_name=?2, first_name=?3, last_name=?4, phone=?5, role_id=?6 WHERE id=?7",
        params![
            input.username.trim(),
            full_name,
            first_name,
            last_name,
            input.phone,
            input.role_id,
            input.id
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(pw) = input.password {
        if !pw.is_empty() {
            if pw.len() < 8 {
                return Err("Mot de passe ≥ 8 caractères".into());
            }
            let hash = auth_service::hash_password(&pw)?;
            conn.execute(
                "UPDATE staff SET password_hash=?1 WHERE id=?2",
                params![hash, input.id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let founder = founder_id(&conn);
    conn.query_row(
        &format!("{STAFF_SELECT} WHERE s.id = ?1"),
        [&input.id],
        |row| map_staff(row, &founder),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deactivate_staff(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock();
    if let Some(founder) = founder_id(&conn) {
        if founder == id {
            return Err("Le premier compte enregistré ne peut pas être désactivé".into());
        }
    }
    let n = conn
        .execute("UPDATE staff SET is_active = 0 WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Staff introuvable".into());
    }
    Ok(())
}
