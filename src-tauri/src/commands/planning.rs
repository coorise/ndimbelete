//! Planning — contribution period calendar (AG meeting dates & collect windows).

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::db::seed_year;
use crate::models::period_label;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningPeriod {
    pub id: String,
    pub year: i32,
    pub period_month: i32,
    pub label: String,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: i32,
    #[serde(default)]
    pub label_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPlanningInput {
    pub id: Option<String>,
    pub year: i32,
    pub period_month: i32,
    pub label: Option<String>,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: Option<i32>,
    pub label_color: Option<String>,
}

#[tauri::command]
pub fn list_planning(state: State<'_, AppState>, year: i32) -> Result<Vec<PlanningPeriod>, String> {
    let conn = state.db.lock();
    let year_id = seed_year(&conn, year).map_err(|e| e.to_string())?;

    // Repair drifted sort_order (DEFAULT 0 made edited months sort after untouched ones).
    let _ = conn.execute(
        "UPDATE contribution_periods
         SET sort_order = period_month
         WHERE year_id = ?1
           AND (sort_order IS NULL OR sort_order = 0 OR sort_order != period_month)",
        [&year_id],
    );

    let mut stmt = conn
        .prepare(
            "SELECT p.id, y.year, p.period_month, p.label,
                    p.meeting_date, p.collect_start, p.collect_end,
                    p.period_month, p.label_color
             FROM contribution_periods p
             JOIN contribution_years y ON y.id = p.year_id
             WHERE p.year_id = ?1
             ORDER BY p.period_month ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&year_id], |r| {
            Ok(PlanningPeriod {
                id: r.get(0)?,
                year: r.get(1)?,
                period_month: r.get(2)?,
                label: r.get(3)?,
                meeting_date: r.get(4)?,
                collect_start: r.get(5)?,
                collect_end: r.get(6)?,
                sort_order: r.get(7)?,
                label_color: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_planning_period(
    state: State<'_, AppState>,
    input: UpsertPlanningInput,
) -> Result<PlanningPeriod, String> {
    if !(1..=12).contains(&input.period_month) {
        return Err("Le mois doit être entre 1 et 12".into());
    }

    let conn = state.db.lock();
    let year_id = seed_year(&conn, input.year).map_err(|e| e.to_string())?;
    let label = input
        .label
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| period_label(input.period_month).to_string());
    let meeting = input.meeting_date.filter(|s| !s.trim().is_empty());
    let start = input
        .collect_start
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "14:30".into());
    let end = input
        .collect_end
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "15:30".into());
    // Calendar order is authoritative; keep sort_order aligned with period_month.
    let sort = input.period_month;
    let color = input
        .label_color
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let id = if let Some(ref existing) = input.id {
        conn.execute(
            "UPDATE contribution_periods
             SET period_month=?1, label=?2, meeting_date=?3,
                 collect_start=?4, collect_end=?5, sort_order=?6, label_color=?7
             WHERE id=?8 AND year_id=?9",
            rusqlite::params![
                input.period_month,
                label,
                meeting,
                start,
                end,
                sort,
                color,
                existing,
                year_id
            ],
        )
        .map_err(|e| e.to_string())?;
        existing.clone()
    } else {
        // Upsert by year+month if already exists
        let existing: Result<String, _> = conn.query_row(
            "SELECT id FROM contribution_periods WHERE year_id=?1 AND period_month=?2",
            rusqlite::params![year_id, input.period_month],
            |r| r.get(0),
        );
        match existing {
            Ok(eid) => {
                conn.execute(
                    "UPDATE contribution_periods
                     SET label=?1, meeting_date=?2, collect_start=?3, collect_end=?4, sort_order=?5, label_color=?6
                     WHERE id=?7",
                    rusqlite::params![label, meeting, start, end, sort, color, eid],
                )
                .map_err(|e| e.to_string())?;
                eid
            }
            Err(_) => {
                let nid = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO contribution_periods
                     (id, year_id, period_month, label, meeting_date, collect_start, collect_end, sort_order, label_color)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    rusqlite::params![
                        nid,
                        year_id,
                        input.period_month,
                        label,
                        meeting,
                        start,
                        end,
                        sort,
                        color
                    ],
                )
                .map_err(|e| e.to_string())?;
                nid
            }
        }
    };

    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_PLANNING,
        "upsert",
        format!(
            "Planning {label} (mois {}, année {})",
            input.period_month, input.year
        ),
    );

    conn.query_row(
        "SELECT p.id, y.year, p.period_month, p.label,
                p.meeting_date, p.collect_start, p.collect_end,
                COALESCE(p.sort_order, p.period_month), p.label_color
         FROM contribution_periods p
         JOIN contribution_years y ON y.id = p.year_id
         WHERE p.id = ?1",
        [&id],
        |r| {
            Ok(PlanningPeriod {
                id: r.get(0)?,
                year: r.get(1)?,
                period_month: r.get(2)?,
                label: r.get(3)?,
                meeting_date: r.get(4)?,
                collect_start: r.get(5)?,
                collect_end: r.get(6)?,
                sort_order: r.get(7)?,
                label_color: r.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_planning_period(
    state: State<'_, AppState>,
    id: String,
    force: Option<bool>,
) -> Result<(), String> {
    let conn = state.db.lock();

    let paid_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM member_period_entries
             WHERE period_id = ?1 AND amount_paid IS NOT NULL",
            [&id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if paid_count > 0 && !force.unwrap_or(false) {
        return Err(format!(
            "Cette période a {paid_count} paiement(s). Confirmez la suppression (force)."
        ));
    }

    // Cascade clear entries then delete period
    conn.execute(
        "DELETE FROM member_period_entries WHERE period_id = ?1",
        [&id],
    )
    .map_err(|e| e.to_string())?;
    let n = conn
        .execute("DELETE FROM contribution_periods WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Période introuvable".into());
    }
    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_PLANNING,
        "delete",
        "Suppression d'une période de planning",
    );
    Ok(())
}
