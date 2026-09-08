//! Desktop printer listing + receipt printing with fixed page presets.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::models::AppSettings;
use crate::services::receipt_pdf::{
    list_system_printers, print_receipt_native, write_receipt_pdf, PrinterInfo, ReceiptPrintPayload,
};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintReceiptInput {
    pub printer_name: String,
    pub receipt: ReceiptPrintPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintReceiptResult {
    pub pdf_path: String,
    pub format: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub printer_name: String,
}

fn page_dims(format: &str, mini_w: f64) -> (f64, f64) {
    if format.eq_ignore_ascii_case("mini") {
        (mini_w.max(1.0), 80.0)
    } else {
        (210.0, 297.0)
    }
}

fn temp_receipt_path(format: &str) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let name = if format.eq_ignore_ascii_case("mini") {
        format!("recu-mini-{stamp}.pdf")
    } else {
        format!("recu-a4-{stamp}.pdf")
    };
    std::env::temp_dir().join(name)
}

fn load_settings(state: &State<'_, AppState>) -> Result<AppSettings, String> {
    crate::commands::settings::get_settings(state.clone())
}

#[tauri::command]
pub fn list_printers() -> Result<Vec<PrinterInfo>, String> {
    list_system_printers()
}

#[tauri::command]
pub fn print_receipt(
    state: State<'_, AppState>,
    input: PrintReceiptInput,
) -> Result<PrintReceiptResult, String> {
    if input.printer_name.trim().is_empty() {
        return Err("Sélectionnez une imprimante.".into());
    }
    let settings = load_settings(&state)?;
    let format = input.receipt.format.clone();
    let path = temp_receipt_path(&format);
    let _ = write_receipt_pdf(&input.receipt, &settings, &path);
    print_receipt_native(input.printer_name.trim(), &input.receipt, &settings)?;

    let (w, h) = page_dims(&format, settings.receipt_mini_width_mm);
    Ok(PrintReceiptResult {
        pdf_path: path.to_string_lossy().into_owned(),
        format,
        page_width_mm: w,
        page_height_mm: h,
        printer_name: input.printer_name,
    })
}

#[tauri::command]
pub fn export_receipt_pdf(
    state: State<'_, AppState>,
    receipt: ReceiptPrintPayload,
    path: String,
) -> Result<PrintReceiptResult, String> {
    let settings = load_settings(&state)?;
    let format = receipt.format.clone();
    let out = PathBuf::from(&path);
    write_receipt_pdf(&receipt, &settings, &out)?;
    let (w, h) = page_dims(&format, settings.receipt_mini_width_mm);
    Ok(PrintReceiptResult {
        pdf_path: out.to_string_lossy().into_owned(),
        format,
        page_width_mm: w,
        page_height_mm: h,
        printer_name: String::new(),
    })
}
