//! Native Windows printing: ESC/POS RAW for Mini(POS), GDI for A4.
//! RAW avoids thermal ghosting caused by TrueType antialiasing via GDI.

#![cfg(windows)]

use windows::core::{PSTR, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HANDLE};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteDC, DeleteObject, GetDeviceCaps, SelectObject, SetBkMode, SetTextColor,
    TextOutW, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    HDC, LOGPIXELSY, NONANTIALIASED_QUALITY, OUT_TT_PRECIS, OPAQUE,
};
use windows::Win32::Graphics::Printing::{
    ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterA, StartPagePrinter,
    WritePrinter, DOC_INFO_1A,
};
use windows::Win32::Storage::Xps::{AbortDoc, EndDoc, EndPage, StartDocW, StartPage, DOCINFOW};

use super::receipt_pdf::ReceiptPrintPayload;
use super::receipt_template::{to_thermal_ascii, RenderedLine};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wrap_thermal(text: &str, cols: usize) -> Vec<String> {
    let ascii = to_thermal_ascii(text);
    if ascii.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = ascii.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars.len() - i <= cols {
            out.push(chars[i..].iter().collect());
            break;
        }
        let hard = i + cols;
        let mut end = hard;
        for j in (i + cols / 3..hard).rev() {
            if chars[j] == ' ' {
                end = j;
                break;
            }
        }
        out.push(chars[i..end].iter().collect::<String>().trim_end().to_string());
        i = if end < hard { end + 1 } else { end };
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn build_escpos(lines: &[RenderedLine], cols: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    // Initialize printer — do not send cut; some drivers echo garbage (@P) with GS V.
    buf.extend_from_slice(&[0x1B, 0x40]);
    // Select Font A, left align, denser line spacing
    buf.extend_from_slice(&[0x1B, 0x4D, 0x00]); // ESC M 0 font A
    buf.extend_from_slice(&[0x1B, 0x33, 0x28]); // ESC 3 n line spacing (~3.1mm)

    for line in lines {
        let align = match line.align.as_str() {
            "center" => 1u8,
            "right" => 2u8,
            _ => 0u8,
        };
        buf.extend_from_slice(&[0x1B, 0x61, align]); // ESC a n

        if line.text.is_empty() {
            buf.push(b'\n');
            continue;
        }
        if line.bold {
            buf.extend_from_slice(&[0x1B, 0x45, 0x01]);
        }
        // Avoid double-height (causes ghosting on many 58mm heads)
        for wrapped in wrap_thermal(&line.text, cols) {
            buf.extend_from_slice(wrapped.as_bytes());
            buf.push(b'\n');
        }
        if line.bold {
            buf.extend_from_slice(&[0x1B, 0x45, 0x00]);
        }
    }
    buf.extend_from_slice(b"\n\n\n");
    buf
}

fn print_raw_bytes(printer_name: &str, data: &[u8]) -> Result<(), String> {
    unsafe {
        let name_w = to_wide(printer_name);
        let mut handle = HANDLE::default();
        OpenPrinterW(PCWSTR(name_w.as_ptr()), &mut handle, None)
            .map_err(|e| format!("OpenPrinter « {printer_name} »: {e}"))?;

        let doc_name = std::ffi::CString::new("Recu NDIMBELENTE").unwrap();
        let datatype = std::ffi::CString::new("RAW").unwrap();
        let mut doc_info = DOC_INFO_1A {
            pDocName: PSTR(doc_name.as_ptr() as *mut u8),
            pOutputFile: PSTR::null(),
            pDatatype: PSTR(datatype.as_ptr() as *mut u8),
        };

        let job_id = StartDocPrinterA(handle, 1, &mut doc_info);
        if job_id == 0 {
            let _ = ClosePrinter(handle);
            return Err("StartDocPrinter a échoué (RAW).".into());
        }
        if StartPagePrinter(handle) == false {
            let _ = EndDocPrinter(handle);
            let _ = ClosePrinter(handle);
            return Err("StartPagePrinter a échoué.".into());
        }

        let mut written = 0u32;
        let ok = WritePrinter(
            handle,
            data.as_ptr() as *const core::ffi::c_void,
            data.len() as u32,
            &mut written,
        );
        let _ = EndPagePrinter(handle);
        let _ = EndDocPrinter(handle);
        let _ = ClosePrinter(handle);

        if !ok.as_bool() || written == 0 {
            return Err("WritePrinter a échoué (aucune donnée envoyée).".into());
        }
        Ok(())
    }
}

fn print_gdi_a4(printer_name: &str, lines: &[RenderedLine]) -> Result<(), String> {
    unsafe {
        let printer_w = to_wide(printer_name);
        let driver = to_wide("WINSPOOL");
        let hdc: HDC = windows::Win32::Graphics::Gdi::CreateDCW(
            PCWSTR(driver.as_ptr()),
            PCWSTR(printer_w.as_ptr()),
            PCWSTR::null(),
            None,
        );
        if hdc.is_invalid() {
            return Err(format!(
                "Impossible d'ouvrir l'imprimante « {printer_name} » (CreateDC)."
            ));
        }

        let doc_name = to_wide("Recu cotisation NDIMBELENTE");
        let di = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: PCWSTR(doc_name.as_ptr()),
            lpszOutput: PCWSTR::null(),
            lpszDatatype: PCWSTR::null(),
            fwType: 0,
        };

        if StartDocW(hdc, &di) <= 0 {
            let _ = DeleteDC(hdc);
            return Err("StartDoc a échoué.".into());
        }
        if StartPage(hdc) <= 0 {
            let _ = AbortDoc(hdc);
            let _ = DeleteDC(hdc);
            return Err("StartPage a échoué.".into());
        }

        let dpi_y = GetDeviceCaps(hdc, LOGPIXELSY).max(72);
        let mm_to_px = |mm: i32| -> i32 { (mm * dpi_y) / 25 };
        let left = mm_to_px(15);
        let mut y = mm_to_px(15);
        let _ = SetBkMode(hdc, OPAQUE);
        let _ = SetTextColor(hdc, COLORREF(0));

        for line in lines {
            if line.text.is_empty() {
                y += mm_to_px(3);
                continue;
            }
            let pt = line.font_size.max(9.0) as i32;
            let height = -((pt * dpi_y) / 72);
            let weight = if line.bold {
                FW_BOLD.0 as i32
            } else {
                FW_NORMAL.0 as i32
            };
            let italic = if line.italic { 1u32 } else { 0u32 };
            let face = to_wide("Arial");
            let font = CreateFontW(
                height,
                0,
                0,
                0,
                weight,
                italic,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_TT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                NONANTIALIASED_QUALITY.0 as u32,
                DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
                PCWSTR(face.as_ptr()),
            );
            if font.is_invalid() {
                continue;
            }
            let old = SelectObject(hdc, font);
            let wide: Vec<u16> = line.text.encode_utf16().collect();
            let _ = TextOutW(hdc, left, y, &wide);
            let _ = SelectObject(hdc, old);
            let _ = DeleteObject(font);
            y += mm_to_px(7).max((pt * dpi_y) / 72 + dpi_y / 24);
        }

        if EndPage(hdc) <= 0 {
            let _ = AbortDoc(hdc);
            let _ = DeleteDC(hdc);
            return Err("EndPage a échoué.".into());
        }
        if EndDoc(hdc) <= 0 {
            let _ = DeleteDC(hdc);
            return Err("EndDoc a échoué.".into());
        }
        let _ = DeleteDC(hdc);
    }
    Ok(())
}

/// Print rendered receipt lines to the named printer.
pub fn print_receipt_lines(
    printer_name: &str,
    payload: &ReceiptPrintPayload,
    lines: &[RenderedLine],
) -> Result<(), String> {
    if payload.format.eq_ignore_ascii_case("mini") {
        let data = build_escpos(lines, 32);
        print_raw_bytes(printer_name, &data)
    } else {
        print_gdi_a4(printer_name, lines)
    }
}
