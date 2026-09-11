//! Member commands.

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::db::{
    get_member_by_id, map_member, resolve_payment_and_virement, MEMBER_SELECT,
};
use crate::models::{CreateMemberInput, Member, UpdateMemberInput};
use crate::state::AppState;

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[tauri::command]
pub fn list_members(state: State<'_, AppState>) -> Result<Vec<Member>, String> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MEMBER_SELECT}
             FROM members m
             LEFT JOIN member_roles mr ON mr.id = m.member_role_id
             ORDER BY m.last_name, m.first_name"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_member)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_member(state: State<'_, AppState>, id: String) -> Result<Member, String> {
    let conn = state.db.lock();
    get_member_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_member(
    state: State<'_, AppState>,
    input: CreateMemberInput,
) -> Result<Member, String> {
    if input.card_number.trim().is_empty() {
        return Err("N°CARTE requis".into());
    }
    let conn = state.db.lock();
    let id = Uuid::new_v4().to_string();
    let now = now_iso();

    let (role_id, payment_method, virement) = resolve_payment_and_virement(
        &conn,
        input.member_role_id.as_deref(),
        input.payment_method.as_deref(),
        input.bank_transfer_status.as_deref(),
    )?;

    conn.execute(
        "INSERT INTO members (
            id, card_number, last_name, first_name, adhesion_fee,
            address, address_complement, postal_code, city, phone, email,
            bank_transfer_status, status, notes, member_role_id, payment_method,
            created_at, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'active',?13,?14,?15,?16,?16)",
        params![
            id,
            input.card_number.trim(),
            input.last_name,
            input.first_name,
            input.adhesion_fee,
            input.address,
            input.address_complement,
            input.postal_code,
            input.city,
            input.phone,
            input.email,
            virement,
            input.notes,
            role_id,
            payment_method,
            now,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ce N°CARTE existe déjà".into()
        } else {
            e.to_string()
        }
    })?;

    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_MEMBERS,
        "create",
        format!(
            "Ajout du membre {} {} (carte {})",
            input.first_name, input.last_name, input.card_number.trim()
        ),
    );

    get_member_by_id(&conn, &id)
}

#[tauri::command]
pub fn update_member(
    state: State<'_, AppState>,
    input: UpdateMemberInput,
) -> Result<Member, String> {
    let conn = state.db.lock();
    let now = now_iso();

    let (role_id, payment_method, virement) = resolve_payment_and_virement(
        &conn,
        input.member_role_id.as_deref(),
        input.payment_method.as_deref(),
        input.bank_transfer_status.as_deref(),
    )?;

    let n = conn
        .execute(
            "UPDATE members SET
                card_number=?1, last_name=?2, first_name=?3, adhesion_fee=?4,
                address=?5, address_complement=?6, postal_code=?7, city=?8,
                phone=?9, email=?10, bank_transfer_status=?11, status=?12,
                notes=?13, member_role_id=?14, payment_method=?15, updated_at=?16
             WHERE id=?17",
            params![
                input.card_number.trim(),
                input.last_name,
                input.first_name,
                input.adhesion_fee,
                input.address,
                input.address_complement,
                input.postal_code,
                input.city,
                input.phone,
                input.email,
                virement,
                input.status,
                input.notes,
                role_id,
                payment_method,
                now,
                input.id,
            ],
        )
        .map_err(|e| e.to_string())?;

    if n == 0 {
        return Err("Membre introuvable".into());
    }

    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_MEMBERS,
        "update",
        format!(
            "Modification du membre {} {} (carte {})",
            input.first_name, input.last_name, input.card_number.trim()
        ),
    );

    get_member_by_id(&conn, &input.id)
}

#[tauri::command]
pub fn delete_member(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock();
    let label: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name || ' (' || card_number || ')' FROM members WHERE id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| id.clone());
    let n = conn
        .execute("DELETE FROM members WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Membre introuvable".into());
    }
    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_MEMBERS,
        "delete",
        format!("Suppression du membre {label}"),
    );
    Ok(())
}

#[tauri::command]
pub fn delete_members(state: State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let conn = state.db.lock();
    let mut deleted = 0usize;
    let count = ids.len();
    for id in ids {
        let n = conn
            .execute("DELETE FROM members WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
        deleted += n;
    }
    if deleted > 0 {
        crate::commands::note(
            &state,
            &conn,
            crate::db::AREA_MEMBERS,
            "delete",
            format!("Suppression de {deleted} membre(s) (sélection de {count})"),
        );
    }
    Ok(deleted)
}

#[tauri::command]
pub fn search_members(state: State<'_, AppState>, query: String) -> Result<Vec<Member>, String> {
    let conn = state.db.lock();
    let q = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MEMBER_SELECT}
             FROM members m
             LEFT JOIN member_roles mr ON mr.id = m.member_role_id
             WHERE m.last_name LIKE ?1 OR m.first_name LIKE ?1 OR m.card_number LIKE ?1
                OR m.phone LIKE ?1 OR m.email LIKE ?1 OR m.city LIKE ?1
             ORDER BY m.last_name, m.first_name
             LIMIT 100"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([&q], map_member)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
