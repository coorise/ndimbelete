//! Period helpers — bi-monthly cotisation calendar.

use serde::{Deserialize, Serialize};

/// Bi-monthly period months used by the association.
pub const PERIOD_MONTHS: [i32; 6] = [1, 3, 5, 7, 9, 11];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodInfo {
    pub month: i32,
    pub label: String,
}

/// French short labels for the six bi-monthly periods.
pub fn period_label(month: i32) -> &'static str {
    match month {
        1 => "Janvier",
        3 => "Mars",
        5 => "Mai",
        7 => "Juillet",
        9 => "Septembre",
        11 => "Novembre",
        _ => "Inconnu",
    }
}

pub fn all_periods() -> Vec<PeriodInfo> {
    PERIOD_MONTHS
        .iter()
        .map(|&m| PeriodInfo {
            month: m,
            label: period_label(m).to_string(),
        })
        .collect()
}
