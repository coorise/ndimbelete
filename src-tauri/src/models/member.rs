//! Member domain types.

use serde::{Deserialize, Serialize};

/// Allowed member statuses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Demissionnaire,
    Exclu,
}

impl MemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Demissionnaire => "demissionnaire",
            Self::Exclu => "exclu",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "demissionnaire" => Self::Demissionnaire,
            "exclu" => Self::Exclu,
            _ => Self::Active,
        }
    }
}

/// Default Excel « VIREMENT BANQUAIRE » values treated as member roles (not payment methods).
pub const DEFAULT_VIREMENT_ROLE_VALUES: &[&str] = &[
    "EXEMPTE",
    "Maladie",
    "Commissaire",
    "President",
    "Bureautique",
    "Retraite",
    "TRESORIER",
    "Aide bureaux",
];

/// Payment method stored on the member (UI columns + Excel reconstruction).
pub const PAYMENT_METHOD_CASH: &str = "cash";
pub const PAYMENT_METHOD_BANK: &str = "bank_transfer";
pub const PAYMENT_METHOD_NONE: &str = "none";

/// Member-role permission keys (association members, not staff).
pub const MEMBER_PERM_CAN_PAY: &str = "member:can_pay";
pub const MEMBER_PERM_PAY_BANK: &str = "member:pay_bank_transfer";
pub const MEMBER_PERM_PAY_CASH: &str = "member:pay_cash";

pub const NORMAL_MEMBER_ROLE: &str = "Normal";

/// Normalize payment method strings from the UI / DB.
pub fn normalize_payment_method(s: &str) -> String {
    match s.trim().to_lowercase().as_str() {
        "bank" | "bank_transfer" | "virement" => PAYMENT_METHOD_BANK.into(),
        "none" | "aucun" => PAYMENT_METHOD_NONE.into(),
        _ => PAYMENT_METHOD_CASH.into(),
    }
}

/// Rebuild the Excel « VIREMENT BANQUAIRE » cell from role + payment method.
///
/// - Role without payment (exempt / officers) → role name
/// - Bank transfer → `VIREMENT`
/// - Cash → empty
pub fn reconstruct_virement_cell(role_name: &str, payment_method: &str) -> String {
    let pm = normalize_payment_method(payment_method);
    if pm == PAYMENT_METHOD_NONE {
        let name = role_name.trim();
        if name.is_empty() || name.eq_ignore_ascii_case(NORMAL_MEMBER_ROLE) {
            String::new()
        } else {
            name.to_string()
        }
    } else if pm == PAYMENT_METHOD_BANK {
        "VIREMENT".into()
    } else {
        String::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRole {
    pub id: String,
    pub name: String,
    /// JSON array of permission strings for association members.
    pub permissions_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemberRoleInput {
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemberRoleInput {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Raw / reconstructed Excel « VIREMENT BANQUAIRE » cell for export round-trip.
    pub bank_transfer_status: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub member_role_id: Option<String>,
    #[serde(default)]
    pub payment_method: String,
    #[serde(default)]
    pub member_role_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemberInput {
    pub card_number: String,
    pub last_name: String,
    pub first_name: String,
    #[serde(default)]
    pub adhesion_fee: f64,
    pub address: Option<String>,
    pub address_complement: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub bank_transfer_status: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub member_role_id: Option<String>,
    #[serde(default)]
    pub payment_method: Option<String>,
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
    #[serde(default)]
    pub bank_transfer_status: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub member_role_id: Option<String>,
    #[serde(default)]
    pub payment_method: Option<String>,
}
