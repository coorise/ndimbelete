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
    /// Selected planning months (1–12). Empty = whole year.
    #[serde(default)]
    pub period_months: Vec<i32>,
    /// Legacy: first selected month, or None when whole year / multi.
    #[serde(default)]
    pub period_month: Option<i32>,
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
    /// Paid amount in scope (year total or selected months).
    paid: f64,
    #[allow(dead_code)]
    ristourne: f64,
    paid_periods: i64,
    total_periods: i64,
    /// Balance in scope (year-end or as-of max selected month).
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

fn normalize_months(raw: Option<Vec<i32>>) -> Vec<i32> {
    let mut out: Vec<i32> = raw
        .unwrap_or_default()
        .into_iter()
        .filter(|m| (1..=12).contains(m))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Running Excel debt after payment for the last period ≤ `as_of`.
fn balance_as_of_cells(period_cells: &[(i32, f64, f64)], as_of: i32, prior: f64, ristourne: f64) -> f64 {
    period_cells
        .iter()
        .filter(|(pm, _, _)| *pm <= as_of)
        .last()
        .map(|(_, due, paid)| due - paid)
        .unwrap_or(prior - ristourne)
}

#[tauri::command]
pub fn get_overview_stats(
    state: State<'_, AppState>,
    year: i32,
    member_id: Option<String>,
    status_filter: Option<String>,
    period_months: Option<Vec<i32>>,
    period_month: Option<i32>,
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
    let cohort = status_filter
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let mut months = normalize_months(period_months);
    if months.is_empty() {
        if let Some(m) = period_month.filter(|m| (1..=12).contains(m)) {
            months.push(m);
        }
    }
    let month_scope = !months.is_empty();
    let as_of = months.iter().copied().max();

    let member_filter = member_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

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

    let n_periods = period_rows.len().max(1) as i64;
    let year_dues_one = base * n_periods as f64;
    let scope_period_count = if month_scope {
        months.len().max(1) as i64
    } else {
        n_periods
    };

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
            let (id, status, prior, year_paid, ristourne) = row.map_err(|e| e.to_string())?;
            if let Some(ref mid) = member_filter {
                if &id != mid {
                    continue;
                }
            }

            let period_cells: Vec<(i32, f64, f64)> = {
                let mut st = conn
                    .prepare(
                        "SELECT p.period_month,
                                COALESCE(e.amount_due, 0),
                                COALESCE(e.amount_paid, 0)
                         FROM contribution_periods p
                         LEFT JOIN member_period_entries e
                           ON e.period_id = p.id AND e.member_id = ?2
                         WHERE p.year_id = ?1
                         ORDER BY p.period_month",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = st
                    .query_map(rusqlite::params![year_id, id], |r| {
                        Ok((r.get::<_, i32>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?))
                    })
                    .map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row.map_err(|e| e.to_string())?);
                }
                out
            };

            let (paid, paid_periods, total_periods, balance) = if month_scope {
                let paid_amt: f64 = period_cells
                    .iter()
                    .filter(|(pm, _, _)| months.contains(pm))
                    .map(|(_, _, p)| *p)
                    .sum();
                let paid_flag = months
                    .iter()
                    .filter(|m| {
                        period_cells
                            .iter()
                            .any(|(pm, _, p)| pm == *m && *p > 0.001)
                    })
                    .count() as i64;
                let total = months.len() as i64;
                let bal = balance_as_of_cells(
                    &period_cells,
                    as_of.unwrap_or(12),
                    prior,
                    ristourne,
                );
                (paid_amt, paid_flag, total, bal)
            } else {
                let paid_periods = period_cells
                    .iter()
                    .filter(|(_, _, p)| *p > 0.001)
                    .count() as i64;
                let total_periods = period_cells.len() as i64;
                let bal = prior + year_dues_one - year_paid - ristourne;
                (year_paid, paid_periods, total_periods, bal)
            };

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
    let total_debt: f64 = selected.iter().map(|s| s.balance.max(0.0)).sum();
    let total_surplus: f64 = selected.iter().map(|s| (-s.balance).max(0.0)).sum();
    let prior_sum: f64 = selected.iter().map(|s| s.prior).sum();

    // KPI "dû" = members × period base × scoped periods (never Excel running dues).
    let total_due = base * scope_period_count as f64 * member_count as f64;

    let periods_for_chart: Vec<(String, i32, String)> = if month_scope {
        period_rows
            .iter()
            .filter(|(_, month, _)| months.contains(month))
            .cloned()
            .collect()
    } else {
        period_rows.clone()
    };

    let periods_for_curve: Vec<(String, i32, String)> = match as_of {
        Some(m) => period_rows
            .iter()
            .filter(|(_, month, _)| *month <= m)
            .cloned()
            .collect(),
        None => period_rows.clone(),
    };

    let chart_points = |rows: &[(String, i32, String)]| -> Vec<PeriodSeriesPoint> {
        rows.iter()
            .map(|(pid, month, label)| {
                let mut paid_sum = 0.0_f64;
                if !selected_ids.is_empty() {
                    if let Ok(mut st) = conn.prepare(
                        "SELECT e.member_id, COALESCE(e.amount_paid, 0)
                         FROM member_period_entries e
                         WHERE e.period_id = ?1",
                    ) {
                        if let Ok(qrows) = st.query_map([pid], |r| {
                            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                        }) {
                            for row in qrows.flatten() {
                                let (mid, paid) = row;
                                if selected_ids.contains(&mid) {
                                    paid_sum += paid;
                                }
                            }
                        }
                    }
                }
                let due_sum = selected_ids.len() as f64 * base;
                PeriodSeriesPoint {
                    period_month: *month,
                    label: label.clone(),
                    total_due: due_sum,
                    total_paid: paid_sum,
                    unpaid: (due_sum - paid_sum).max(0.0),
                }
            })
            .collect()
    };

    let by_period = chart_points(&periods_for_chart);
    let curve_period = chart_points(&periods_for_curve);

    let mut bal_run = prior_sum;
    let mut paid_run = 0.0_f64;
    let debt_vs_paid: Vec<DebtVsPaidPoint> = curve_period
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
            label: if month_scope {
                "Payé".into()
            } else {
                "Soldé".into()
            },
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
        period_months: months.clone(),
        period_month: if months.len() == 1 {
            months.first().copied()
        } else {
            None
        },
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
