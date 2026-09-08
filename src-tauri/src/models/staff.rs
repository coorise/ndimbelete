//! Staff (users who can log into the app).

use serde::{Deserialize, Serialize};

/// Split a legacy "full name" into (last_name / NOM, first_name / PRENOM).
/// Last whitespace-separated token is treated as the family name.
pub fn split_full_name(full: &str) -> (String, String) {
    let mut parts: Vec<&str> = full.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), String::new());
    }
    let last = parts.pop().unwrap_or("").to_string();
    let first = parts.join(" ");
    (last, first)
}

pub fn compose_full_name(last_name: &str, first_name: &str) -> String {
    format!("{} {}", last_name.trim(), first_name.trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Staff {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub role_id: String,
    pub role_name: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    /// Whether recovery school + color hints are configured (hashes never exposed).
    pub has_recovery_hints: bool,
    /// Earliest registered staff account — cannot be deactivated.
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
    /// Optional: only update password when provided and non-empty.
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

/// First-run primary administrator (Commissaire aux comptes).
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
    /// Non-empty → update recovery school hash. Empty / None → leave unchanged.
    pub recovery_school: Option<String>,
    /// Non-empty → update recovery color hash. Empty / None → leave unchanged.
    pub recovery_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordWithHintsInput {
    pub username: String,
    pub school_answer: String,
    pub color_answer: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStatus {
    pub username: String,
    pub has_hints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub staff: Staff,
    pub permissions: Vec<String>,
}
