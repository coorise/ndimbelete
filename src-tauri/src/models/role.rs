//! Roles and permissions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    /// JSON array of permission strings, e.g. `["*"]` or `["members:read"]`.
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
