//! Excel export — write a year cotisation sheet via rust_xlsxwriter.
//! Headers match the association import layout (duplicate date columns per period),
//! with column fills close to the traditional Excel workbook.

use rusqlite::{params, Connection};
use rust_xlsxwriter::{Color, Format, Workbook};
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
    format!("{year:04}-{:02}-01", period.period_month)
}

fn parse_dmy(s: &str) -> Option<(u32, u32, i32)> {
    let parts: Vec<&str> = s.split(|c| c == '/' || c == '-').collect();
    if parts.len() == 3 {
        if let (Ok(d), Ok(m), Ok(y)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<i32>(),
        ) {
            if d <= 31 && m <= 12 && y > 1900 {
                return Some((d, m, y));
            }
        }
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

    let fmt_plain = Format::new().set_bold();
    let fmt_carte = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x8B6914))
        .set_font_color(Color::White);
    let fmt_adhesion = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF4B183));
    let fmt_dette = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xFF0000));
    let fmt_ristourne = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xED7D31));
    let fmt_due_hdr = Format::new()
        .set_bold()
        .set_background_color(Color::Black)
        .set_font_color(Color::White);
    let fmt_paid_hdr = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x808080))
        .set_font_color(Color::White);
    let fmt_total = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x548235))
        .set_font_color(Color::White);
    let fmt_virement = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xD9D9D9));
    let fmt_adresse = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x8FAADC));
    let fmt_compl = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xB4A7D6));

    let cell_dette = Format::new().set_background_color(Color::RGB(0xFF0000));
    let cell_adhesion = Format::new().set_background_color(Color::RGB(0xF4B183));
    let cell_ristourne = Format::new().set_background_color(Color::RGB(0xED7D31));
    let cell_total = Format::new().set_background_color(Color::RGB(0xC6EFCE));

    let mut headers: Vec<(String, Format)> = vec![
        ("NOM".into(), fmt_plain.clone()),
        ("PRENOM".into(), fmt_plain.clone()),
        ("N°CARTE".into(), fmt_carte),
        ("cotisation adhesion".into(), fmt_adhesion),
        (format!("DETTE DECEMBRE {}", year - 1), fmt_dette.clone()),
        ("RISTOURNE séjour Sénégal ou Maladie".into(), fmt_ristourne),
    ];
    for p in &periods {
        let date_h = period_header_date(year, p);
        headers.push((date_h.clone(), fmt_due_hdr.clone()));
        headers.push((date_h, fmt_paid_hdr.clone()));
    }
    headers.extend([
        (format!("TOTAL {year}"), fmt_total),
        (format!("DETTE DECEMBRE {year}"), fmt_dette),
        ("VIREMENT BANQUAIRE".into(), fmt_virement),
        ("ADRESSE".into(), fmt_adresse),
        ("COMPLEMENT ADRESSE".into(), fmt_compl),
        ("CODE POSTAL".into(), fmt_plain.clone()),
        ("VILLE".into(), fmt_plain.clone()),
        ("PHONE".into(), fmt_plain.clone()),
        ("EMAIL".into(), fmt_plain),
    ]);

    for (col, (h, fmt)) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, h, fmt)
            .map_err(|e| e.to_string())?;
    }

    let mut members_stmt = conn
        .prepare(
            "SELECT m.id, m.card_number, m.last_name, m.first_name, m.adhesion_fee,
                    m.address, m.address_complement, m.postal_code, m.city, m.phone, m.email,
                    m.bank_transfer_status, m.payment_method, COALESCE(mr.name, 'Normal')
             FROM members m
             LEFT JOIN member_roles mr ON mr.id = m.member_role_id
             ORDER BY m.last_name, m.first_name",
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
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
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
            virement_stored,
            payment_method,
            role_name,
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

        // Prefer stored Excel cell; fall back to role + payment reconstruction.
        let virement = virement_stored
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                crate::models::reconstruct_virement_cell(role_name, payment_method)
            });

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
                .write_number_with_format(excel_row, col, *adhesion, &cell_adhesion)
                .map_err(|e| e.to_string())?;
        } else {
            sheet
                .write_string_with_format(excel_row, col, "", &cell_adhesion)
                .map_err(|e| e.to_string())?;
        }
        col += 1;
        sheet
            .write_number_with_format(excel_row, col, prior, &cell_dette)
            .map_err(|e| e.to_string())?;
        col += 1;
        if ristourne.abs() > 0.001 {
            sheet
                .write_number_with_format(excel_row, col, ristourne, &cell_ristourne)
                .map_err(|e| e.to_string())?;
        } else {
            sheet
                .write_string_with_format(excel_row, col, "", &cell_ristourne)
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
            col += 1;
        }

        sheet
            .write_number_with_format(excel_row, col, total_paid, &cell_total)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_number_with_format(excel_row, col, dec_debt, &cell_dette)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write_string(excel_row, col, &virement)
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
