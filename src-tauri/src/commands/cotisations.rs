//! Cotisation / year-grid commands.

use rusqlite::params;
use tauri::State;

use crate::db::seed_year;
use crate::models::{
    ContributionYear, Member, MemberDebtSummary, YearGrid, YearGridRow,
};
use crate::services::cotisation_engine::{
    clear_payment as cotisation_engine_clear_payment, compute_balance,
    ensure_member_period_entries, load_period_cells, load_periods, period_due_base,
    rebuild_running_dues, recalculate_all_members as engine_recalculate_all_members,
    recalculate_member_year, record_payment as cotisation_engine_record_payment,
    update_year_monthly_amount,
};
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

#[tauri::command]
pub fn list_years(state: State<'_, AppState>) -> Result<Vec<ContributionYear>, String> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, year, monthly_amount, sheet_label FROM contribution_years ORDER BY year DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ContributionYear {
                id: r.get(0)?,
                year: r.get(1)?,
                monthly_amount: r.get(2)?,
                sheet_label: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_year(state: State<'_, AppState>, year: i32) -> Result<ContributionYear, String> {
    let conn = state.db.lock();
    let id = seed_year(&conn, year).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, year, monthly_amount, sheet_label FROM contribution_years WHERE id = ?1",
        [&id],
        |r| {
            Ok(ContributionYear {
                id: r.get(0)?,
                year: r.get(1)?,
                monthly_amount: r.get(2)?,
                sheet_label: r.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_year_grid(state: State<'_, AppState>, year: i32) -> Result<YearGrid, String> {
    let conn = state.db.lock();

    let year_row: ContributionYear = conn
        .query_row(
            "SELECT id, year, monthly_amount, sheet_label FROM contribution_years WHERE year = ?1",
            [year],
            |r| {
                Ok(ContributionYear {
                    id: r.get(0)?,
                    year: r.get(1)?,
                    monthly_amount: r.get(2)?,
                    sheet_label: r.get(3)?,
                })
            },
        )
        .map_err(|_| {
            format!("Année {year} introuvable — utilisez ensure_year d'abord")
        })?;

    let periods = load_periods(&conn, &year_row.id)?;

    let mut members_stmt = conn
        .prepare(
            "SELECT id, card_number, last_name, first_name, adhesion_fee,
                    address, address_complement, postal_code, city, phone, email,
                    bank_transfer_status, status, notes, created_at, updated_at
             FROM members ORDER BY last_name, first_name",
        )
        .map_err(|e| e.to_string())?;

    let members: Vec<Member> = members_stmt
        .query_map([], map_member)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for member in members {
        ensure_member_period_entries(&conn, &member.id, &year_row.id, year_row.monthly_amount)?;
        // Excel-style running debt columns (fixes flat amount_due = monthly×2 defaults).
        rebuild_running_dues(&conn, &member.id, &year_row.id, year_row.monthly_amount)?;
        let summary = recalculate_member_year(&conn, &member.id, &year_row.id)?;

        let period_cells = load_period_cells(&conn, &member.id, &periods)?;

        // Re-read member status after demissionnaire recalc
        let member = conn
            .query_row(
                "SELECT id, card_number, last_name, first_name, adhesion_fee,
                        address, address_complement, postal_code, city, phone, email,
                        bank_transfer_status, status, notes, created_at, updated_at
                 FROM members WHERE id = ?1",
                [&member.id],
                map_member,
            )
            .unwrap_or(member);

        rows.push(YearGridRow {
            member,
            prior_december_debt: summary.prior_december_debt,
            ristourne: summary.ristourne,
            periods: period_cells,
            total_paid: summary.total_paid,
            december_debt: summary.december_debt,
            balance: summary.balance,
        });
    }

    Ok(YearGrid {
        year: year_row,
        periods,
        rows,
    })
}

#[tauri::command]
pub fn set_monthly_amount(
    state: State<'_, AppState>,
    year: i32,
    amount: f64,
) -> Result<ContributionYear, String> {
    let conn = state.db.lock();
    let year_id: String = conn
        .query_row(
            "SELECT id FROM contribution_years WHERE year = ?1",
            [year],
            |r| r.get(0),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;

    update_year_monthly_amount(&conn, &year_id, amount)?;

    conn.query_row(
        "SELECT id, year, monthly_amount, sheet_label FROM contribution_years WHERE id = ?1",
        [&year_id],
        |r| {
            Ok(ContributionYear {
                id: r.get(0)?,
                year: r.get(1)?,
                monthly_amount: r.get(2)?,
                sheet_label: r.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_payment(
    state: State<'_, AppState>,
    member_id: String,
    period_id: String,
    amount: f64,
) -> Result<MemberDebtSummary, String> {
    let conn = state.db.lock();
    cotisation_engine_record_payment(&conn, &member_id, &period_id, amount)
}

#[tauri::command]
pub fn clear_payment(
    state: State<'_, AppState>,
    member_id: String,
    period_id: String,
) -> Result<MemberDebtSummary, String> {
    let conn = state.db.lock();
    cotisation_engine_clear_payment(&conn, &member_id, &period_id)
}

/// Excel-like cell edit: set due and/or paid for one period cell.
#[tauri::command]
pub fn set_period_cell(
    state: State<'_, AppState>,
    member_id: String,
    period_id: String,
    field: String,
    value: Option<f64>,
) -> Result<MemberDebtSummary, String> {
    let conn = state.db.lock();
    crate::services::cotisation_engine::set_period_cell(
        &conn,
        &member_id,
        &period_id,
        &field,
        value,
    )
}

#[tauri::command]
pub fn get_member_debt(
    state: State<'_, AppState>,
    member_id: String,
    year: i32,
) -> Result<MemberDebtSummary, String> {
    let conn = state.db.lock();
    let (year_id, monthly): (String, f64) = conn
        .query_row(
            "SELECT id, monthly_amount FROM contribution_years WHERE year = ?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;

    ensure_member_period_entries(&conn, &member_id, &year_id, monthly)?;
    crate::services::cotisation_engine::recalculate_member_year(&conn, &member_id, &year_id)
}

/// Debt as of a given calendar month: prior + dues for periods <= as_of_month − paid − ristourne.
#[tauri::command]
pub fn get_member_debt_as_of(
    state: State<'_, AppState>,
    member_id: String,
    year: i32,
    as_of_month: i32,
) -> Result<MemberDebtSummary, String> {
    let conn = state.db.lock();
    let (year_id, monthly): (String, f64) = conn
        .query_row(
            "SELECT id, monthly_amount FROM contribution_years WHERE year = ?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;

    ensure_member_period_entries(&conn, &member_id, &year_id, monthly)?;

    let (prior, ristourne): (f64, f64) = conn
        .query_row(
            "SELECT prior_december_debt, ristourne FROM member_year_meta
             WHERE member_id = ?1 AND year_id = ?2",
            params![member_id, year_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0.0, 0.0));

    let periods = load_periods(&conn, &year_id)?;
    let mut total_paid = 0.0;
    for p in &periods {
        if p.period_month > as_of_month {
            continue;
        }
        let paid: Option<f64> = conn
            .query_row(
                "SELECT amount_paid FROM member_period_entries
                 WHERE member_id = ?1 AND period_id = ?2",
                params![member_id, p.id],
                |r| r.get(0),
            )
            .unwrap_or(None);
        if let Some(pmt) = paid {
            total_paid += pmt;
        }
    }

    // as-of dues = elapsed periods × base (not sum of cumulative amount_due)
    let n_elapsed = periods
        .iter()
        .filter(|p| p.period_month <= as_of_month)
        .count() as f64;
    let due_for_balance = n_elapsed * period_due_base(monthly);
    let balance = compute_balance(prior, due_for_balance, total_paid, ristourne);
    Ok(MemberDebtSummary {
        member_id,
        year,
        prior_december_debt: prior,
        total_due: due_for_balance,
        total_paid,
        ristourne,
        balance,
        december_debt: balance.max(0.0),
    })
}

#[tauri::command]
pub fn recalculate_all_members(
    state: State<'_, AppState>,
    year: i32,
) -> Result<usize, String> {
    let conn = state.db.lock();
    let year_id: String = conn
        .query_row(
            "SELECT id FROM contribution_years WHERE year = ?1",
            [year],
            |r| r.get(0),
        )
        .map_err(|_| format!("Année {year} introuvable"))?;
    engine_recalculate_all_members(&conn, &year_id)
}
