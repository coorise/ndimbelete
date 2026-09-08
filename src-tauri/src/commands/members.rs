//! Member commands.

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::models::{CreateMemberInput, Member, UpdateMemberInput};
use crate::state::AppState;

fn map_member(row: &rusqlite::Row<'_>) -> rusqlite::Result<Member> {
    Ok(Member {
        id: row.get(0)?,
        card_number: row.get(1)?,
        last_name: row.get(2)?,
        first_name: row.get(3)?,
        adhesion_fee: row.get(4)?,
        address: row.get(5)?,
        address_complement: row.get(6)?,
        postal_code: row.get(7)?,
        city: row.get(8)?,
        phone: row.get(9)?,
        email: row.get(10)?,
        bank_transfer_status: row.get(11)?,
        status: row.get(12)?,
        notes: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

const MEMBER_COLS: &str = "
id, card_number, last_name, first_name, adhesion_fee,
address, address_complement, postal_code, city, phone, email,
bank_transfer_status, status, notes, created_at, updated_at
";

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[tauri::command]
pub fn list_members(state: State<'_, AppState>) -> Result<Vec<Member>, String> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MEMBER_COLS} FROM members ORDER BY last_name, first_name"
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
    conn.query_row(
        &format!("SELECT {MEMBER_COLS} FROM members WHERE id = ?1"),
        [&id],
        map_member,
    )
    .map_err(|_| "Membre introuvable".to_string())
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

    conn.execute(
        "INSERT INTO members (
            id, card_number, last_name, first_name, adhesion_fee,
            address, address_complement, postal_code, city, phone, email,
            bank_transfer_status, status, notes, created_at, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'active',?13,?14,?14)",
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
            input.bank_transfer_status,
            input.notes,
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

    conn.query_row(
        &format!("SELECT {MEMBER_COLS} FROM members WHERE id = ?1"),
        [&id],
        map_member,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_member(
    state: State<'_, AppState>,
    input: UpdateMemberInput,
) -> Result<Member, String> {
    let conn = state.db.lock();
    let now = now_iso();

    let n = conn
        .execute(
            "UPDATE members SET
                card_number=?1, last_name=?2, first_name=?3, adhesion_fee=?4,
                address=?5, address_complement=?6, postal_code=?7, city=?8,
                phone=?9, email=?10, bank_transfer_status=?11, status=?12,
                notes=?13, updated_at=?14
             WHERE id=?15",
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
                input.bank_transfer_status,
                input.status,
                input.notes,
                now,
                input.id,
            ],
        )
        .map_err(|e| e.to_string())?;

    if n == 0 {
        return Err("Membre introuvable".into());
    }

    conn.query_row(
        &format!("SELECT {MEMBER_COLS} FROM members WHERE id = ?1"),
        [&input.id],
        map_member,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_member(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock();
    let n = conn
        .execute("DELETE FROM members WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Membre introuvable".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_members(state: State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let conn = state.db.lock();
    let mut deleted = 0usize;
    for id in ids {
        let n = conn
            .execute("DELETE FROM members WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
        deleted += n;
    }
    Ok(deleted)
}

#[tauri::command]
pub fn search_members(state: State<'_, AppState>, query: String) -> Result<Vec<Member>, String> {
    let conn = state.db.lock();
    let q = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MEMBER_COLS} FROM members
             WHERE last_name LIKE ?1 OR first_name LIKE ?1 OR card_number LIKE ?1
                OR phone LIKE ?1 OR email LIKE ?1 OR city LIKE ?1
             ORDER BY last_name, first_name
             LIMIT 100"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([&q], map_member)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
