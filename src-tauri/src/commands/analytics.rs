//! Analytics / dashboard overview stats.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::services::cotisation_engine::period_due_base;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodSeriesPoint {
    pub period_month: i32,
    pub label: String,
    pub total_due: f64,
    pub total_paid: f64,
    pub unpaid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatusSlice {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtVsPaidPoint {
    pub label: String,
    pub debt_cumulative: f64,
    pub paid_cumulative: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewStats {
    pub year: i32,
    pub member_count: i64,
    pub active_count: i64,
    pub demissionnaire_count: i64,
    pub exclu_count: i64,
    pub total_paid: f64,
    pub total_due: f64,
    pub total_unpaid: f64,
    pub by_period: Vec<PeriodSeriesPoint>,
    pub payment_status_pie: Vec<PaymentStatusSlice>,
    pub debt_vs_paid: Vec<DebtVsPaidPoint>,
}

#[tauri::command]
pub fn get_overview_stats(
    state: State<'_, AppState>,
    year: i32,
    member_id: Option<String>,
) -> Result<OverviewStats, String> {
    let conn = state.db.lock();

    let (year_id, monthly): (String, f64) = conn
        .query_row(
            "SELECT id, monthly_amount FROM contribution_years WHERE year = ?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;

    let base = period_due_base(monthly);

    let member_filter = member_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (member_count, active_count, demissionnaire_count, exclu_count) =
        if let Some(ref mid) = member_filter {
            let status: String = conn
                .query_row("SELECT status FROM members WHERE id = ?1", [mid], |r| {
                    r.get(0)
                })
                .unwrap_or_else(|_| "active".into());
            let active = if status == "active" { 1 } else { 0 };
            let dem = if status == "demissionnaire" { 1 } else { 0 };
            let exclu = if status == "exclu" { 1 } else { 0 };
            (1i64, active, dem, exclu)
        } else {
            let member_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM members", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            let active_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM members WHERE status = 'active'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let demissionnaire_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM members WHERE status = 'demissionnaire'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let exclu_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM members WHERE status = 'exclu'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            (
                member_count,
                active_count,
                demissionnaire_count,
                exclu_count,
            )
        };

    // Period due = base × members (not Excel running amount_due, which double-counts).
    let by_period: Vec<PeriodSeriesPoint> = if let Some(ref mid) = member_filter {
        let mut stmt = conn
            .prepare(
                "SELECT p.period_month, p.label,
                        COALESCE(CASE WHEN e.amount_paid IS NOT NULL THEN e.amount_paid ELSE 0 END, 0)
                 FROM contribution_periods p
                 LEFT JOIN member_period_entries e
                   ON e.period_id = p.id AND e.member_id = ?2
                 WHERE p.year_id = ?1
                 ORDER BY p.period_month",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<PeriodSeriesPoint> = stmt
            .query_map(rusqlite::params![year_id, mid], |r| {
                let total_paid: f64 = r.get(2)?;
                let total_due = base;
                Ok(PeriodSeriesPoint {
                    period_month: r.get(0)?,
                    label: r.get(1)?,
                    total_due,
                    total_paid,
                    unpaid: (total_due - total_paid).max(0.0),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT p.period_month, p.label,
                        COUNT(e.member_id),
                        COALESCE(SUM(CASE WHEN e.amount_paid IS NOT NULL THEN e.amount_paid ELSE 0 END), 0)
                 FROM contribution_periods p
                 LEFT JOIN member_period_entries e ON e.period_id = p.id
                 WHERE p.year_id = ?1
                 GROUP BY p.id
                 ORDER BY p.period_month",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<PeriodSeriesPoint> = stmt
            .query_map([&year_id], |r| {
                let entry_count: i64 = r.get(2)?;
                let total_paid: f64 = r.get(3)?;
                let total_due = entry_count as f64 * base;
                Ok(PeriodSeriesPoint {
                    period_month: r.get(0)?,
                    label: r.get(1)?,
                    total_due,
                    total_paid,
                    unpaid: (total_due - total_paid).max(0.0),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };

    let total_due: f64 = by_period.iter().map(|p| p.total_due).sum();
    let total_paid: f64 = by_period.iter().map(|p| p.total_paid).sum();
    let total_unpaid = (total_due - total_paid).max(0.0);

    // True cumulative curves for the line chart.
    let mut debt_run = 0.0_f64;
    let mut paid_run = 0.0_f64;
    let debt_vs_paid: Vec<DebtVsPaidPoint> = by_period
        .iter()
        .map(|p| {
            debt_run += p.unpaid;
            paid_run += p.total_paid;
            DebtVsPaidPoint {
                label: p.label.clone(),
                debt_cumulative: debt_run,
                paid_cumulative: paid_run,
            }
        })
        .collect();

    // Payment status based on real payments (amount_paid), not vs cumulative amount_due.
    let (fully_paid, unpaid, with_entries) = if let Some(ref mid) = member_filter {
        let (paid_periods, total_periods): (i64, i64) = conn
            .query_row(
                "SELECT
                    SUM(CASE WHEN e.amount_paid IS NOT NULL AND e.amount_paid > 0.001 THEN 1 ELSE 0 END),
                    COUNT(*)
                 FROM member_period_entries e
                 JOIN contribution_periods p ON p.id = e.period_id
                 WHERE p.year_id = ?1 AND e.member_id = ?2",
                rusqlite::params![year_id, mid],
                |r| {
                    Ok((
                        r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    ))
                },
            )
            .unwrap_or((0, 0));
        if total_periods <= 0 {
            (0i64, 0i64, 0i64)
        } else if paid_periods >= total_periods {
            (1, 0, 1)
        } else if paid_periods == 0 {
            (0, 1, 1)
        } else {
            (0, 0, 1)
        }
    } else {
        let fully_paid: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
                    SELECT e.member_id
                    FROM member_period_entries e
                    JOIN contribution_periods p ON p.id = e.period_id
                    WHERE p.year_id = ?1
                    GROUP BY e.member_id
                    HAVING SUM(CASE WHEN e.amount_paid IS NOT NULL AND e.amount_paid > 0.001 THEN 1 ELSE 0 END)
                         = COUNT(*)
                       AND COUNT(*) > 0
                 )",
                [&year_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let unpaid: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
                    SELECT e.member_id
                    FROM member_period_entries e
                    JOIN contribution_periods p ON p.id = e.period_id
                    WHERE p.year_id = ?1
                    GROUP BY e.member_id
                    HAVING SUM(CASE WHEN e.amount_paid IS NOT NULL AND e.amount_paid > 0.001 THEN 1 ELSE 0 END) = 0
                       AND COUNT(*) > 0
                 )",
                [&year_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let with_entries: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT e.member_id)
                 FROM member_period_entries e
                 JOIN contribution_periods p ON p.id = e.period_id
                 WHERE p.year_id = ?1",
                [&year_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (fully_paid, unpaid, with_entries)
    };

    let partial = (with_entries - fully_paid - unpaid).max(0);

    let payment_status_pie = vec![
        PaymentStatusSlice {
            label: "Soldé".into(),
            count: fully_paid,
        },
        PaymentStatusSlice {
            label: "Partiel".into(),
            count: partial,
        },
        PaymentStatusSlice {
            label: "Impayé".into(),
            count: unpaid,
        },
    ];

    Ok(OverviewStats {
        year,
        member_count,
        active_count,
        demissionnaire_count,
        exclu_count,
        total_paid,
        total_due,
        total_unpaid,
        by_period,
        payment_status_pie,
        debt_vs_paid,
    })
}
