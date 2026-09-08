//! Auth commands: login, logout, session, change password, first-run setup, recovery.

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::models::{
    ChangePasswordInput, LoginInput, RecoveryStatus, ResetPasswordWithHintsInput, SessionInfo,
    SetupAdminInput, Staff, UpdateMyProfileInput,
};
use crate::services::auth_service;
use crate::state::AppState;

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn hints_present(school: &Option<String>, color: &Option<String>) -> bool {
    school.as_ref().is_some_and(|s| !s.is_empty()) && color.as_ref().is_some_and(|c| !c.is_empty())
}

/// `true` when no staff account exists yet — show the setup screen.
#[tauri::command]
pub fn needs_setup(state: State<'_, AppState>) -> Result<bool, String> {
    let conn = state.db.lock();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM staff", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count == 0)
}

/// Create the primary administrator on first run, then open a session.
#[tauri::command]
pub fn setup_admin(
    state: State<'_, AppState>,
    input: SetupAdminInput,
) -> Result<SessionInfo, String> {
    let conn = state.db.lock();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM staff", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Err("L'administrateur a déjà été configuré".into());
    }

    let last_name = input.last_name.trim().to_string();
    let first_name = input.first_name.trim().to_string();
    let full_name = crate::models::compose_full_name(&last_name, &first_name);
    let username = input.username.trim().to_string();
    if last_name.is_empty() && first_name.is_empty() {
        return Err("Nom et prénom obligatoires".into());
    }
    if username.len() < 3 {
        return Err("L'identifiant doit faire au moins 3 caractères".into());
    }
    if input.password.len() < 8 {
        return Err("Le mot de passe doit faire au moins 8 caractères".into());
    }

    let role_id: String = conn
        .query_row(
            "SELECT id FROM roles WHERE name = 'Commissaire aux comptes' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Rôle Commissaire aux comptes introuvable".to_string())?;

    let password_hash = auth_service::hash_password(&input.password)?;
    let school_hash =
        auth_service::optional_recovery_hash(input.recovery_school.as_deref())?;
    let color_hash = auth_service::optional_recovery_hash(input.recovery_color.as_deref())?;
    let staff_id = Uuid::new_v4().to_string();
    let phone = input
        .phone
        .as_ref()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    conn.execute(
        "INSERT INTO staff (id, username, password_hash, full_name, first_name, last_name, phone, role_id, is_active, created_at, recovery_school_hash, recovery_color_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11)",
        params![
            staff_id,
            username,
            password_hash,
            full_name,
            first_name,
            last_name,
            phone,
            role_id,
            now_iso(),
            school_hash,
            color_hash,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Cet identifiant est déjà utilisé".into()
        } else {
            e.to_string()
        }
    })?;

    let (staff, permissions) = load_staff(&conn, &staff_id)?;
    drop(conn);
    state.set_session(Some(staff_id));

    Ok(SessionInfo {
        staff,
        permissions,
    })
}

fn load_staff(conn: &rusqlite::Connection, staff_id: &str) -> Result<(Staff, Vec<String>), String> {
    let founder: Option<String> = conn
        .query_row(
            "SELECT id FROM staff ORDER BY created_at ASC, id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.username, s.full_name, s.first_name, s.last_name, s.phone, s.role_id, r.name, s.is_active, s.created_at,
                    s.recovery_school_hash, s.recovery_color_hash, r.permissions_json
             FROM staff s JOIN roles r ON r.id = s.role_id
             WHERE s.id = ?1",
        )
        .map_err(|e| e.to_string())?;

    stmt.query_row([staff_id], |row| {
        let id: String = row.get(0)?;
        let full_name: String = row.get(2)?;
        let mut first_name: String = row.get(3)?;
        let mut last_name: String = row.get(4)?;
        if first_name.trim().is_empty() && last_name.trim().is_empty() && !full_name.trim().is_empty()
        {
            let (l, f) = crate::models::split_full_name(&full_name);
            last_name = l;
            first_name = f;
        }
        let school: Option<String> = row.get(10)?;
        let color: Option<String> = row.get(11)?;
        let perms_json: String = row.get(12)?;
        let permissions: Vec<String> =
            serde_json::from_str(&perms_json).unwrap_or_default();
        Ok((
            Staff {
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
            },
            permissions,
        ))
    })
    .map_err(|_| "Staff not found".to_string())
}

#[tauri::command]
pub fn login(state: State<'_, AppState>, input: LoginInput) -> Result<SessionInfo, String> {
    let conn = state.db.lock();

    let (id, hash, is_active): (String, String, i64) = conn
        .query_row(
            "SELECT id, password_hash, is_active FROM staff WHERE username = ?1",
            [&input.username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Identifiant ou mot de passe incorrect".to_string())?;

    if is_active == 0 {
        return Err("Ce compte est désactivé".into());
    }

    if !auth_service::verify_password(&input.password, &hash)? {
        return Err("Identifiant ou mot de passe incorrect".into());
    }

    let (staff, permissions) = load_staff(&conn, &id)?;
    drop(conn);
    state.set_session(Some(id));

    Ok(SessionInfo {
        staff,
        permissions,
    })
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> Result<(), String> {
    state.set_session(None);
    Ok(())
}

#[tauri::command]
pub fn get_current_session(state: State<'_, AppState>) -> Result<Option<SessionInfo>, String> {
    let session = state.session_staff_id.lock().clone();
    let Some(id) = session else {
        return Ok(None);
    };
    let conn = state.db.lock();
    let (staff, permissions) = load_staff(&conn, &id)?;
    Ok(Some(SessionInfo {
        staff,
        permissions,
    }))
}

#[tauri::command]
pub fn change_password(
    state: State<'_, AppState>,
    input: ChangePasswordInput,
) -> Result<(), String> {
    let staff_id = state
        .session_staff_id
        .lock()
        .clone()
        .ok_or_else(|| "Non connecté".to_string())?;

    let conn = state.db.lock();
    let hash: String = conn
        .query_row(
            "SELECT password_hash FROM staff WHERE id = ?1",
            [&staff_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    if !auth_service::verify_password(&input.current_password, &hash)? {
        return Err("Mot de passe actuel incorrect".into());
    }

    if input.new_password.len() < 8 {
        return Err("Le nouveau mot de passe doit faire au moins 8 caractères".into());
    }

    let new_hash = auth_service::hash_password(&input.new_password)?;
    conn.execute(
        "UPDATE staff SET password_hash = ?1 WHERE id = ?2",
        params![new_hash, staff_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Public: whether the given username has recovery hints configured.
#[tauri::command]
pub fn get_recovery_status(
    state: State<'_, AppState>,
    username: String,
) -> Result<RecoveryStatus, String> {
    let username = username.trim().to_string();
    let conn = state.db.lock();

    let row: Result<(Option<String>, Option<String>), _> = conn.query_row(
        "SELECT recovery_school_hash, recovery_color_hash FROM staff WHERE username = ?1",
        [&username],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );

    match row {
        Ok((school, color)) => Ok(RecoveryStatus {
            username,
            has_hints: hints_present(&school, &color),
        }),
        Err(_) => Ok(RecoveryStatus {
            username,
            // Do not reveal whether the account exists.
            has_hints: false,
        }),
    }
}

/// Public: reset password using school + favorite color recovery answers.
#[tauri::command]
pub fn reset_password_with_hints(
    state: State<'_, AppState>,
    input: ResetPasswordWithHintsInput,
) -> Result<(), String> {
    let username = input.username.trim().to_string();
    if input.new_password.len() < 8 {
        return Err("Le nouveau mot de passe doit faire au moins 8 caractères".into());
    }

    let conn = state.db.lock();
    let (id, school_hash, color_hash): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT id, recovery_school_hash, recovery_color_hash FROM staff WHERE username = ?1",
            [&username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Identifiant ou réponses incorrects".to_string())?;

    if !hints_present(&school_hash, &color_hash) {
        return Err("Aucune question de récupération configurée pour ce compte".into());
    }

    let school_ok = auth_service::verify_recovery_answer(
        &input.school_answer,
        school_hash.as_deref().unwrap_or(""),
    )?;
    let color_ok = auth_service::verify_recovery_answer(
        &input.color_answer,
        color_hash.as_deref().unwrap_or(""),
    )?;

    if !school_ok || !color_ok {
        return Err("Identifiant ou réponses incorrects".into());
    }

    let new_hash = auth_service::hash_password(&input.new_password)?;
    conn.execute(
        "UPDATE staff SET password_hash = ?1 WHERE id = ?2",
        params![new_hash, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Update the logged-in user's profile (name, phone, optional recovery hints).
#[tauri::command]
pub fn update_my_profile(
    state: State<'_, AppState>,
    input: UpdateMyProfileInput,
) -> Result<Staff, String> {
    let staff_id = state
        .session_staff_id
        .lock()
        .clone()
        .ok_or_else(|| "Non connecté".to_string())?;

    let last_name = input.last_name.trim().to_string();
    let first_name = input.first_name.trim().to_string();
    let full_name = crate::models::compose_full_name(&last_name, &first_name);
    if last_name.is_empty() && first_name.is_empty() {
        return Err("Nom et prénom obligatoires".into());
    }

    let phone = input
        .phone
        .as_ref()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    let conn = state.db.lock();
    conn.execute(
        "UPDATE staff SET full_name = ?1, first_name = ?2, last_name = ?3, phone = ?4 WHERE id = ?5",
        params![full_name, first_name, last_name, phone, staff_id],
    )
    .map_err(|e| e.to_string())?;

    if let Some(school) = input.recovery_school.as_ref() {
        if !school.trim().is_empty() {
            let hash = auth_service::hash_recovery_answer(school)?;
            conn.execute(
                "UPDATE staff SET recovery_school_hash = ?1 WHERE id = ?2",
                params![hash, staff_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    if let Some(color) = input.recovery_color.as_ref() {
        if !color.trim().is_empty() {
            let hash = auth_service::hash_recovery_answer(color)?;
            conn.execute(
                "UPDATE staff SET recovery_color_hash = ?1 WHERE id = ?2",
                params![hash, staff_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let (staff, _) = load_staff(&conn, &staff_id)?;
    Ok(staff)
}
