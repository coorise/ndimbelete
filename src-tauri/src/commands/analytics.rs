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
    /// Remaining debt only (Excel +), never mixed with surplus.
    pub total_debt: f64,
    /// Credit / surplus only (Excel − as positive magnitude).
    pub total_surplus: f64,
    /// Alias of `total_debt` (legacy field name).
    pub total_unpaid: f64,
    pub by_period: Vec<PeriodSeriesPoint>,
    pub payment_status_pie: Vec<PaymentStatusSlice>,
    pub debt_vs_paid: Vec<DebtVsPaidPoint>,
    pub paid_year_count: i64,
    pub unfulfilled_count: i64,
    pub with_debt_count: i64,
    #[serde(default)]
    pub with_surplus_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMemberRow {
    pub id: String,
    pub last_name: String,
    pub first_name: String,
    pub card_number: String,
    pub total_paid: f64,
    pub balance: f64,
}

#[derive(Clone)]
struct MemberSnap {
    id: String,
    status: String,
    prior: f64,
    paid: f64,
    ristourne: f64,
    paid_periods: i64,
    total_periods: i64,
    balance: f64,
}

fn matches_cohort(s: &MemberSnap, cohort: &str) -> bool {
    match cohort {
        "paid" | "solde" | "soldes" | "paid_year" => {
            s.total_periods > 0 && s.paid_periods >= s.total_periods
        }
        "unfulfilled" | "non_solde" | "nonsolde" | "unpaid" => {
            s.total_periods > 0 && s.paid_periods < s.total_periods
        }
        "debt" | "dette" | "with_debt" => s.balance > 0.001,
        "surplus" | "avec_surplus" | "with_surplus" => s.balance < -0.001,
        _ => true,
    }
}

#[tauri::command]
pub fn get_overview_stats(
    state: State<'_, AppState>,
    year: i32,
    member_id: Option<String>,
    status_filter: Option<String>,
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
    let n_periods: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contribution_periods WHERE year_id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .unwrap_or(6)
        .max(1);
    let year_dues_one = base * n_periods as f64;
    let cohort = status_filter
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let member_filter = member_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut snaps: Vec<MemberSnap> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.status,
                        COALESCE(meta.prior_december_debt, 0),
                        COALESCE(meta.total_paid, 0),
                        COALESCE(meta.ristourne, 0)
                 FROM members m
                 LEFT JOIN member_year_meta meta
                   ON meta.member_id = m.id AND meta.year_id = ?1
                 ORDER BY m.last_name, m.first_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&year_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, f64>(3)?,
                    r.get::<_, f64>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, status, prior, paid, ristourne) = row.map_err(|e| e.to_string())?;
            if let Some(ref mid) = member_filter {
                if &id != mid {
                    continue;
                }
            }
            let (paid_periods, total_periods): (i64, i64) = conn
                .query_row(
                    "SELECT
                        SUM(CASE WHEN e.amount_paid IS NOT NULL AND e.amount_paid > 0.001 THEN 1 ELSE 0 END),
                        COUNT(*)
                     FROM member_period_entries e
                     JOIN contribution_periods p ON p.id = e.period_id
                     WHERE p.year_id = ?1 AND e.member_id = ?2",
                    rusqlite::params![year_id, id],
                    |r| {
                        Ok((
                            r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                            r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        ))
                    },
                )
                .unwrap_or((0, 0));
            let balance = prior + year_dues_one - paid - ristourne;
            snaps.push(MemberSnap {
                id,
                status,
                prior,
                paid,
                ristourne,
                paid_periods,
                total_periods,
                balance,
            });
        }
    }

    let paid_year_count = snaps
        .iter()
        .filter(|s| s.total_periods > 0 && s.paid_periods >= s.total_periods)
        .count() as i64;
    let unfulfilled_count = snaps
        .iter()
        .filter(|s| s.total_periods > 0 && s.paid_periods < s.total_periods)
        .count() as i64;
    let with_debt_count = snaps.iter().filter(|s| s.balance > 0.001).count() as i64;
    let with_surplus_count = snaps.iter().filter(|s| s.balance < -0.001).count() as i64;

    let selected: Vec<MemberSnap> = snaps
        .into_iter()
        .filter(|s| matches_cohort(s, &cohort))
        .collect();
    let selected_ids: std::collections::HashSet<String> =
        selected.iter().map(|s| s.id.clone()).collect();

    let member_count = selected.len() as i64;
    let active_count = selected.iter().filter(|s| s.status == "active").count() as i64;
    let demissionnaire_count = selected
        .iter()
        .filter(|s| s.status == "demissionnaire")
        .count() as i64;
    let exclu_count = selected.iter().filter(|s| s.status == "exclu").count() as i64;

    let total_paid: f64 = selected.iter().map(|s| s.paid).sum();
    let total_due = year_dues_one * member_count as f64;
    let total_debt: f64 = selected.iter().map(|s| s.balance.max(0.0)).sum();
    let total_surplus: f64 = selected.iter().map(|s| (-s.balance).max(0.0)).sum();
    let prior_sum: f64 = selected.iter().map(|s| s.prior).sum();

    let mut periods_stmt = conn
        .prepare(
            "SELECT p.id, p.period_month, p.label
             FROM contribution_periods p
             WHERE p.year_id = ?1
             ORDER BY p.period_month",
        )
        .map_err(|e| e.to_string())?;
    let period_rows: Vec<(String, i32, String)> = periods_stmt
        .query_map([&year_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let by_period: Vec<PeriodSeriesPoint> = period_rows
        .iter()
        .map(|(pid, month, label)| {
            let mut paid_sum = 0.0_f64;
            if !selected_ids.is_empty() {
                if let Ok(mut st) = conn.prepare(
                    "SELECT e.member_id, COALESCE(e.amount_paid, 0)
                     FROM member_period_entries e
                     WHERE e.period_id = ?1",
                ) {
                    if let Ok(rows) =
                        st.query_map([pid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))
                    {
                        for row in rows.flatten() {
                            let (mid, paid) = row;
                            if selected_ids.contains(&mid) {
                                paid_sum += paid;
                            }
                        }
                    }
                }
            }
            let due = selected_ids.len() as f64 * base;
            PeriodSeriesPoint {
                period_month: *month,
                label: label.clone(),
                total_due: due,
                total_paid: paid_sum,
                unpaid: (due - paid_sum).max(0.0),
            }
        })
        .collect();

    let mut bal_run = prior_sum;
    let mut paid_run = 0.0_f64;
    let debt_vs_paid: Vec<DebtVsPaidPoint> = by_period
        .iter()
        .map(|p| {
            bal_run += p.total_due;
            paid_run += p.total_paid;
            bal_run -= p.total_paid;
            DebtVsPaidPoint {
                label: p.label.clone(),
                debt_cumulative: bal_run,
                paid_cumulative: paid_run,
            }
        })
        .collect();

    let fully_paid = selected
        .iter()
        .filter(|s| s.total_periods > 0 && s.paid_periods >= s.total_periods)
        .count() as i64;
    let unpaid = selected
        .iter()
        .filter(|s| s.total_periods > 0 && s.paid_periods == 0)
        .count() as i64;
    let with_entries = selected.iter().filter(|s| s.total_periods > 0).count() as i64;
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
        total_debt,
        total_surplus,
        total_unpaid: total_debt,
        by_period,
        payment_status_pie,
        debt_vs_paid,
        paid_year_count,
        unfulfilled_count,
        with_debt_count,
        with_surplus_count,
    })
}

/// List members for overview filters (kept for tooling; UI uses chart filters).
#[tauri::command]
pub fn list_overview_members(
    state: State<'_, AppState>,
    year: i32,
    category: String,
) -> Result<Vec<OverviewMemberRow>, String> {
    let conn = state.db.lock();
    let (year_id, monthly): (String, f64) = conn
        .query_row(
            "SELECT id, monthly_amount FROM contribution_years WHERE year = ?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;
    let base = period_due_base(monthly);
    let n_periods: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contribution_periods WHERE year_id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let year_dues_one = base * n_periods as f64;
    let cat = category.trim().to_ascii_lowercase();

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.last_name, m.first_name, m.card_number,
                    COALESCE(meta.prior_december_debt, 0),
                    COALESCE(meta.total_paid, 0),
                    COALESCE(meta.ristourne, 0)
             FROM members m
             LEFT JOIN member_year_meta meta
               ON meta.member_id = m.id AND meta.year_id = ?1
             ORDER BY m.last_name COLLATE NOCASE, m.first_name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&year_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, f64>(5)?,
                r.get::<_, f64>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        let (id, last_name, first_name, card_number, prior, total_paid, ristourne) =
            row.map_err(|e| e.to_string())?;
        let balance = prior + year_dues_one - total_paid - ristourne;

        let (paid_periods, total_periods): (i64, i64) = conn
            .query_row(
                "SELECT
                    SUM(CASE WHEN e.amount_paid IS NOT NULL AND e.amount_paid > 0.001 THEN 1 ELSE 0 END),
                    COUNT(*)
                 FROM member_period_entries e
                 JOIN contribution_periods p ON p.id = e.period_id
                 WHERE p.year_id = ?1 AND e.member_id = ?2",
                rusqlite::params![year_id, id],
                |r| {
                    Ok((
                        r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    ))
                },
            )
            .unwrap_or((0, 0));

        let snap = MemberSnap {
            id: id.clone(),
            status: String::new(),
            prior,
            paid: total_paid,
            ristourne,
            paid_periods,
            total_periods,
            balance,
        };
        if matches_cohort(&snap, &cat) {
            out.push(OverviewMemberRow {
                id,
                last_name,
                first_name,
                card_number,
                total_paid,
                balance,
            });
        }
    }
    Ok(out)
}
