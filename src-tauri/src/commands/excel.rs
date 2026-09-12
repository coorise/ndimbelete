//! Excel import / export / preview commands.

use tauri::State;

use crate::services::excel_export::{self, ExportResult};
use crate::services::excel_import::{self, ExcelPreview, ImportResult};
use crate::state::AppState;

#[tauri::command]
pub fn preview_excel(
    path: String,
    sheet_name: Option<String>,
) -> Result<ExcelPreview, String> {
    excel_import::preview_excel(&path, sheet_name)
}

#[tauri::command]
pub fn import_excel(
    state: State<'_, AppState>,
    path: String,
    sheet_name: String,
    year: i32,
    role_values: Option<Vec<String>>,
) -> Result<ImportResult, String> {
    let conn = state.db.lock();
    let roles = role_values.unwrap_or_else(|| {
        crate::models::DEFAULT_VIREMENT_ROLE_VALUES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    });
    excel_import::import_excel(&conn, &path, &sheet_name, year, &roles)
}

#[tauri::command]
pub fn import_excel_grid(
    state: State<'_, AppState>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    year: i32,
    sheet_label: Option<String>,
    role_values: Option<Vec<String>>,
    prune_missing: Option<bool>,
    monthly_amount: Option<f64>,
) -> Result<ImportResult, String> {
    let conn = state.db.lock();
    let roles = role_values.unwrap_or_else(|| {
        crate::models::DEFAULT_VIREMENT_ROLE_VALUES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    });
    let label = sheet_label.unwrap_or_default();
    let prune = prune_missing.unwrap_or(true);
    excel_import::import_excel_grid(
        &conn,
        &headers,
        &rows,
        year,
        &label,
        &roles,
        prune,
        monthly_amount,
    )
}

#[tauri::command]
pub fn export_excel(
    state: State<'_, AppState>,
    path: String,
    year: i32,
) -> Result<ExportResult, String> {
    let conn = state.db.lock();
    excel_export::export_excel(&conn, &path, year)
}
