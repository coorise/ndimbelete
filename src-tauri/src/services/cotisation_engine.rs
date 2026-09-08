//! Cotisation calculation engine.
//!
//! Business rules:
//! - Each bi-monthly period base due = `monthly_amount * 2`
//! - `amount_due` is stored Excel-style as running debt before that period's payment
//! - Balance = prior_debt + n*base − sum(paid) − ristourne
//! - A period is unpaid when `amount_paid` is None or ≈0 (not paid < cumulative due)
//! - After unpaid spanning ≥ 6 months → status `demissionnaire`
//!   (3 consecutive unpaid bi-monthly periods, OR cumulative unpaid months ≥ 6)

use chrono::Datelike;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::{ContributionPeriod, MemberDebtSummary, MemberStatus, PeriodCell};

/// Bi-monthly period base due amount.
pub fn period_due_base(monthly_amount: f64) -> f64 {
    monthly_amount * 2.0
}

/// Compute running balance for a member in a year.
pub fn compute_balance(
    prior_december_debt: f64,
    total_due: f64,
    total_paid: f64,
    ristourne: f64,
) -> f64 {
    prior_december_debt + total_due - total_paid - ristourne
}

/// Apply surplus carry-forward across ordered period cells (pure helper).
#[allow(dead_code)]
pub fn apply_surplus_carry(cells: &[(f64, Option<f64>)]) -> Vec<f64> {
    let mut surplus = 0.0_f64;
    let mut effective_dues = Vec::with_capacity(cells.len());

    for &(due, paid) in cells {
        let adjusted_due = (due - surplus).max(0.0);
        effective_dues.push(adjusted_due);

        if let Some(p) = paid {
            if p > adjusted_due {
                surplus = p - adjusted_due;
            } else {
                surplus = 0.0;
            }
        } else {
            surplus = 0.0;
        }
    }

    effective_dues
}

fn is_unpaid_payment(paid: Option<f64>) -> bool {
    paid.map(|p| p.abs() < 0.001).unwrap_or(true)
}

/// Count consecutive unpaid bi-monthly periods (longest streak) and cumulative unpaid.
/// Returns unpaid months (= unpaid_periods * 2).
///
/// A period counts as unpaid when there is positive running debt and no real payment
/// (`amount_paid` is None or ≈0). Do **not** compare paid against cumulative `amount_due`.
///
/// Only cells whose `period_month` is `<= as_of_month` are considered — future
/// unpaid periods must not mark a member démissionnaire mid-year.
pub fn unpaid_months(cells: &[(i32, f64, Option<f64>)], as_of_month: i32) -> i32 {
    let relevant: Vec<(f64, Option<f64>)> = cells
        .iter()
        .filter(|(month, _, _)| *month <= as_of_month)
        .map(|(_, due, paid)| (*due, *paid))
        .collect();

    let mut max_streak = 0i32;
    let mut current = 0i32;

    for &(due, paid) in &relevant {
        // due > 0: skip periods already covered by prior surplus / credit
        let is_unpaid = due > 0.001 && is_unpaid_payment(paid);
        if is_unpaid {
            current += 1;
            max_streak = max_streak.max(current);
        } else {
            current = 0;
        }
    }

    let total_unpaid = relevant
        .iter()
        .filter(|(due, paid)| *due > 0.001 && is_unpaid_payment(*paid))
        .count() as i32;

    let consecutive_months = max_streak * 2;
    let cumulative_months = total_unpaid * 2;
    consecutive_months.max(cumulative_months)
}

pub fn should_mark_demissionnaire(
    cells: &[(i32, f64, Option<f64>)],
    as_of_month: Option<i32>,
) -> bool {
    let as_of = as_of_month.unwrap_or_else(|| chrono::Local::now().month() as i32);
    unpaid_months(cells, as_of) >= 6
}

/// Ensure period entries exist for a member+year, creating missing ones at base due.
pub fn ensure_member_period_entries(
    conn: &Connection,
    member_id: &str,
    year_id: &str,
    monthly_amount: f64,
) -> Result<(), String> {
    let periods = load_periods(conn, year_id)?;
    let due = period_due_base(monthly_amount);

    for p in &periods {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM member_period_entries WHERE member_id = ?1 AND period_id = ?2",
                params![member_id, p.id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        if exists == 0 {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO member_period_entries (id, member_id, period_id, amount_due, amount_paid, note)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
                params![id, member_id, p.id, due],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Ensure year meta row
    let meta_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM member_year_meta WHERE member_id = ?1 AND year_id = ?2",
            params![member_id, year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    if meta_exists == 0 {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO member_year_meta (id, member_id, year_id, prior_december_debt, ristourne, total_paid, december_debt)
             VALUES (?1, ?2, ?3, 0, 0, 0, 0)",
            params![id, member_id, year_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn load_periods(conn: &Connection, year_id: &str) -> Result<Vec<ContributionPeriod>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, year_id, period_month, label,
                    meeting_date, collect_start, collect_end, sort_order
             FROM contribution_periods
             WHERE year_id = ?1
             ORDER BY COALESCE(sort_order, period_month), period_month ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([year_id], |r| {
            Ok(ContributionPeriod {
                id: r.get(0)?,
                year_id: r.get(1)?,
                period_month: r.get(2)?,
                label: r.get(3)?,
                meeting_date: r.get(4)?,
                collect_start: r.get(5)?,
                collect_end: r.get(6)?,
                sort_order: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Rebuild Excel-style running `amount_due` for each period (debt column before payment).
///
/// ```text
/// running = prior - ristourne
/// for each period:
///   running += base
///   amount_due = running
///   running -= paid.unwrap_or(0)
/// ```
pub fn rebuild_running_dues(
    conn: &Connection,
    member_id: &str,
    year_id: &str,
    monthly_amount: f64,
) -> Result<(), String> {
    let (prior, ristourne): (f64, f64) = conn
        .query_row(
            "SELECT prior_december_debt, ristourne FROM member_year_meta
             WHERE member_id = ?1 AND year_id = ?2",
            params![member_id, year_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0.0, 0.0));

    let periods = load_periods(conn, year_id)?;
    let base = period_due_base(monthly_amount);
    let mut running = prior - ristourne;

    for p in &periods {
        let paid: Option<f64> = conn
            .query_row(
                "SELECT amount_paid FROM member_period_entries
                 WHERE member_id = ?1 AND period_id = ?2",
                params![member_id, p.id],
                |r| r.get(0),
            )
            .unwrap_or(None);

        running += base;
        conn.execute(
            "UPDATE member_period_entries SET amount_due = ?1
             WHERE member_id = ?2 AND period_id = ?3",
            params![running, member_id, p.id],
        )
        .map_err(|e| e.to_string())?;

        running -= paid.unwrap_or(0.0);
    }

    Ok(())
}

/// Recalculate totals / december debt / demissionnaire flag after a payment change.
pub fn recalculate_member_year(
    conn: &Connection,
    member_id: &str,
    year_id: &str,
) -> Result<MemberDebtSummary, String> {
    let (year, monthly): (i32, f64) = conn
        .query_row(
            "SELECT year, monthly_amount FROM contribution_years WHERE id = ?1",
            [year_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let (prior, ristourne): (f64, f64) = conn
        .query_row(
            "SELECT prior_december_debt, ristourne FROM member_year_meta
             WHERE member_id = ?1 AND year_id = ?2",
            params![member_id, year_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0.0, 0.0));

    let periods = load_periods(conn, year_id)?;
    let base = period_due_base(monthly);
    // (period_month, due, paid)
    let mut cells_raw: Vec<(i32, f64, Option<f64>)> = Vec::new();
    let mut total_paid = 0.0;

    for p in &periods {
        let (due, paid): (f64, Option<f64>) = conn
            .query_row(
                "SELECT amount_due, amount_paid FROM member_period_entries
                 WHERE member_id = ?1 AND period_id = ?2",
                params![member_id, p.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0.0, None));

        if let Some(pmt) = paid {
            total_paid += pmt;
        }
        cells_raw.push((p.period_month, due, paid));
    }

    // Year dues for balance = n × base (not sum of cumulative Excel due columns)
    let total_due = periods.len() as f64 * base;
    let balance = compute_balance(prior, total_due, total_paid, ristourne);
    let december_debt = balance.max(0.0);

    conn.execute(
        "UPDATE member_year_meta SET total_paid = ?1, december_debt = ?2
         WHERE member_id = ?3 AND year_id = ?4",
        params![total_paid, december_debt, member_id, year_id],
    )
    .map_err(|e| e.to_string())?;

    let current: String = conn
        .query_row(
            "SELECT status FROM members WHERE id = ?1",
            [member_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| MemberStatus::Active.as_str().into());

    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Skip mass demissionnaire marking when the year has zero recorded payments
    // (typical after Excel import with empty paid columns).
    let year_has_payments: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM member_period_entries e
             JOIN contribution_periods p ON p.id = e.period_id
             WHERE p.year_id = ?1
               AND e.amount_paid IS NOT NULL
               AND e.amount_paid > 0.001",
            [year_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if year_has_payments > 0 && should_mark_demissionnaire(&cells_raw, None) {
        if current == MemberStatus::Active.as_str() {
            conn.execute(
                "UPDATE members SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![MemberStatus::Demissionnaire.as_str(), now, member_id],
            )
            .map_err(|e| e.to_string())?;
        }
    } else if current == MemberStatus::Demissionnaire.as_str()
        && (year_has_payments == 0 || !should_mark_demissionnaire(&cells_raw, None))
    {
        // Restore actifs wrongly marked when due was cumulative Excel debt / empty paid cols
        conn.execute(
            "UPDATE members SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![MemberStatus::Active.as_str(), now, member_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(MemberDebtSummary {
        member_id: member_id.to_string(),
        year,
        prior_december_debt: prior,
        total_due,
        total_paid,
        ristourne,
        balance,
        december_debt,
    })
}

/// Recalculate every member for a year (rebuild running dues + status).
pub fn recalculate_all_members(conn: &Connection, year_id: &str) -> Result<usize, String> {
    let monthly: f64 = conn
        .query_row(
            "SELECT monthly_amount FROM contribution_years WHERE id = ?1",
            [year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id FROM members")
        .map_err(|e| e.to_string())?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut count = 0usize;
    for member_id in &ids {
        ensure_member_period_entries(conn, member_id, year_id, monthly)?;
        rebuild_running_dues(conn, member_id, year_id, monthly)?;
        recalculate_member_year(conn, member_id, year_id)?;
        count += 1;
    }
    Ok(count)
}

/// Clear a period payment (`amount_paid = NULL`), then recalculate dues/balance.
pub fn clear_payment(
    conn: &Connection,
    member_id: &str,
    period_id: &str,
) -> Result<MemberDebtSummary, String> {
    let year_id: String = conn
        .query_row(
            "SELECT year_id FROM contribution_periods WHERE id = ?1",
            [period_id],
            |r| r.get(0),
        )
        .map_err(|_| "Period not found".to_string())?;

    let monthly: f64 = conn
        .query_row(
            "SELECT monthly_amount FROM contribution_years WHERE id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    ensure_member_period_entries(conn, member_id, &year_id, monthly)?;

    conn.execute(
        "UPDATE member_period_entries SET amount_paid = NULL
         WHERE member_id = ?1 AND period_id = ?2",
        params![member_id, period_id],
    )
    .map_err(|e| e.to_string())?;

    rebuild_running_dues(conn, member_id, &year_id, monthly)?;
    recalculate_member_year(conn, member_id, &year_id)
}

/// Record a payment for one period, then recalculate.
pub fn record_payment(
    conn: &Connection,
    member_id: &str,
    period_id: &str,
    amount: f64,
) -> Result<MemberDebtSummary, String> {
    if amount < 0.0 {
        return Err("Payment amount cannot be negative".into());
    }

    let year_id: String = conn
        .query_row(
            "SELECT year_id FROM contribution_periods WHERE id = ?1",
            [period_id],
            |r| r.get(0),
        )
        .map_err(|_| "Period not found".to_string())?;

    let monthly: f64 = conn
        .query_row(
            "SELECT monthly_amount FROM contribution_years WHERE id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    ensure_member_period_entries(conn, member_id, &year_id, monthly)?;

    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM member_period_entries WHERE member_id = ?1 AND period_id = ?2",
        params![member_id, period_id],
        |r| r.get(0),
    );

    match existing {
        Ok(entry_id) => {
            conn.execute(
                "UPDATE member_period_entries SET amount_paid = ?1 WHERE id = ?2",
                params![amount, entry_id],
            )
            .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let id = Uuid::new_v4().to_string();
            let due = period_due_base(monthly);
            conn.execute(
                "INSERT INTO member_period_entries (id, member_id, period_id, amount_due, amount_paid, note)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![id, member_id, period_id, due, amount],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    rebuild_running_dues(conn, member_id, &year_id, monthly)?;
    recalculate_member_year(conn, member_id, &year_id)
}

/// Excel-like: update one cell (`due` or `paid`). Empty paid clears payment.
pub fn set_period_cell(
    conn: &Connection,
    member_id: &str,
    period_id: &str,
    field: &str,
    value: Option<f64>,
) -> Result<MemberDebtSummary, String> {
    let year_id: String = conn
        .query_row(
            "SELECT year_id FROM contribution_periods WHERE id = ?1",
            [period_id],
            |r| r.get(0),
        )
        .map_err(|_| "Période introuvable".to_string())?;

    let monthly: f64 = conn
        .query_row(
            "SELECT monthly_amount FROM contribution_years WHERE id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    ensure_member_period_entries(conn, member_id, &year_id, monthly)?;

    match field {
        "paid" => match value {
            Some(v) if v > 0.001 => record_payment(conn, member_id, period_id, v),
            _ => clear_payment(conn, member_id, period_id),
        },
        "due" => {
            let due = value.unwrap_or(0.0).max(0.0);
            conn.execute(
                "UPDATE member_period_entries SET amount_due = ?1
                 WHERE member_id = ?2 AND period_id = ?3",
                params![due, member_id, period_id],
            )
            .map_err(|e| e.to_string())?;
            recalculate_member_year(conn, member_id, &year_id)
        }
        other => Err(format!("Champ inconnu: {other}")),
    }
}

/// Load period cells for a member+year (for grid display).
pub fn load_period_cells(
    conn: &Connection,
    member_id: &str,
    periods: &[ContributionPeriod],
) -> Result<Vec<PeriodCell>, String> {
    let mut cells = Vec::new();
    for p in periods {
        let (due, paid, note): (f64, Option<f64>, Option<String>) = conn
            .query_row(
                "SELECT amount_due, amount_paid, note FROM member_period_entries
                 WHERE member_id = ?1 AND period_id = ?2",
                params![member_id, p.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap_or((0.0, None, None));

        cells.push(PeriodCell {
            period_id: p.id.clone(),
            period_month: p.period_month,
            label: p.label.clone(),
            amount_due: due,
            amount_paid: paid,
            note,
        });
    }
    Ok(cells)
}

/// Update all members' amount_due when monthly_amount changes.
pub fn update_year_monthly_amount(
    conn: &Connection,
    year_id: &str,
    new_monthly: f64,
) -> Result<(), String> {
    if new_monthly < 0.0 {
        return Err("Monthly amount cannot be negative".into());
    }

    conn.execute(
        "UPDATE contribution_years SET monthly_amount = ?1 WHERE id = ?2",
        params![new_monthly, year_id],
    )
    .map_err(|e| e.to_string())?;

    recalculate_all_members(conn, year_id)?;
    Ok(())
}
