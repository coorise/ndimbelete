//! Cotisation money display helpers (debt / surplus sign convention).

use super::types::PaymentReceipt;

/// Stored balance: positive = debt owed, negative = surplus (Excel convention).
/// `intuitive` flips the sign for display (debt negative, surplus positive).
pub fn display_amount(stored: f64, intuitive: bool) -> f64 {
    if intuitive {
        -stored
    } else {
        stored
    }
}

pub fn format_money(v: f64) -> String {
    format!("{v:.2}")
}

pub fn format_stored_money(stored: f64, intuitive: bool) -> String {
    format_money(display_amount(stored, intuitive))
}

/// Color by economic meaning of *stored* value (independent of display sign).
pub fn debt_text_class(stored: f64) -> &'static str {
    if stored > 0.001 {
        "font-semibold text-[var(--brand-red)]"
    } else if stored < -0.001 {
        "font-semibold text-[var(--brand)]"
    } else {
        "text-[var(--fg)]"
    }
}

pub fn paid_text_class(paid: Option<f64>) -> &'static str {
    match paid {
        Some(p) if p.abs() > 0.001 => "font-semibold text-[var(--brand)]",
        _ => "text-[var(--fg)]",
    }
}

pub fn is_intuitive_sign(settings_sign: &str) -> bool {
    !settings_sign.eq_ignore_ascii_case("excel")
}

/// Receipt remaining / surplus from year obligation math.
/// `n_periods` = number of planning periods in the year (typically 6).
/// Returns `(year_total_due, remaining_to_pay, nouveau_solde_credits, surplus_received)`.
pub fn receipt_year_figures(
    period_amount: f64,
    prior_december_debt: f64,
    total_paid_year: f64,
    n_periods: i32,
) -> (f64, f64, f64, Option<f64>) {
    let n = if n_periods > 0 {
        n_periods as f64
    } else {
        6.0
    };
    let year_total_due = period_amount * n;
    let prior_debt = prior_december_debt.max(0.0);
    let surplus_prev = (-prior_december_debt).max(0.0);
    let credits = total_paid_year + surplus_prev;
    let obligation = year_total_due + prior_debt;
    let remaining_raw = obligation - credits;
    let surplus_received = if remaining_raw < -0.001 {
        Some(-remaining_raw)
    } else {
        None
    };
    let remaining = remaining_raw.max(0.0);
    (year_total_due, remaining, credits, surplus_received)
}

pub fn today_payment_date() -> String {
    let now = js_sys::Date::new_0();
    format!(
        "{:02}/{:02}/{}",
        now.get_date(),
        now.get_month() + 1,
        now.get_full_year()
    )
}

/// Format `YYYY-MM-DD` (or similar) as `DD/MM/YYYY`.
pub fn format_iso_date_fr(iso: &str) -> Option<String> {
    let parts: Vec<&str> = iso.trim().split('-').collect();
    if parts.len() < 3 || parts[0].len() != 4 {
        return None;
    }
    let y = parts[0];
    let m: u32 = parts[1].parse().ok()?;
    let d_str: String = parts[2]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let d: u32 = d_str.parse().ok()?;
    Some(format!("{d:02}/{m:02}/{y}"))
}

/// Prefer meeting date of the paid period; else 01/MM/YYYY from period month.
pub fn payment_date_for_period(
    meeting_date: Option<&str>,
    period_month: i32,
    year: i32,
) -> String {
    meeting_date
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(format_iso_date_fr)
        .unwrap_or_else(|| format!("01/{:02}/{year}", period_month.clamp(1, 12)))
}

/// Build a receipt with year-obligation fields for print / preview.
/// `total_paid_year` must be the raw sum of payments for the year (not including prior surplus).
/// `payment_date` should be the last real payment date when known.
pub fn build_payment_receipt(
    member_name: String,
    card_number: String,
    member_uid: String,
    period_label: String,
    amount: f64,
    balance_after: f64,
    total_paid_year: f64,
    year: i32,
    org_name: String,
    org_address: String,
    monthly_amount: f64,
    prior_december_debt: f64,
    payment_date: Option<String>,
) -> PaymentReceipt {
    let (_year_total, remaining, _credits, _surplus) =
        receipt_year_figures(monthly_amount, prior_december_debt, total_paid_year, 6);
    PaymentReceipt {
        member_name,
        card_number,
        member_uid,
        period_label,
        amount,
        debt_before: remaining,
        balance_after,
        total_paid_year,
        year,
        org_name,
        org_address,
        monthly_amount,
        prior_december_debt,
        payment_date: payment_date
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(today_payment_date),
    }
}
