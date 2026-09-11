//! Cotisation / contribution grid types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionYear {
    pub id: String,
    pub year: i32,
    pub monthly_amount: f64,
    pub sheet_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionPeriod {
    pub id: String,
    pub year_id: String,
    /// Bi-monthly: 1, 3, 5, 7, 9, 11.
    pub period_month: i32,
    pub label: String,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: Option<i32>,
    /// Hex color for cotisation column headers (optional).
    #[serde(default)]
    pub label_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPeriodEntry {
    pub id: String,
    pub member_id: String,
    pub period_id: String,
    pub amount_due: f64,
    /// `None` / null in DB = not paid yet.
    pub amount_paid: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberYearMeta {
    pub id: String,
    pub member_id: String,
    pub year_id: String,
    pub prior_december_debt: f64,
    pub ristourne: f64,
    pub total_paid: f64,
    pub december_debt: f64,
}

/// One cell pair in the year grid (due + paid for a period).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodCell {
    pub period_id: String,
    pub period_month: i32,
    pub label: String,
    pub amount_due: f64,
    pub amount_paid: Option<f64>,
    pub note: Option<String>,
}

/// One member row in the year cotisation grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearGridRow {
    pub member: crate::models::Member,
    pub prior_december_debt: f64,
    pub ristourne: f64,
    pub periods: Vec<PeriodCell>,
    pub total_paid: f64,
    pub december_debt: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearGrid {
    pub year: ContributionYear,
    pub periods: Vec<ContributionPeriod>,
    pub rows: Vec<YearGridRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberDebtSummary {
    pub member_id: String,
    pub year: i32,
    pub prior_december_debt: f64,
    pub total_due: f64,
    pub total_paid: f64,
    pub ristourne: f64,
    pub balance: f64,
    pub december_debt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordPaymentInput {
    pub member_id: String,
    pub period_id: String,
    pub amount: f64,
}
