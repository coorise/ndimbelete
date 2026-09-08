//! Build fixed-size receipt PDFs (A4 or Mini) for archive / save.

use printpdf::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use crate::models::AppSettings;
use super::receipt_template::{resolve_lines, ReceiptContext, RenderedLine};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPrintPayload {
    pub org_name: String,
    #[serde(default)]
    pub org_address: String,
    pub member_name: String,
    pub card_number: String,
    #[serde(default)]
    pub member_uid: String,
    pub period_label: String,
    pub amount: f64,
    /// Debt before this payment (dette antérieure).
    #[serde(default)]
    pub debt_before: f64,
    pub balance_after: f64,
    /// Total paid for the year — {{NEW_BALANCE}} / Nouveau Solde.
    #[serde(default)]
    pub total_paid_year: f64,
    pub year: i32,
    /// `"a4"` or `"mini"`
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
}

fn find_font_path() -> Result<PathBuf, String> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let candidates = [
        PathBuf::from(&windir).join("Fonts").join("arial.ttf"),
        PathBuf::from(&windir).join("Fonts").join("calibri.ttf"),
        PathBuf::from(&windir).join("Fonts").join("segoeui.ttf"),
        PathBuf::from(&windir).join("Fonts").join("tahoma.ttf"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| "Aucune police système (Arial/Calibri) trouvée pour le PDF.".into())
}

pub fn build_receipt_lines(payload: &ReceiptPrintPayload, settings: &AppSettings) -> Vec<RenderedLine> {
    let is_mini = payload.format.eq_ignore_ascii_case("mini");
    let ctx = ReceiptContext::from_payload(payload, &settings.org_address);
    let default_size = if is_mini { 9.0 } else { 11.0 };
    let allow_color = !is_mini;
    if is_mini {
        resolve_lines(
            &settings.receipt_editor_mode_mini,
            &settings.receipt_template_mini,
            &settings.receipt_fields_mini,
            &ctx,
            default_size,
            allow_color,
        )
    } else {
        resolve_lines(
            &settings.receipt_editor_mode_a4,
            &settings.receipt_template_a4,
            &settings.receipt_fields_a4,
            &ctx,
            default_size,
            allow_color,
        )
    }
}

/// Create a PDF with forced page size and return its path.
pub fn write_receipt_pdf(
    payload: &ReceiptPrintPayload,
    settings: &AppSettings,
    out: &Path,
) -> Result<(), String> {
    let is_mini = payload.format.eq_ignore_ascii_case("mini");
    let lines = build_receipt_lines(payload, settings);
    let mini_w = if settings.receipt_mini_width_mm > 0.0 {
        settings.receipt_mini_width_mm as f32
    } else {
        58.0
    };

    let (page_w, page_h, margin, line_gap) = if is_mini {
        let content_h = 8.0_f32 + (lines.len() as f32) * 4.4 + 8.0;
        let h = content_h.clamp(40.0, 160.0);
        (Mm(mini_w), Mm(h), Mm(2.5), 4.4_f32)
    } else {
        (Mm(210.0), Mm(297.0), Mm(18.0), 6.5_f32)
    };

    let (doc, page_idx, layer_idx) =
        PdfDocument::new("Reçu de cotisation NDIMBELENTÉ", page_w, page_h, "Layer 1");

    let font_path = find_font_path()?;
    let font = doc
        .add_external_font(File::open(&font_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Police PDF: {e}"))?;
    let font_bold_path = {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let bold = PathBuf::from(&windir).join("Fonts").join("arialbd.ttf");
        if bold.exists() {
            bold
        } else {
            font_path.clone()
        }
    };
    let font_bold = doc
        .add_external_font(File::open(&font_bold_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Police PDF bold: {e}"))?;

    let layer = doc.get_page(page_idx).get_layer(layer_idx);
    let mut y = page_h - margin;

    for line in lines {
        if line.text.is_empty() {
            y -= Mm(line_gap * 0.45);
            continue;
        }
        let size = line.font_size.max(7.0) as f32;
        let f = if line.bold { &font_bold } else { &font };
        layer.use_text(line.text, size, margin, y - Mm(size * 0.35), f);
        y -= Mm(line_gap.max(size * 0.45));
        if y < margin {
            break;
        }
    }

    let note = if is_mini {
        format!("Format Mini(POS) · {mini_w} mm")
    } else {
        "Format A4 · 210 × 297 mm".into()
    };
    layer.use_text(note, 6.5, margin, Mm(3.5), &font);

    let file = File::create(out).map_err(|e| e.to_string())?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| format!("Écriture PDF: {e}"))?;
    Ok(())
}

/// List installed desktop printers (Windows).
pub fn list_system_printers() -> Result<Vec<PrinterInfo>, String> {
    #[cfg(windows)]
    {
        list_printers_windows()
    }
    #[cfg(not(windows))]
    {
        Err("Liste des imprimantes disponible uniquement sous Windows.".into())
    }
}

#[cfg(windows)]
fn list_printers_windows() -> Result<Vec<PrinterInfo>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[derive(Deserialize)]
    struct PsPrinter {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "Default")]
        default: Option<bool>,
    }

    let script = r#"
$ErrorActionPreference = 'Stop'
Get-CimInstance Win32_Printer | Select-Object Name,@{N='Default';E={$_.Default}} | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("PowerShell imprimantes: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Échec liste imprimantes: {err}"));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() || raw == "null" {
        return Ok(vec![]);
    }

    let list: Vec<PsPrinter> = if raw.starts_with('[') {
        serde_json::from_str(&raw).map_err(|e| format!("JSON imprimantes: {e}"))?
    } else {
        let one: PsPrinter = serde_json::from_str(&raw)
            .map_err(|e| format!("JSON imprimantes invalide: {e} / {raw}"))?;
        vec![one]
    };

    Ok(list
        .into_iter()
        .map(|p| PrinterInfo {
            name: p.name,
            is_default: p.default.unwrap_or(false),
        })
        .collect())
}

/// Send receipt to a named Windows printer.
pub fn print_receipt_native(
    printer_name: &str,
    payload: &ReceiptPrintPayload,
    settings: &AppSettings,
) -> Result<(), String> {
    let lines = build_receipt_lines(payload, settings);
    #[cfg(windows)]
    {
        crate::services::win_gdi_print::print_receipt_lines(printer_name, payload, &lines)
    }
    #[cfg(not(windows))]
    {
        let _ = (printer_name, payload, lines);
        Err("Impression native disponible uniquement sous Windows.".into())
    }
}
