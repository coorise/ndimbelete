//! Excel import — read association cotisation sheets via calamine.
//!
//! Expected columns (flexible header matching):
//! NOM, PRENOM, N°CARTE, cotisation adhesion, DETTE DECEMBRE {prev},
//! RISTOURNE..., then due/paid pairs for Jan/Mar/May/Jul/Sep/Nov
//! (headers are often Excel DateTime values, duplicated per period),
//! TOTAL, DETTE DECEMBRE {year}, VIREMENT BANQUAIRE,
//! ADRESSE, COMPLEMENT, CODE POSTAL, VILLE, PHONE, EMAIL

use calamine::{open_workbook_auto, Data, Reader};
use chrono::Datelike;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::seed_year;
use crate::models::PERIOD_MONTHS;
use crate::services::cotisation_engine::{
    ensure_member_period_entries, period_due_base, rebuild_running_dues, recalculate_member_year,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelPreview {
    pub sheets: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailure {
    pub row: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub ok: usize,
    pub failed: Vec<ImportFailure>,
    /// Members removed because they were absent from the Excel sheet (replace sync).
    #[serde(default)]
    pub removed: usize,
}

fn cell_str(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => {
            if let Some(naive) = dt.as_datetime() {
                naive.format("%Y-%m-%d").to_string()
            } else {
                format!("{dt}")
            }
        }
        Data::DateTimeIso(s) => {
            // Prefer YYYY-MM-DD when ISO contains a date
            if let Some((y, m, d)) = parse_ymd(s) {
                format!("{y:04}-{m:02}-{d:02}")
            } else {
                s.clone()
            }
        }
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("ERR:{e:?}"),
    }
}

fn cell_f64(cell: &Data) -> Option<f64> {
    match cell {
        Data::Empty => None,
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => {
            let t = s.trim().replace(',', ".").replace(' ', "");
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        }
        _ => None,
    }
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let t = s.trim();
    // YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS
    if let Some(date_part) = t.split('T').next().or_else(|| t.split(' ').next()) {
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() == 3 {
            let y: i32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let d: u32 = parts[2].parse().ok()?;
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                return Some((y, m, d));
            }
        }
    }
    // DD/MM/YYYY or MM/YYYY
    if t.contains('/') {
        let parts: Vec<&str> = t.split('/').collect();
        if parts.len() == 3 {
            let d: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let y: i32 = parts[2].parse().ok()?;
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                return Some((y, m, d));
            }
        } else if parts.len() == 2 {
            let m: u32 = parts[0].parse().ok()?;
            let y: i32 = parts[1].parse().ok()?;
            if (1..=12).contains(&m) && y > 1900 {
                return Some((y, m, 1));
            }
        }
    }
    None
}

/// Extract calendar month (1–12) from a header cell (DateTime, ISO, or date-like string).
fn header_month(cell: &Data) -> Option<i32> {
    match cell {
        Data::DateTime(dt) => dt.as_datetime().map(|d| d.month() as i32),
        Data::DateTimeIso(s) => parse_ymd(s).map(|(_, m, _)| m as i32),
        Data::Float(f) => {
            // Excel serial date sometimes appears as float
            let dt = calamine::ExcelDateTime::new(*f, calamine::ExcelDateTimeType::DateTime, false);
            dt.as_datetime().map(|d| d.month() as i32)
        }
        _ => {
            let s = cell_str(cell);
            if let Some((_, m, _)) = parse_ymd(&s) {
                return Some(m as i32);
            }
            let n = norm_header(&s);
            month_from_name(&n)
        }
    }
}

fn month_from_name(n: &str) -> Option<i32> {
    const PAIRS: &[(&[&str], i32)] = &[
        (&["JANV", "JANUARY", "JAN"], 1),
        (&["MARS", "MARCH", "MAR"], 3),
        (&["MAI", "MAY"], 5),
        (&["JUIL", "JULY", "JUL"], 7),
        (&["SEPT", "SEPTEMBER", "SEP"], 9),
        (&["NOVEM", "NOVEMBER", "NOV"], 11),
    ];
    for (keys, month) in PAIRS {
        if keys.iter().any(|k| n.contains(k)) {
            return Some(*month);
        }
    }
    None
}

fn norm_header(h: &str) -> String {
    h.trim()
        .to_uppercase()
        .replace('\u{00A0}', " ")
        .replace('\n', " ")
        .chars()
        .map(|c| match c {
            'É' | 'È' | 'Ê' => 'E',
            'À' | 'Â' => 'A',
            'Ô' => 'O',
            'Ù' | 'Û' => 'U',
            'Ç' => 'C',
            '°' => 'O',
            _ => c,
        })
        .collect::<String>()
        .replace("  ", " ")
}

fn find_col(headers: &[String], predicates: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let n = norm_header(h);
        predicates.iter().any(|p| n.contains(p))
    })
}

/// Preview workbook: sheet names + headers/sample rows for the chosen sheet
/// (or the first sheet when `sheet_name` is `None`).
pub fn preview_excel(path: &str, sheet_name: Option<String>) -> Result<ExcelPreview, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("Cannot open Excel: {e}"))?;
    let sheets = workbook.sheet_names().to_vec();
    if sheets.is_empty() {
        return Ok(ExcelPreview {
            sheets,
            sample_rows: vec![],
            headers: vec![],
        });
    }

    let target = sheet_name
        .filter(|s| !s.trim().is_empty())
        .filter(|s| sheets.iter().any(|n| n == s))
        .unwrap_or_else(|| sheets[0].clone());

    let range = workbook
        .worksheet_range(&target)
        .map_err(|e| format!("Cannot read sheet '{target}': {e}"))?;

    let mut headers = Vec::new();
    let mut sample_rows = Vec::new();

    for (i, row) in range.rows().enumerate() {
        let cells: Vec<String> = row.iter().map(cell_str).collect();
        if i == 0 {
            headers = cells;
        } else if i <= 6 {
            sample_rows.push(cells);
        } else {
            break;
        }
    }

    Ok(ExcelPreview {
        sheets,
        sample_rows,
        headers,
    })
}

struct ColMap {
    nom: Option<usize>,
    prenom: Option<usize>,
    carte: Option<usize>,
    adhesion: Option<usize>,
    dette_prev: Option<usize>,
    ristourne: Option<usize>,
    /// (due_col, paid_col) for each of the 6 periods
    periods: Vec<(Option<usize>, Option<usize>)>,
    #[allow(dead_code)]
    total: Option<usize>,
    dette_year: Option<usize>,
    virement: Option<usize>,
    adresse: Option<usize>,
    complement: Option<usize>,
    cp: Option<usize>,
    ville: Option<usize>,
    phone: Option<usize>,
    email: Option<usize>,
}

fn is_meta_header(n: &str) -> bool {
    n.contains("DETTE")
        || n.contains("RISTOURNE")
        || n.contains("TOTAL")
        || n.contains("VIREMENT")
        || n.contains("ADRESSE")
        || n.contains("COMPLEMENT")
        || n.contains("POSTAL")
        || n.contains("VILLE")
        || n.contains("PHONE")
        || n.contains("EMAIL")
        || n.contains("ADHESION")
        || n == "NOM"
        || n.starts_with("NOM ")
        || n.contains("PRENOM")
        || n.contains("CARTE")
}

/// Map columns from raw header `Data` cells (DateTime period headers supported).
fn map_columns(header_cells: &[Data]) -> ColMap {
    let headers: Vec<String> = header_cells.iter().map(cell_str).collect();

    let mut period_cols: Vec<(Option<usize>, Option<usize>)> = Vec::new();

    // Prefer matching by month number from DateTime / parsed dates
    let mut matched_by_month = true;
    for &month in &PERIOD_MONTHS {
        let cols: Vec<usize> = header_cells
            .iter()
            .enumerate()
            .filter(|(i, cell)| {
                let n = norm_header(&headers[*i]);
                if is_meta_header(&n) {
                    return false;
                }
                header_month(cell) == Some(month)
            })
            .map(|(i, _)| i)
            .collect();

        if cols.len() >= 2 {
            period_cols.push((Some(cols[0]), Some(cols[1])));
        } else if cols.len() == 1 {
            period_cols.push((Some(cols[0]), None));
        } else {
            matched_by_month = false;
            period_cols.push((None, None));
        }
    }

    // Fallback: collect all date-like columns in order and pair consecutive (due, paid)
    let mapped_count = period_cols
        .iter()
        .filter(|(d, _)| d.is_some())
        .count();
    if !matched_by_month || mapped_count < 6 {
        let mut date_like: Vec<usize> = Vec::new();
        for (i, cell) in header_cells.iter().enumerate() {
            let n = norm_header(&headers[i]);
            if is_meta_header(&n) {
                continue;
            }
            if header_month(cell).is_some()
                || n.contains('/')
                || contains_month(&n)
                || matches!(cell, Data::DateTime(_) | Data::DateTimeIso(_))
            {
                date_like.push(i);
            }
        }

        if date_like.len() >= 2 {
            period_cols.clear();
            for chunk in date_like.chunks(2).take(6) {
                if chunk.len() >= 2 {
                    period_cols.push((Some(chunk[0]), Some(chunk[1])));
                } else {
                    period_cols.push((Some(chunk[0]), None));
                }
            }
        }
    }

    while period_cols.len() < 6 {
        period_cols.push((None, None));
    }
    period_cols.truncate(6);

    // DETTE DECEMBRE: first = prior year, last = current year end
    let mut dette_cols: Vec<usize> = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        if norm_header(h).contains("DETTE DECEMBRE") {
            dette_cols.push(i);
        }
    }
    let dette_prev = dette_cols.first().copied();
    let dette_year = dette_cols.last().copied();

    ColMap {
        nom: headers.iter().position(|h| {
            let n = norm_header(h);
            n == "NOM" || n.starts_with("NOM ")
        }),
        prenom: find_col(&headers, &["PRENOM"]),
        carte: headers.iter().position(|h| {
            let n = norm_header(h);
            n.contains("CARTE") || n.contains("NOCARTE")
        }),
        adhesion: find_col(&headers, &["ADHESION", "COTISATION ADHESION"]),
        dette_prev,
        ristourne: find_col(&headers, &["RISTOURNE"]),
        periods: period_cols,
        total: find_col(&headers, &["TOTAL", "TOTAU"]),
        dette_year,
        virement: find_col(&headers, &["VIREMENT"]),
        adresse: find_col(&headers, &["ADRESSE"]),
        complement: find_col(&headers, &["COMPLEMENT"]),
        cp: find_col(&headers, &["CODE POSTAL", "CP"]),
        ville: find_col(&headers, &["VILLE"]),
        phone: find_col(&headers, &["PHONE", "TEL", "TELEPHONE"]),
        email: find_col(&headers, &["EMAIL", "MAIL"]),
    }
}

fn contains_month(n: &str) -> bool {
    ["JANV", "MARS", "MAI", "JUIL", "SEPT", "NOVEM", "JAN", "MAR", "JUL", "SEP", "NOV"]
        .iter()
        .any(|m| n.contains(m))
}

fn get_cell<'a>(row: &'a [Data], idx: Option<usize>) -> Option<&'a Data> {
    idx.and_then(|i| row.get(i))
}

fn get_str(row: &[Data], idx: Option<usize>) -> String {
    get_cell(row, idx).map(cell_str).unwrap_or_default()
}

fn get_opt_f64(row: &[Data], idx: Option<usize>) -> Option<f64> {
    get_cell(row, idx).and_then(cell_f64)
}

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Import a sheet into the given year (creates year if missing).
pub fn import_excel(
    conn: &Connection,
    path: &str,
    sheet_name: &str,
    year: i32,
) -> Result<ImportResult, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("Cannot open Excel: {e}"))?;
    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| format!("Cannot read sheet '{sheet_name}': {e}"))?;

    let mut rows_iter = range.rows();
    let header_row = rows_iter
        .next()
        .ok_or_else(|| "Sheet is empty".to_string())?;
    let header_cells: Vec<Data> = header_row.to_vec();
    let cols = map_columns(&header_cells);

    if cols.carte.is_none() {
        return Err("Could not find N°CARTE column in sheet headers".into());
    }

    let year_id = seed_year(conn, year).map_err(|e| e.to_string())?;
    let monthly: f64 = conn
        .query_row(
            "SELECT monthly_amount FROM contribution_years WHERE id = ?1",
            [&year_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let base_due = period_due_base(monthly);

    let periods: Vec<(String, i32)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, period_month FROM contribution_periods WHERE year_id = ?1 ORDER BY period_month",
            )
            .map_err(|e| e.to_string())?;
        let r = stmt
            .query_map([&year_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        r.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let mut ok = 0usize;
    let mut failed = Vec::new();
    let mut seen_cards: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (row_idx, row) in rows_iter.enumerate() {
        let excel_row = row_idx + 2; // 1-based + header
        if is_summary_row(row, &cols) {
            continue;
        }
        let card = get_str(row, cols.carte);
        if card.is_empty() {
            let nom = get_str(row, cols.nom);
            if nom.is_empty() {
                continue;
            }
            failed.push(ImportFailure {
                row: excel_row,
                reason: "Missing N°CARTE".into(),
            });
            continue;
        }

        match import_one_row(
            conn,
            row,
            &cols,
            &card,
            &year_id,
            &periods,
            base_due,
            monthly,
        ) {
            Ok(()) => {
                ok += 1;
                seen_cards.insert(card);
            }
            Err(reason) => failed.push(ImportFailure {
                row: excel_row,
                reason,
            }),
        }
    }

    let _ = conn.execute(
        "UPDATE contribution_years SET sheet_label = ?1 WHERE id = ?2",
        params![sheet_name, year_id],
    );

    // Excel is the source of truth: drop members absent from this sheet.
    let seen_list: Vec<String> = seen_cards.into_iter().collect();
    let removed = prune_members_not_in_import(conn, &seen_list)?;

    // Final pass: rebuild dues + restore demissionnaire → active where appropriate
    let _ = crate::services::cotisation_engine::recalculate_all_members(conn, &year_id);

    Ok(ImportResult {
        ok,
        failed,
        removed,
    })
}

/// Totals / legend rows at the bottom of association sheets (no identity).
fn is_summary_row(row: &[Data], cols: &ColMap) -> bool {
    let card = get_str(row, cols.carte);
    let nom = get_str(row, cols.nom);
    let prenom = get_str(row, cols.prenom);
    if !card.is_empty() || !nom.is_empty() || !prenom.is_empty() {
        return false;
    }
    // Row with only numeric aggregates in period / total columns
    let mut numeric = 0usize;
    for (due, paid) in &cols.periods {
        if get_opt_f64(row, *due).is_some() {
            numeric += 1;
        }
        if get_opt_f64(row, *paid).is_some() {
            numeric += 1;
        }
    }
    if get_opt_f64(row, cols.total).is_some() {
        numeric += 1;
    }
    numeric >= 2
}

fn prune_members_not_in_import(conn: &Connection, seen_cards: &[String]) -> Result<usize, String> {
    if seen_cards.is_empty() {
        return Ok(0);
    }
    // Keep only cards present in this import.
    let placeholders = (1..=seen_cards.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("DELETE FROM members WHERE card_number NOT IN ({placeholders})");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let n = stmt
        .execute(rusqlite::params_from_iter(seen_cards.iter()))
        .map_err(|e| e.to_string())?;
    Ok(n)
}

fn import_one_row(
    conn: &Connection,
    row: &[Data],
    cols: &ColMap,
    card: &str,
    year_id: &str,
    periods: &[(String, i32)],
    base_due: f64,
    monthly: f64,
) -> Result<(), String> {
    let last_name = get_str(row, cols.nom);
    let first_name = get_str(row, cols.prenom);
    if last_name.is_empty() && first_name.is_empty() {
        return Err("Missing NOM and PRENOM".into());
    }

    let adhesion = get_opt_f64(row, cols.adhesion).unwrap_or(0.0);
    let prior_debt = get_opt_f64(row, cols.dette_prev).unwrap_or(0.0);
    let ristourne = get_opt_f64(row, cols.ristourne).unwrap_or(0.0);
    let dec_debt = get_opt_f64(row, cols.dette_year).unwrap_or(0.0);
    let virement = {
        let v = get_str(row, cols.virement);
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };
    let address = nonempty(get_str(row, cols.adresse));
    let complement = nonempty(get_str(row, cols.complement));
    let cp = nonempty(get_str(row, cols.cp));
    let ville = nonempty(get_str(row, cols.ville));
    let phone = nonempty(get_str(row, cols.phone));
    let email = nonempty(get_str(row, cols.email));
    let now = now_iso();

    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM members WHERE card_number = ?1",
        [card],
        |r| r.get(0),
    );

    let member_id = match existing {
        Ok(id) => {
            conn.execute(
                "UPDATE members SET last_name=?1, first_name=?2, adhesion_fee=?3,
                 address=?4, address_complement=?5, postal_code=?6, city=?7,
                 phone=?8, email=?9, bank_transfer_status=?10, updated_at=?11
                 WHERE id=?12",
                params![
                    last_name,
                    first_name,
                    adhesion,
                    address,
                    complement,
                    cp,
                    ville,
                    phone,
                    email,
                    virement,
                    now,
                    id,
                ],
            )
            .map_err(|e| e.to_string())?;
            id
        }
        Err(_) => {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO members (
                    id, card_number, last_name, first_name, adhesion_fee,
                    address, address_complement, postal_code, city, phone, email,
                    bank_transfer_status, status, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'active',NULL,?13,?13)",
                params![
                    id, card, last_name, first_name, adhesion, address, complement, cp, ville,
                    phone, email, virement, now
                ],
            )
            .map_err(|e| e.to_string())?;
            id
        }
    };

    ensure_member_period_entries(conn, &member_id, year_id, monthly)?;

    conn.execute(
        "UPDATE member_year_meta SET prior_december_debt=?1, ristourne=?2, december_debt=?3
         WHERE member_id=?4 AND year_id=?5",
        params![prior_debt, ristourne, dec_debt, member_id, year_id],
    )
    .map_err(|e| e.to_string())?;

    // Write Excel due/paid; then rebuild_running_dues keeps Excel parity from payments
    for (i, (period_id, _month)) in periods.iter().enumerate() {
        if i >= PERIOD_MONTHS.len() || i >= cols.periods.len() {
            break;
        }
        let (due_col, paid_col) = cols.periods[i];
        let due = get_opt_f64(row, due_col).unwrap_or(base_due);
        let paid = get_opt_f64(row, paid_col);

        conn.execute(
            "UPDATE member_period_entries SET amount_due=?1, amount_paid=?2
             WHERE member_id=?3 AND period_id=?4",
            params![due, paid, member_id, period_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Prefer stored Excel amount_due; rebuild after payments to keep later periods consistent
    rebuild_running_dues(conn, &member_id, year_id, monthly)?;
    recalculate_member_year(conn, &member_id, year_id)?;
    Ok(())
}

fn nonempty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, seed_year};
    use rusqlite::Connection;
    use std::path::PathBuf;

    fn sample_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".samples/files/Fichier cotisation - juillet 2026.xlsm")
    }

    #[test]
    fn map_columns_pairs_due_and_paid() {
        let mut wb = open_workbook_auto(sample_path()).unwrap();
        let range = wb.worksheet_range("2026").unwrap();
        let header: Vec<Data> = range.rows().next().unwrap().to_vec();
        let cols = map_columns(&header);
        assert_eq!(cols.periods.len(), 6);
        for (i, (due, paid)) in cols.periods.iter().enumerate() {
            assert!(due.is_some(), "period {i} missing due");
            assert!(
                paid.is_some(),
                "period {i} missing paid — due={due:?} paid={paid:?}"
            );
            assert_eq!(
                paid.unwrap(),
                due.unwrap() + 1,
                "paid should be the column immediately after due"
            );
        }
    }

    #[test]
    fn import_sample_preserves_paid_amounts() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let _ = seed_year(&conn, 2026);
        let res = import_excel(
            &conn,
            sample_path().to_str().unwrap(),
            "2026",
            2026,
        )
        .expect("import");
        eprintln!("ok={} failed={}", res.ok, res.failed.len());
        let (n, sum): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount_paid),0) FROM member_period_entries
                 WHERE amount_paid IS NOT NULL AND amount_paid > 0.001",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        eprintln!("paid_n={n} paid_sum={sum}");
        assert!(n > 200, "expected many paid cells, got {n}");
        assert!(
            (sum - 17050.0).abs() < 50.0,
            "expected ~17050 paid from Excel gray columns, got {sum}"
        );
    }
}
