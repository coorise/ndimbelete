//! App settings commands.

use tauri::State;

use crate::models::{
    default_a4_fields, default_a4_template, default_mini_fields, default_mini_template,
    maybe_upgrade_short_template, AppSettings, ReceiptField,
};
use crate::services::receipt_template::fields_to_template;
use crate::state::AppState;

fn read_setting(conn: &rusqlite::Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

fn read_fields(conn: &rusqlite::Connection, key: &str, fallback: Vec<ReceiptField>) -> Vec<ReceiptField> {
    let raw = read_setting(conn, key, "");
    if raw.trim().is_empty() {
        return fallback;
    }
    serde_json::from_str(&raw).unwrap_or(fallback)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let conn = state.db.lock();
    let defaults = AppSettings::default();
    let tpl_a4_raw = read_setting(&conn, "receipt_template_a4", &default_a4_template());
    let tpl_mini_raw = read_setting(&conn, "receipt_template_mini", &default_mini_template());
    let tpl_a4 = maybe_upgrade_short_template(&tpl_a4_raw, &default_a4_template());
    let tpl_mini = maybe_upgrade_short_template(&tpl_mini_raw, &default_mini_template());
    let mut fields_a4 = read_fields(&conn, "receipt_fields_a4", default_a4_fields());
    let mut fields_mini = read_fields(&conn, "receipt_fields_mini", default_mini_fields());
    if !fields_a4.iter().any(|f| f.content.to_lowercase().contains("membre")) {
        fields_a4 = default_a4_fields();
    }
    if !fields_mini
        .iter()
        .any(|f| f.content.to_lowercase().contains("membre"))
    {
        fields_mini = default_mini_fields();
    }
    let tpl_a4_upgraded = tpl_a4 != tpl_a4_raw;
    let tpl_mini_upgraded = tpl_mini != tpl_mini_raw;
    if tpl_a4_upgraded || tpl_mini_upgraded
        || !fields_a4.iter().any(|f| f.content.to_lowercase().contains("membre"))
        || !fields_mini
            .iter()
            .any(|f| f.content.to_lowercase().contains("membre"))
    {
        let _ = conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params!["receipt_template_a4", &tpl_a4],
        );
        let _ = conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params!["receipt_template_mini", &tpl_mini],
        );
        if let Ok(fa) = serde_json::to_string(&fields_a4) {
            let _ = conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params!["receipt_fields_a4", fa],
            );
        }
        if let Ok(fm) = serde_json::to_string(&fields_mini) {
            let _ = conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params!["receipt_fields_mini", fm],
            );
        }
    }
    Ok(AppSettings {
        org_name: read_setting(&conn, "org_name", &defaults.org_name),
        org_description: read_setting(&conn, "org_description", &defaults.org_description),
        org_address: read_setting(&conn, "org_address", &defaults.org_address),
        theme_color: read_setting(&conn, "theme_color", &defaults.theme_color),
        font_scale: read_setting(&conn, "font_scale", "1.0")
            .parse()
            .unwrap_or(1.0),
        logo_path: read_setting(&conn, "logo_path", ""),
        currency_unit: read_setting(&conn, "currency_unit", &defaults.currency_unit),
        receipt_template_a4: tpl_a4,
        receipt_template_mini: tpl_mini,
        receipt_mini_width_mm: read_setting(&conn, "receipt_mini_width_mm", "58")
            .parse()
            .unwrap_or(58.0),
        receipt_editor_mode_a4: read_setting(&conn, "receipt_editor_mode_a4", "template"),
        receipt_editor_mode_mini: read_setting(&conn, "receipt_editor_mode_mini", "template"),
        receipt_fields_a4: fields_a4,
        receipt_fields_mini: fields_mini,
    })
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<AppSettings, String> {
    // Keep template text in sync when editing via fields.
    if settings.receipt_editor_mode_a4.eq_ignore_ascii_case("fields") {
        settings.receipt_template_a4 = fields_to_template(&settings.receipt_fields_a4);
    }
    if settings
        .receipt_editor_mode_mini
        .eq_ignore_ascii_case("fields")
    {
        settings.receipt_template_mini = fields_to_template(&settings.receipt_fields_mini);
    }

    let conn = state.db.lock();
    let font_scale_str = settings.font_scale.to_string();
    let mini_w = settings.receipt_mini_width_mm.to_string();
    let fields_a4 =
        serde_json::to_string(&settings.receipt_fields_a4).unwrap_or_else(|_| "[]".into());
    let fields_mini =
        serde_json::to_string(&settings.receipt_fields_mini).unwrap_or_else(|_| "[]".into());

    let pairs = [
        ("org_name", settings.org_name.as_str()),
        ("org_description", settings.org_description.as_str()),
        ("org_address", settings.org_address.as_str()),
        ("theme_color", settings.theme_color.as_str()),
        ("font_scale", font_scale_str.as_str()),
        ("logo_path", settings.logo_path.as_str()),
        ("currency_unit", settings.currency_unit.as_str()),
        ("receipt_template_a4", settings.receipt_template_a4.as_str()),
        (
            "receipt_template_mini",
            settings.receipt_template_mini.as_str(),
        ),
        ("receipt_mini_width_mm", mini_w.as_str()),
        (
            "receipt_editor_mode_a4",
            settings.receipt_editor_mode_a4.as_str(),
        ),
        (
            "receipt_editor_mode_mini",
            settings.receipt_editor_mode_mini.as_str(),
        ),
        ("receipt_fields_a4", fields_a4.as_str()),
        ("receipt_fields_mini", fields_mini.as_str()),
    ];

    for (k, v) in pairs {
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![k, v],
        )
        .map_err(|e| e.to_string())?;
    }

    crate::commands::note(
        &state,
        &conn,
        crate::db::AREA_SETTINGS,
        "update",
        "Mise à jour des paramètres généraux",
    );

    drop(conn);
    get_settings(state)
}
