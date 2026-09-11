//! Local activity log — management actions (staff, members, cotisations, …).

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

pub const AREA_STAFF: &str = "personnel";
pub const AREA_MEMBERS: &str = "membres";
pub const AREA_COTISATIONS: &str = "cotisations";
pub const AREA_PLANNING: &str = "planning";
pub const AREA_SETTINGS: &str = "parametres";

pub fn ensure_activity_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS activity_log (
            id TEXT PRIMARY KEY,
            staff_id TEXT,
            staff_name TEXT,
            area TEXT NOT NULL,
            action TEXT NOT NULL,
            summary TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_log(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_activity_area ON activity_log(area);",
    )?;
    Ok(())
}

pub fn log_activity(
    conn: &Connection,
    staff_id: Option<&str>,
    staff_name: Option<&str>,
    area: &str,
    action: &str,
    summary: &str,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO activity_log (id, staff_id, staff_name, area, action, summary, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, staff_id, staff_name, area, action, summary, now],
    )?;
    Ok(())
}

pub fn resolve_actor_name(conn: &Connection, staff_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT TRIM(first_name || ' ' || last_name) FROM staff WHERE id = ?1",
        [staff_id],
        |r| r.get(0),
    )
    .ok()
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: String,
    pub staff_id: Option<String>,
    pub staff_name: Option<String>,
    pub area: String,
    pub action: String,
    pub summary: String,
    pub created_at: String,
}

pub fn list_activities(conn: &Connection, limit: i64) -> Result<Vec<ActivityEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, staff_id, staff_name, area, action, summary, created_at
         FROM activity_log
         ORDER BY created_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok(ActivityEntry {
            id: r.get(0)?,
            staff_id: r.get(1)?,
            staff_name: r.get(2)?,
            area: r.get(3)?,
            action: r.get(4)?,
            summary: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Build a human default message for a data-version push from recent activity.
pub fn suggest_push_message(conn: &Connection) -> String {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM activity_log", [], |r| r.get(0))
        .unwrap_or(0);
    if count == 0 {
        return "Synchronisation des données de gestion".into();
    }
    let areas: String = conn
        .prepare(
            "SELECT DISTINCT area FROM activity_log
             ORDER BY created_at DESC LIMIT 5",
        )
        .ok()
        .and_then(|mut s| {
            let rows = s.query_map([], |r| r.get::<_, String>(0)).ok()?;
            let mut v = Vec::new();
            for row in rows.flatten() {
                v.push(row);
            }
            Some(v.join(", "))
        })
        .unwrap_or_default();
    if areas.is_empty() {
        format!("Synchronisation des données ({count} activité(s))")
    } else {
        format!("Synchronisation — {areas}")
    }
}
