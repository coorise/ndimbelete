//! Native file dialogs via `tauri_plugin_dialog` (more reliable than JS bindings).

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

fn file_path_to_string(path: tauri_plugin_dialog::FilePath) -> String {
    path.to_string()
}

/// Pick an Excel workbook (.xlsx / .xlsm / .xls).
#[tauri::command]
pub async fn pick_excel_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Excel", &["xlsx", "xlsm", "xls"])
        .blocking_pick_file();
    Ok(path.map(file_path_to_string))
}

/// Pick a backup file (.ndimbelente.bak / .bak / .db).
#[tauri::command]
pub async fn pick_backup_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Sauvegarde Ndimbelente", &["ndimbelente.bak", "bak", "db"])
        .blocking_pick_file();
    Ok(path.map(file_path_to_string))
}

/// Save-as dialog with a suggested file name.
#[tauri::command]
pub async fn pick_save_file(
    app: AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("Excel", &["xlsx"])
        .add_filter("Backup", &["ndimbelente.bak", "bak", "db"])
        .blocking_save_file();
    Ok(path.map(file_path_to_string))
}
