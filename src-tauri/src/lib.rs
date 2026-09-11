//! Ndimbelente Tauri backend library.
//!
//! Wires plugins, SQLite AppState, and all invoke commands.
//! Frontend (Leptos CSR) talks to these via `invoke()`.

mod commands;
mod db;
mod models;
mod services;
mod state;

pub use services::collab;
pub use db::{open_database, run_migrations};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = services::collab::default_database_url(); // load optional .env early

    let conn = db::open_database().expect("Failed to open database");
    db::run_migrations(&conn).expect("Failed to run migrations");

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(debug_assertions)]
    let builder = builder.plugin(tauri_plugin_mcp_bridge::init());

    builder
        .manage(AppState::new(conn))
        .setup(|app| {
            services::collab::set_app_handle(app.handle().clone());
            let cfg = services::collab::load_config();
            if cfg.connected {
                if let Some(uri) = cfg.uri {
                    services::collab::start_listener(uri);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::needs_setup,
            commands::setup_admin,
            commands::login,
            commands::logout,
            commands::get_current_session,
            commands::change_password,
            commands::get_recovery_status,
            commands::reset_password_with_hints,
            commands::update_my_profile,
            // Staff
            commands::list_staff,
            commands::create_staff,
            commands::update_staff,
            commands::deactivate_staff,
            // Roles
            commands::list_roles,
            commands::create_role,
            commands::update_role,
            // Members
            commands::list_members,
            commands::get_member,
            commands::create_member,
            commands::update_member,
            commands::search_members,
            commands::delete_member,
            commands::delete_members,
            commands::list_member_roles,
            commands::create_member_role,
            commands::update_member_role,
            // Cotisations
            commands::get_year_grid,
            commands::set_monthly_amount,
            commands::record_payment,
            commands::clear_payment,
            commands::set_period_cell,
            commands::set_prior_december_debt,
            commands::get_member_debt,
            commands::get_member_debt_as_of,
            commands::list_years,
            commands::ensure_year,
            commands::recalculate_all_members,
            // Planning
            commands::list_planning,
            commands::upsert_planning_period,
            commands::delete_planning_period,
            // Settings
            commands::get_settings,
            commands::update_settings,
            // Analytics
            commands::get_overview_stats,
            // Excel
            commands::preview_excel,
            commands::import_excel,
            commands::import_excel_grid,
            commands::export_excel,
            // Dialogs
            commands::pick_excel_file,
            commands::pick_backup_file,
            commands::pick_save_file,
            // Backup
            commands::export_backup,
            commands::import_backup,
            commands::restart_app,
            // Updates
            commands::get_app_version_info,
            commands::check_for_update,
            commands::download_and_install_update,
            // Print
            commands::list_printers,
            commands::print_receipt,
            commands::export_receipt_pdf,
            // Collaboration
            commands::collab_get_default_uri,
            commands::collab_probe,
            commands::collab_status,
            commands::collab_connect,
            commands::collab_disconnect,
            commands::collab_push,
            commands::collab_pull,
            commands::collab_list_commits,
            commands::collab_list_activities,
            commands::collab_rollback,
            commands::collab_cleanup,
            commands::collab_set_keep_commits,
            commands::collab_set_push_acl,
            commands::collab_clear_remote,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
