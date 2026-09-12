//! Schema migrations and first-run seed data.
//!
//! Roles / settings / year 2026 are seeded automatically.
//! The primary administrator is **not** seeded — the user creates it
//! via the first-run setup screen (`setup_admin` command).

use anyhow::Result;
use rusqlite::Connection;
use uuid::Uuid;

use crate::models::{period_label, PERIOD_MONTHS};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    permissions_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS staff (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    first_name TEXT NOT NULL DEFAULT '',
    last_name TEXT NOT NULL DEFAULT '',
    phone TEXT,
    role_id TEXT NOT NULL REFERENCES roles(id),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    recovery_school_hash TEXT,
    recovery_color_hash TEXT
);

CREATE TABLE IF NOT EXISTS member_roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    permissions_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS members (
    id TEXT PRIMARY KEY,
    card_number TEXT NOT NULL UNIQUE,
    last_name TEXT NOT NULL,
    first_name TEXT NOT NULL,
    adhesion_fee REAL NOT NULL DEFAULT 0,
    address TEXT,
    address_complement TEXT,
    postal_code TEXT,
    city TEXT,
    phone TEXT,
    email TEXT,
    bank_transfer_status TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    notes TEXT,
    member_role_id TEXT REFERENCES member_roles(id),
    payment_method TEXT NOT NULL DEFAULT 'cash',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS contribution_years (
    id TEXT PRIMARY KEY,
    year INTEGER NOT NULL UNIQUE,
    monthly_amount REAL NOT NULL DEFAULT 10,
    sheet_label TEXT
);

CREATE TABLE IF NOT EXISTS contribution_periods (
    id TEXT PRIMARY KEY,
    year_id TEXT NOT NULL REFERENCES contribution_years(id) ON DELETE CASCADE,
    period_month INTEGER NOT NULL,
    label TEXT NOT NULL,
    meeting_date TEXT,
    collect_start TEXT DEFAULT '14:30',
    collect_end TEXT DEFAULT '15:30',
    sort_order INTEGER DEFAULT 0,
    label_color TEXT,
    UNIQUE(year_id, period_month)
);

CREATE TABLE IF NOT EXISTS member_period_entries (
    id TEXT PRIMARY KEY,
    member_id TEXT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    period_id TEXT NOT NULL REFERENCES contribution_periods(id) ON DELETE CASCADE,
    amount_due REAL NOT NULL,
    amount_paid REAL,
    note TEXT,
    UNIQUE(member_id, period_id)
);

CREATE TABLE IF NOT EXISTS member_year_meta (
    id TEXT PRIMARY KEY,
    member_id TEXT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    year_id TEXT NOT NULL REFERENCES contribution_years(id) ON DELETE CASCADE,
    prior_december_debt REAL NOT NULL DEFAULT 0,
    ristourne REAL NOT NULL DEFAULT 0,
    total_paid REAL NOT NULL DEFAULT 0,
    december_debt REAL NOT NULL DEFAULT 0,
    UNIQUE(member_id, year_id)
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Run `ALTER TABLE … ADD COLUMN`, ignoring SQLite "duplicate column" errors.
fn add_column_if_missing(conn: &Connection, sql: &str) -> Result<()> {
    match conn.execute_batch(sql) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("duplicate column") {
                Ok(())
            } else {
                Err(e.into())
            }
        }
    }
}

fn backfill_staff_names(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, first_name, last_name FROM staff
         WHERE TRIM(COALESCE(first_name,'')) = '' AND TRIM(COALESCE(last_name,'')) = ''",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for (id, full) in rows {
        let (last, first) = crate::models::split_full_name(&full);
        conn.execute(
            "UPDATE staff SET last_name=?1, first_name=?2 WHERE id=?3",
            rusqlite::params![last, first, id],
        )?;
    }
    Ok(())
}

/// Official AG meeting dates for year 2026 (association card).
fn meeting_date_for(year: i32, month: i32) -> String {
    if year == 2026 {
        match month {
            1 => return "2026-01-11".into(),
            3 => return "2026-03-08".into(),
            5 => return "2026-05-10".into(),
            7 => return "2026-07-12".into(),
            9 => return "2026-09-13".into(),
            11 => return "2026-11-08".into(),
            _ => {}
        }
    }
    // Fallback: first day of the month
    format!("{year}-{month:02}-01")
}

/// Run schema + seed if this is a fresh database.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)?;

    // Additive migrations for existing databases.
    add_column_if_missing(
        conn,
        "ALTER TABLE staff ADD COLUMN recovery_school_hash TEXT;",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE staff ADD COLUMN recovery_color_hash TEXT;",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE contribution_periods ADD COLUMN meeting_date TEXT;",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE contribution_periods ADD COLUMN collect_start TEXT DEFAULT '14:30';",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE contribution_periods ADD COLUMN collect_end TEXT DEFAULT '15:30';",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE contribution_periods ADD COLUMN sort_order INTEGER DEFAULT 0;",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE contribution_periods ADD COLUMN label_color TEXT;",
    )?;
    add_column_if_missing(conn, "ALTER TABLE staff ADD COLUMN first_name TEXT NOT NULL DEFAULT '';")?;
    add_column_if_missing(conn, "ALTER TABLE staff ADD COLUMN last_name TEXT NOT NULL DEFAULT '';")?;
    backfill_staff_names(conn)?;

    // Member roles + payment method (Excel VIREMENT BANQUAIRE split).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS member_roles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            permissions_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE members ADD COLUMN member_role_id TEXT REFERENCES member_roles(id);",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE members ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'cash';",
    )?;
    seed_member_roles(conn)?;
    backfill_member_roles_from_virement(conn)?;

    crate::db::ensure_activity_table(conn)?;

    // Backfill meeting dates / collect times / sort_order for existing periods.
    backfill_planning_defaults(conn)?;

    let role_count: i64 = conn.query_row("SELECT COUNT(*) FROM roles", [], |r| r.get(0))?;
    if role_count == 0 {
        seed_roles(conn)?;
        seed_settings(conn)?;
        seed_year(conn, 2026)?;
    }

    Ok(())
}

fn seed_member_roles(conn: &Connection) -> Result<()> {
    use crate::models::{
        MEMBER_PERM_CAN_PAY, MEMBER_PERM_PAY_BANK, MEMBER_PERM_PAY_CASH, NORMAL_MEMBER_ROLE,
    };

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM member_roles", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let now = now_iso();
    let normal_perms = serde_json::to_string(&[
        MEMBER_PERM_CAN_PAY,
        MEMBER_PERM_PAY_CASH,
        MEMBER_PERM_PAY_BANK,
    ])?;
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO member_roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, NORMAL_MEMBER_ROLE, normal_perms, now],
    )?;
    Ok(())
}

fn ensure_member_role_named(conn: &Connection, name: &str, permissions_json: &str) -> Result<String> {
    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM member_roles WHERE LOWER(name) = LOWER(?1)",
        [name],
        |r| r.get(0),
    );
    match existing {
        Ok(id) => Ok(id),
        Err(_) => {
            let id = Uuid::new_v4().to_string();
            let now = now_iso();
            conn.execute(
                "INSERT INTO member_roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, name, permissions_json, now],
            )?;
            Ok(id)
        }
    }
}

fn backfill_member_roles_from_virement(conn: &Connection) -> Result<()> {
    use crate::models::{
        reconstruct_virement_cell, DEFAULT_VIREMENT_ROLE_VALUES, NORMAL_MEMBER_ROLE,
        PAYMENT_METHOD_BANK, PAYMENT_METHOD_CASH, PAYMENT_METHOD_NONE,
    };

    let normal_id = ensure_member_role_named(
        conn,
        NORMAL_MEMBER_ROLE,
        &serde_json::to_string(&[
            crate::models::MEMBER_PERM_CAN_PAY,
            crate::models::MEMBER_PERM_PAY_CASH,
            crate::models::MEMBER_PERM_PAY_BANK,
        ])?,
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, bank_transfer_status, member_role_id, payment_method FROM members",
    )?;
    let rows: Vec<(String, Option<String>, Option<String>, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get::<_, String>(3).unwrap_or_else(|_| PAYMENT_METHOD_CASH.into()),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for (id, status, role_id, _pm) in rows {
        if role_id.is_some() {
            continue;
        }
        let raw = status.as_deref().unwrap_or("").trim();
        let (role_id, payment_method, role_name) = if raw.is_empty() {
            (normal_id.clone(), PAYMENT_METHOD_CASH, NORMAL_MEMBER_ROLE)
        } else if raw.eq_ignore_ascii_case("VIREMENT") {
            (normal_id.clone(), PAYMENT_METHOD_BANK, NORMAL_MEMBER_ROLE)
        } else if DEFAULT_VIREMENT_ROLE_VALUES
            .iter()
            .any(|v| v.eq_ignore_ascii_case(raw))
        {
            let rid = ensure_member_role_named(conn, raw, "[]")?;
            (rid, PAYMENT_METHOD_NONE, raw)
        } else {
            // Unknown flag → treat as role without payment obligation.
            let rid = ensure_member_role_named(conn, raw, "[]")?;
            (rid, PAYMENT_METHOD_NONE, raw)
        };
        let excel_cell = reconstruct_virement_cell(role_name, payment_method);
        let excel_opt = if excel_cell.is_empty() {
            None
        } else {
            Some(excel_cell)
        };
        conn.execute(
            "UPDATE members SET member_role_id=?1, payment_method=?2, bank_transfer_status=?3 WHERE id=?4",
            rusqlite::params![role_id, payment_method, excel_opt, id],
        )?;
    }
    Ok(())
}

fn backfill_planning_defaults(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT p.id, y.year, p.period_month, p.meeting_date, p.collect_start, p.collect_end, p.sort_order
         FROM contribution_periods p
         JOIN contribution_years y ON y.id = p.year_id",
    )?;
    let rows: Vec<(String, i32, i32, Option<String>, Option<String>, Option<String>, Option<i32>)> =
        stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

    for (id, year, month, meeting, start, end, _sort) in rows {
        let meeting = meeting
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| meeting_date_for(year, month));
        let start = start.filter(|s| !s.is_empty()).unwrap_or_else(|| "14:30".into());
        let end = end.filter(|s| !s.is_empty()).unwrap_or_else(|| "15:30".into());
        // Always align sort_order with calendar month.
        let sort = month;
        conn.execute(
            "UPDATE contribution_periods
             SET meeting_date=?1, collect_start=?2, collect_end=?3, sort_order=?4
             WHERE id=?5",
            rusqlite::params![meeting, start, end, sort, id],
        )?;
    }
    Ok(())
}

fn seed_roles(conn: &Connection) -> Result<()> {
    let now = now_iso();

    let roles: &[(&str, &str)] = &[
        ("Commissaire aux comptes", r#"["*"]"#),
        (
            "Trésorier",
            r#"["members:read","members:write","cotisations:read","cotisations:write","analytics:read","excel:import","excel:export","settings:read"]"#,
        ),
        (
            "Président",
            r#"["members:read","members:write","cotisations:read","analytics:read","settings:read","staff:read"]"#,
        ),
        (
            "Adjoint",
            r#"["members:read","members:write","cotisations:read","cotisations:write","analytics:read"]"#,
        ),
        (
            "Staff",
            r#"["members:read","cotisations:read","analytics:read"]"#,
        ),
    ];

    for (name, perms) in roles {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO roles (id, name, permissions_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, perms, now],
        )?;
    }

    Ok(())
}

fn seed_settings(conn: &Connection) -> Result<()> {
    let defaults = crate::models::AppSettings::default();
    let pairs = [
        ("org_name", defaults.org_name.as_str()),
        ("org_description", defaults.org_description.as_str()),
        ("org_address", defaults.org_address.as_str()),
        ("theme_color", defaults.theme_color.as_str()),
        ("font_scale", "1.0"),
        ("logo_path", ""),
        ("currency_unit", defaults.currency_unit.as_str()),
    ];
    for (k, v) in pairs {
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )?;
    }
    Ok(())
}

/// Ensure a contribution year exists with its 6 bi-monthly periods.
pub fn seed_year(conn: &Connection, year: i32) -> Result<String> {
    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM contribution_years WHERE year = ?1",
        [year],
        |r| r.get(0),
    );
    if let Ok(id) = existing {
        return Ok(id);
    }

    let year_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO contribution_years (id, year, monthly_amount, sheet_label) VALUES (?1, ?2, 20, ?3)",
        rusqlite::params![year_id, year, year.to_string()],
    )?;

    for (idx, &month) in PERIOD_MONTHS.iter().enumerate() {
        let period_id = Uuid::new_v4().to_string();
        let meeting = meeting_date_for(year, month);
        conn.execute(
            "INSERT INTO contribution_periods
             (id, year_id, period_month, label, meeting_date, collect_start, collect_end, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, '14:30', '15:30', ?6)",
            rusqlite::params![
                period_id,
                year_id,
                month,
                period_label(month),
                meeting,
                idx as i32
            ],
        )?;
    }

    Ok(year_id)
}
