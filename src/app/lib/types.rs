//! Domain types mirrored from the Tauri backend (snake_case serde).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Staff {
    pub id: String,
    pub username: String,
    pub full_name: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub phone: Option<String>,
    pub role_id: String,
    pub role_name: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub has_recovery_hints: bool,
    #[serde(default)]
    pub is_founder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStaffInput {
    pub username: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub role_id: String,
    pub recovery_school: Option<String>,
    pub recovery_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStaffInput {
    pub id: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub role_id: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupAdminInput {
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub password: String,
    pub phone: Option<String>,
    pub recovery_school: Option<String>,
    pub recovery_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMyProfileInput {
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub recovery_school: Option<String>,
    pub recovery_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordWithHintsInput {
    pub username: String,
    pub school_answer: String,
    pub color_answer: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryStatus {
    pub username: String,
    pub has_hints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub staff: Staff,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub permissions_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleInput {
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleInput {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub id: String,
    pub card_number: String,
    pub last_name: String,
    pub first_name: String,
    pub adhesion_fee: f64,
    pub address: Option<String>,
    pub address_complement: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bank_transfer_status: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateMemberInput {
    pub card_number: String,
    pub last_name: String,
    pub first_name: String,
    pub adhesion_fee: f64,
    pub address: Option<String>,
    pub address_complement: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bank_transfer_status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemberInput {
    pub id: String,
    pub card_number: String,
    pub last_name: String,
    pub first_name: String,
    pub adhesion_fee: f64,
    pub address: Option<String>,
    pub address_complement: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bank_transfer_status: Option<String>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributionYear {
    pub id: String,
    pub year: i32,
    pub monthly_amount: f64,
    pub sheet_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributionPeriod {
    pub id: String,
    pub year_id: String,
    pub period_month: i32,
    pub label: String,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodCell {
    pub period_id: String,
    pub period_month: i32,
    pub label: String,
    pub amount_due: f64,
    pub amount_paid: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YearGridRow {
    pub member: Member,
    pub prior_december_debt: f64,
    pub ristourne: f64,
    pub periods: Vec<PeriodCell>,
    pub total_paid: f64,
    pub december_debt: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YearGrid {
    pub year: ContributionYear,
    pub periods: Vec<ContributionPeriod>,
    pub rows: Vec<YearGridRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptField {
    pub id: String,
    pub content: String,
    pub position: i32,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    #[serde(default = "default_align")]
    pub align: String,
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_align() -> String {
    "left".into()
}

fn default_color() -> String {
    "#000000".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub org_name: String,
    pub org_description: String,
    pub org_address: String,
    pub theme_color: String,
    pub font_scale: f64,
    pub logo_path: String,
    #[serde(default = "default_currency_unit")]
    pub currency_unit: String,
    #[serde(default = "default_a4_template")]
    pub receipt_template_a4: String,
    #[serde(default = "default_mini_template")]
    pub receipt_template_mini: String,
    #[serde(default = "default_mini_width")]
    pub receipt_mini_width_mm: f64,
    #[serde(default = "default_editor_mode")]
    pub receipt_editor_mode_a4: String,
    #[serde(default = "default_editor_mode")]
    pub receipt_editor_mode_mini: String,
    #[serde(default = "default_a4_fields")]
    pub receipt_fields_a4: Vec<ReceiptField>,
    #[serde(default = "default_mini_fields")]
    pub receipt_fields_mini: Vec<ReceiptField>,
}

fn default_currency_unit() -> String {
    "EUR".into()
}

fn default_editor_mode() -> String {
    "template".into()
}

fn default_a4_template() -> String {
    plain_lines_to_html(
        "{{ORG.NAME}}\n{{ORG.ADDRESS}}\n\nReçu de cotisation — {{COTISATION.YEAR}}\n{{DATE}}\n------------------------------\nMembre : {{USER.NAME}}\nN° carte : {{USER.CARD}}\nPériode : {{COTISATION.MONTH}}/{{COTISATION.YEAR}}\n\n**Montant Reçu : {{RECEIVED_AMOUNT}}**\nDette antérieure : {{REMAINING_DEBT}}\nNouveau Solde : {{NEW_BALANCE}}\n\nMerci pour votre solidarité\nNDIMBELENTÉ",
    )
}

fn default_mini_template() -> String {
    plain_lines_to_html(
        "{{ORG.NAME}}\nReçu de cotisation — {{COTISATION.YEAR}}\n{{DATE}}\n------------------------------\nMembre : {{USER.NAME}}\nN° carte : {{USER.CARD}}\nPériode : {{COTISATION.MONTH}}/{{COTISATION.YEAR}}\n**Montant Reçu : {{RECEIVED_AMOUNT}}**\nDette antérieure : {{REMAINING_DEBT}}\nNouveau Solde : {{NEW_BALANCE}}\n\nMerci pour votre solidarité\nNDIMBELENTÉ",
    )
}

fn plain_lines_to_html(plain: &str) -> String {
    plain
        .lines()
        .map(|line| {
            let t = line.trim_end();
            if t.is_empty() {
                "<div><br></div>".into()
            } else if let Some(inner) = t.strip_prefix("**").and_then(|s| s.strip_suffix("**")) {
                format!("<div><b>{inner}</b></div>")
            } else {
                format!("<div>{t}</div>")
            }
        })
        .collect()
}

fn default_mini_width() -> f64 {
    58.0
}

fn template_to_fields(tpl: &str) -> Vec<ReceiptField> {
    let plain = tpl
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    let mut stripped = String::new();
    let mut in_tag = false;
    for c in plain.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => stripped.push(c),
            _ => {}
        }
    }
    stripped
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let t = line.trim();
            let (content, bold, italic) = if let Some(inner) =
                t.strip_prefix("**").and_then(|s| s.strip_suffix("**"))
            {
                (inner.to_string(), true, false)
            } else if let Some(inner) = t.strip_prefix('*').and_then(|s| s.strip_suffix('*')) {
                if !inner.contains('*') {
                    (inner.to_string(), false, true)
                } else {
                    (t.to_string(), false, false)
                }
            } else {
                (t.to_string(), false, false)
            };
            ReceiptField {
                id: format!("f-{i}"),
                content,
                position: i as i32,
                font_size: if i == 0 { 12.0 } else { 10.0 },
                bold: bold || i == 0,
                italic,
                align: default_align(),
                color: default_color(),
            }
        })
        .collect()
}

fn default_a4_fields() -> Vec<ReceiptField> {
    template_to_fields(&default_a4_template())
}

fn default_mini_fields() -> Vec<ReceiptField> {
    template_to_fields(&default_mini_template())
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            org_name: "NDIMBELENTÉ:  Association Sénégalaise d'Entraide.".into(),
            org_description: "Association d'entraide à but non lucratif — gestion des cotisations".into(),
            org_address: "59 rue Fontaine au roi, 75011 PARIS".into(),
            theme_color: "#0B7A3E".into(),
            font_scale: 1.0,
            logo_path: String::new(),
            currency_unit: default_currency_unit(),
            receipt_template_a4: default_a4_template(),
            receipt_template_mini: default_mini_template(),
            receipt_mini_width_mm: default_mini_width(),
            receipt_editor_mode_a4: default_editor_mode(),
            receipt_editor_mode_mini: default_editor_mode(),
            receipt_fields_a4: default_a4_fields(),
            receipt_fields_mini: default_mini_fields(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodSeriesPoint {
    pub period_month: i32,
    pub label: String,
    pub total_due: f64,
    pub total_paid: f64,
    pub unpaid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentStatusSlice {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebtVsPaidPoint {
    pub label: String,
    pub debt_cumulative: f64,
    pub paid_cumulative: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewStats {
    pub year: i32,
    pub member_count: i64,
    pub active_count: i64,
    pub demissionnaire_count: i64,
    pub exclu_count: i64,
    pub total_paid: f64,
    pub total_due: f64,
    pub total_unpaid: f64,
    pub by_period: Vec<PeriodSeriesPoint>,
    pub payment_status_pie: Vec<PaymentStatusSlice>,
    pub debt_vs_paid: Vec<DebtVsPaidPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanningPeriod {
    pub id: String,
    pub year: i32,
    pub period_month: i32,
    pub label: String,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPlanningInput {
    pub id: Option<String>,
    pub year: i32,
    pub period_month: i32,
    pub label: Option<String>,
    pub meeting_date: Option<String>,
    pub collect_start: Option<String>,
    pub collect_end: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExcelPreview {
    pub sheets: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportFailure {
    pub row: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportResult {
    pub ok: usize,
    pub failed: Vec<ImportFailure>,
    #[serde(default)]
    pub removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportResult {
    pub path: String,
    pub rows_written: usize,
}

/// Receipt payload kept in UI after a successful payment.
#[derive(Debug, Clone, PartialEq)]
pub struct PaymentReceipt {
    pub member_name: String,
    pub card_number: String,
    pub member_uid: String,
    pub period_label: String,
    pub amount: f64,
    pub debt_before: f64,
    /// Remaining debt after this payment.
    pub balance_after: f64,
    /// Cumulative amount paid for the year (maps to {{NEW_BALANCE}} / « Nouveau Solde »).
    pub total_paid_year: f64,
    pub year: i32,
    pub org_name: String,
    pub org_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPrintPayload {
    pub org_name: String,
    #[serde(default)]
    pub org_address: String,
    pub member_name: String,
    pub card_number: String,
    #[serde(default)]
    pub member_uid: String,
    pub period_label: String,
    pub amount: f64,
    #[serde(default)]
    pub debt_before: f64,
    pub balance_after: f64,
    /// Total paid for the year — shown as Nouveau Solde / {{NEW_BALANCE}}.
    #[serde(default)]
    pub total_paid_year: f64,
    pub year: i32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrintReceiptInput {
    pub printer_name: String,
    pub receipt: ReceiptPrintPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrintReceiptResult {
    pub pdf_path: String,
    pub format: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub printer_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppVersionInfo {
    pub version: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub channel: String,
    pub release_name: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub body: String,
    pub message: String,
}

pub const BANK_TRANSFER_VALUES: &[&str] = &[
    "",
    "VIREMENT",
    "EXEMPTE",
    "Maladie",
    "Commissaire",
    "President",
    "Bureautique",
    "Retraite",
    "TRESORIER",
    "Aide bureaux",
];
