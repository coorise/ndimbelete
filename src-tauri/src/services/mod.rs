//! Business logic services — auth, cotisation engine, Excel import/export.

pub mod auth_service;
pub mod collab;
pub mod cotisation_engine;
pub mod excel_export;
pub mod excel_import;
pub mod receipt_pdf;
pub mod receipt_template;

#[cfg(windows)]
pub mod win_gdi_print;
