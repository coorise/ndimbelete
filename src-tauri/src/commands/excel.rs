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
) -> Result<ImportResult, String> {
    let conn = state.db.lock();
    excel_import::import_excel(&conn, &path, &sheet_name, year)
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
