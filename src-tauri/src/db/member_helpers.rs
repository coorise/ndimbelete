//! Shared SQL helpers for member rows (with role join).

use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{
    normalize_payment_method, reconstruct_virement_cell, Member, MEMBER_PERM_CAN_PAY,
    MEMBER_PERM_PAY_BANK, MEMBER_PERM_PAY_CASH, NORMAL_MEMBER_ROLE, PAYMENT_METHOD_BANK,
    PAYMENT_METHOD_CASH, PAYMENT_METHOD_NONE,
};

pub const MEMBER_SELECT: &str = "
m.id, m.card_number, m.last_name, m.first_name, m.adhesion_fee,
m.address, m.address_complement, m.postal_code, m.city, m.phone, m.email,
m.bank_transfer_status, m.status, m.notes, m.created_at, m.updated_at,
m.member_role_id, COALESCE(m.payment_method, 'cash'), mr.name
";

pub fn map_member(row: &rusqlite::Row<'_>) -> rusqlite::Result<Member> {
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
        member_role_id: row.get(16)?,
        payment_method: row.get(17)?,
        member_role_name: row.get(18)?,
    })
}

pub fn get_member_by_id(conn: &Connection, id: &str) -> Result<Member, String> {
    conn.query_row(
        &format!(
            "SELECT {MEMBER_SELECT}
             FROM members m
             LEFT JOIN member_roles mr ON mr.id = m.member_role_id
             WHERE m.id = ?1"
        ),
        [id],
        map_member,
    )
    .map_err(|_| "Membre introuvable".to_string())
}

pub fn ensure_normal_member_role(conn: &Connection) -> Result<String, String> {
    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM member_roles WHERE LOWER(name) = LOWER(?1)",
        [NORMAL_MEMBER_ROLE],
        |r| r.get(0),
    );
    match existing {
        Ok(id) => Ok(id),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            let perms = serde_json::to_string(&[
                MEMBER_PERM_CAN_PAY,
                MEMBER_PERM_PAY_CASH,
                MEMBER_PERM_PAY_BANK,
            ])
            .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO member_roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, NORMAL_MEMBER_ROLE, perms, now],
            )
            .map_err(|e| e.to_string())?;
            Ok(id)
        }
    }
}

pub fn ensure_member_role(
    conn: &Connection,
    name: &str,
    can_pay: bool,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return ensure_normal_member_role(conn);
    }
    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM member_roles WHERE LOWER(name) = LOWER(?1)",
        [name],
        |r| r.get(0),
    );
    match existing {
        Ok(id) => Ok(id),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            let perms = if can_pay {
                serde_json::to_string(&[
                    MEMBER_PERM_CAN_PAY,
                    MEMBER_PERM_PAY_CASH,
                    MEMBER_PERM_PAY_BANK,
                ])
            } else {
                serde_json::to_string(&Vec::<String>::new())
            }
            .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO member_roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, perms, now],
            )
            .map_err(|e| e.to_string())?;
            Ok(id)
        }
    }
}

pub fn role_has_can_pay(conn: &Connection, role_id: &str) -> Result<bool, String> {
    let json: String = conn
        .query_row(
            "SELECT permissions_json FROM member_roles WHERE id = ?1",
            [role_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "[]".into());
    Ok(json.contains(MEMBER_PERM_CAN_PAY) || json.contains("\"*\""))
}

/// Map a raw Excel VIREMENT BANQUAIRE cell using the user's role-value selection.
pub fn map_virement_cell(
    conn: &Connection,
    raw: &str,
    role_values: &[String],
) -> Result<(String, String, Option<String>), String> {
    let normal_id = ensure_normal_member_role(conn)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok((normal_id, PAYMENT_METHOD_CASH.into(), None));
    }

    let is_role = role_values
        .iter()
        .any(|v| v.trim().eq_ignore_ascii_case(trimmed));

    if is_role {
        let role_id = ensure_member_role(conn, trimmed, false)?;
        let cell = reconstruct_virement_cell(trimmed, PAYMENT_METHOD_NONE);
        let status = if cell.is_empty() { None } else { Some(cell) };
        return Ok((role_id, PAYMENT_METHOD_NONE.into(), status));
    }

    if trimmed.eq_ignore_ascii_case("VIREMENT") {
        let cell = reconstruct_virement_cell(NORMAL_MEMBER_ROLE, PAYMENT_METHOD_BANK);
        return Ok((
            normal_id,
            PAYMENT_METHOD_BANK.into(),
            if cell.is_empty() { None } else { Some(cell) },
        ));
    }

    // Unknown non-role value: keep as role without payment (Excel parity).
    let role_id = ensure_member_role(conn, trimmed, false)?;
    let cell = reconstruct_virement_cell(trimmed, PAYMENT_METHOD_NONE);
    Ok((
        role_id,
        PAYMENT_METHOD_NONE.into(),
        if cell.is_empty() { None } else { Some(cell) },
    ))
}

pub fn resolve_payment_and_virement(
    conn: &Connection,
    member_role_id: Option<&str>,
    payment_method: Option<&str>,
    bank_transfer_status: Option<&str>,
) -> Result<(Option<String>, String, Option<String>), String> {
    let role_id = match member_role_id {
        Some(id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => ensure_normal_member_role(conn)?,
    };
    let can_pay = role_has_can_pay(conn, &role_id)?;
    let role_name: String = conn
        .query_row(
            "SELECT name FROM member_roles WHERE id = ?1",
            [&role_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| NORMAL_MEMBER_ROLE.into());

    let mut pm = normalize_payment_method(payment_method.unwrap_or(PAYMENT_METHOD_CASH));
    if !can_pay {
        pm = PAYMENT_METHOD_NONE.into();
    } else if pm == PAYMENT_METHOD_NONE {
        pm = PAYMENT_METHOD_CASH.into();
    }

    // Prefer explicit bank_transfer_status only when reconstructing from legacy UI;
    // otherwise always rebuild from role + payment method.
    let cell = if let Some(raw) = bank_transfer_status.map(str::trim).filter(|s| !s.is_empty()) {
        // If caller still sends a legacy raw cell and no payment_method override,
        // keep it when it matches reconstruction; else reconstruct.
        let rebuilt = reconstruct_virement_cell(&role_name, &pm);
        if rebuilt.eq_ignore_ascii_case(raw) {
            Some(raw.to_string())
        } else if payment_method.is_none() {
            Some(raw.to_string())
        } else {
            if rebuilt.is_empty() {
                None
            } else {
                Some(rebuilt)
            }
        }
    } else {
        let rebuilt = reconstruct_virement_cell(&role_name, &pm);
        if rebuilt.is_empty() {
            None
        } else {
            Some(rebuilt)
        }
    };

    Ok((Some(role_id), pm, cell))
}
