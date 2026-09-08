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

/// Allowed VIREMENT BANQUAIRE values (Excel column).
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
    pub bank_transfer_status: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
