//! Excel export — write a year cotisation sheet via rust_xlsxwriter.
//! Headers match the association import layout (duplicate date columns per period).

use rusqlite::{params, Connection};
use rust_xlsxwriter::{Format, Workbook};
use serde::{Deserialize, Serialize};

use crate::services::cotisation_engine::{load_period_cells, load_periods};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub rows_written: usize,
}

fn period_header_date(year: i32, period: &crate::models::ContributionPeriod) -> String {
    if let Some(ref md) = period.meeting_date {
        let t = md.trim();
        if t.len() >= 10 && t.as_bytes().get(4) == Some(&b'-') {
            return t[..10].to_string();
        }
        if let Some((d, m, y)) = parse_dmy(t) {
            return format!("{y:04}-{m:02}-{d:02}");
        }
    }
    // First day of the bi-monthly period month (Excel-like)
    format!("{year:04}-{:02}-01", period.period_month)
}

fn parse_dmy(s: &str) -> Option<(u32, u32, i32)> {
    let parts: Vec<&str> = s.split(|c| c == '/' || c == '-').collect();
    if parts.len() == 3 {
        // try DD/MM/YYYY
        if let (Ok(d), Ok(m), Ok(y)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<i32>(),
        ) {
            if d <= 31 && m <= 12 && y > 1900 {
                return Some((d, m, y));
            }
        }
        // try YYYY-MM-DD
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if d <= 31 && m <= 12 && y > 1900 {
                return Some((d, m, y));
            }
        }
    }
    None
}

/// Export all members for a year to an `.xlsx` file.
pub fn export_excel(conn: &Connection, path: &str, year: i32) -> Result<ExportResult, String> {
    let (year_id, _monthly): (String, f64) = conn
        .query_row(
            "SELECT id, monthly_amount FROM contribution_years WHERE year = ?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("Year {year} not found — call ensure_year first"))?;

    let periods = load_periods(conn, &year_id)?;

    let mut workbook = Workbook::new();
    let sheet = workbook
        .add_worksheet()
        .set_name(year.to_string())
        .map_err(|e| e.to_string())?;

    let header_fmt = Format::new().set_bold();

    let mut headers: Vec<String> = vec![
        "NOM".into(),
        "PRENOM".into(),
        "N°CARTE".into(),
        "cotisation adhesion".into(),
        format!("DETTE DECEMBRE {}", year - 1),
        "RISTOURNE".into(),
    ];
    for p in &periods {
        let date_h = period_header_date(year, p);
        // Two identical date headers (due, paid) — Excel parity
        headers.push(date_h.clone());
        headers.push(date_h);
    }
    headers.extend([
        format!("TOTAL {year}"),
        format!("DETTE DECEMBRE {year}"),
        "VIREMENT BANQUAIRE".into(),
        "ADRESSE".into(),
        "COMPLEMENT ADRESSE".into(),
        "CODE POSTAL".into(),
        "VILLE".into(),
        "PHONE".into(),
        "EMAIL".into(),
    ]);

    for (col, h) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, h, &header_fmt)
            .map_err(|e| e.to_string())?;
    }

    let mut members_stmt = conn
        .prepare(
            "SELECT id, card_number, last_name, first_name, adhesion_fee,
                    address, address_complement, postal_code, city, phone, email,
                    bank_transfer_status
             FROM members ORDER BY last_name, first_name",
        )
        .map_err(|e| e.to_string())?;

    let member_rows: Vec<_> = members_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, Option<String>>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut rows_written = 0usize;

    for (row_i, m) in member_rows.iter().enumerate() {
        let excel_row = (row_i + 1) as u32;
        let (
            id,
            card,
            last,
            first,
            adhesion,
            address,
            complement,
            cp,
            city,
            phone,
            email,
            virement,
        ) = m;

        let (prior, ristourne, total_paid, dec_debt): (f64, f64, f64, f64) = conn
            .query_row(
                "SELECT prior_december_debt, ristourne, total_paid, december_debt
                 FROM member_year_meta WHERE member_id=?1 AND year_id=?2",
                params![id, year_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap_or((0.0, 0.0, 0.0, 0.0));

        let cells = load_period_cells(conn, id, &periods)?;

        let mut col: u16 = 0;
        sheet
            .write_string(excel_row, col, last)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, first)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, card)
            .map_err(|e| e.to_string())?;
        col += 1;
        if *adhesion > 0.001 {
            sheet
                .write_number(excel_row, col, *adhesion)
                .map_err(|e| e.to_string())?;
        }
        col += 1;
        sheet
            .write_number(excel_row, col, prior)
            .map_err(|e| e.to_string())?;
        col += 1;
        if ristourne.abs() > 0.001 {
            sheet
                .write_number(excel_row, col, ristourne)
                .map_err(|e| e.to_string())?;
        }
        col += 1;

        for cell in &cells {
            sheet
                .write_number(excel_row, col, cell.amount_due)
                .map_err(|e| e.to_string())?;
            col += 1;
            if let Some(paid) = cell.amount_paid {
                sheet
                    .write_number(excel_row, col, paid)
                    .map_err(|e| e.to_string())?;
            }
            // else leave blank
            col += 1;
        }

        sheet
            .write_number(excel_row, col, total_paid)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_number(excel_row, col, dec_debt)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, virement.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, address.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, complement.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, cp.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, city.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, phone.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, email.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;

        rows_written += 1;
    }

    workbook
        .save(path)
        .map_err(|e| format!("Failed to save Excel: {e}"))?;

    Ok(ExportResult {
        path: path.to_string(),
        rows_written,
    })
}
