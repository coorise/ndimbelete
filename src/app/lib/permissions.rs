//! Canonical staff permission keys shown in the roles UI.

/// (key, French label)
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
    ("staff:read", "Personnel — lecture"),
    ("staff:write", "Personnel — écriture"),
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
