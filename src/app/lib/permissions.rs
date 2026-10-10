//! Canonical staff permission keys and RBAC helpers.

/// (key, French label) — shown in the staff Roles editor.
pub const PERMISSION_OPTIONS: &[(&str, &str)] = &[
    ("*", "Toutes les permissions"),
    ("members:read", "Membres — lecture"),
    ("members:write", "Membres — écriture"),
    ("cotisations:read", "Cotisations — lecture"),
    ("cotisations:write", "Cotisations — écriture"),
    ("analytics:read", "Statistiques — lecture"),
    ("excel:import", "Excel — import"),
    ("excel:export", "Excel — export"),
    ("settings:read", "Paramètres — lecture"),
    ("settings:write", "Paramètres — écriture"),
    ("staff:read", "Personnel — lecture"),
    ("staff:write", "Personnel — écriture"),
    ("planning:read", "Planning — lecture"),
    ("planning:write", "Planning — écriture"),
    ("roles:read", "Rôles — lecture"),
    ("roles:write", "Rôles — écriture"),
    ("collab:push", "Collaboration — envoyer"),
    ("collab:manage", "Collaboration — paramètres"),
];

/// Association member-role permissions (Excel VIREMENT BANQUAIRE split).
pub const MEMBER_ROLE_PERMISSION_OPTIONS: &[(&str, &str)] = &[
    ("member:can_pay", "Membre peut payer"),
    ("member:pay_bank_transfer", "Paie par virement bancaire"),
    ("member:pay_cash", "Paie en espèces"),
];

pub fn normalize_permissions(selected: &[String]) -> Vec<String> {
    if selected.iter().any(|p| p == "*") {
        return vec!["*".into()];
    }
    selected
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect()
}

pub fn parse_permissions_list(raw: &str) -> Vec<String> {
    let t = raw.trim();
    if t.is_empty() {
        return Vec::new();
    }
    if t.starts_with('[') {
        t.trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        t.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

pub fn member_permission_label(key: &str) -> String {
    MEMBER_ROLE_PERMISSION_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| key.to_string())
}

#[allow(dead_code)]
pub fn permission_label(key: &str) -> String {
    PERMISSION_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| key.to_string())
}

/// `true` when `perms` grants `need`.
/// - `*` grants everything
/// - `resource:write` also grants `resource:read`
pub fn permissions_allow(perms: &[String], need: &str) -> bool {
    if need.is_empty() {
        return true;
    }
    if perms.iter().any(|p| p == "*") {
        return true;
    }
    if perms.iter().any(|p| p == need) {
        return true;
    }
    if let Some(resource) = need.strip_suffix(":read") {
        let write = format!("{resource}:write");
        if perms.iter().any(|p| p == &write) {
            return true;
        }
    }
    false
}

pub fn permissions_allow_any(perms: &[String], needs: &[&str]) -> bool {
    needs.iter().any(|n| permissions_allow(perms, n))
}

/// Whether the user may open a nav route (read OR write for that area).
pub fn can_see_nav(perms: &[String], href: &str) -> bool {
    match href {
        "/app" => permissions_allow_any(perms, &["analytics:read", "cotisations:read", "members:read"]),
        "/app/staff" => {
            permissions_allow_any(perms, &["staff:read", "staff:write", "roles:read", "roles:write"])
        }
        "/app/planning" => {
            permissions_allow_any(perms, &["planning:read", "planning:write"])
        }
        "/app/members" => permissions_allow_any(perms, &["members:read", "members:write"]),
        "/app/cotisations" => {
            permissions_allow_any(perms, &["cotisations:read", "cotisations:write"])
        }
        "/app/profile" => true,
        "/app/settings" => permissions_allow_any(perms, &["settings:read", "settings:write"]),
        "/app/collaboration" => true, // connect/sync available to all; push gated separately
        _ => true,
    }
}
