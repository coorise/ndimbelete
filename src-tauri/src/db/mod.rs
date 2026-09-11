//! Database layer — connection, migrations, and first-run seed.

mod activity;
mod connection;
mod member_helpers;
mod migrations;
mod session;

pub use activity::{
    ensure_activity_table, list_activities, log_activity, resolve_actor_name, suggest_push_message,
    ActivityEntry, AREA_COTISATIONS, AREA_MEMBERS, AREA_PLANNING, AREA_SETTINGS, AREA_STAFF,
};
pub use connection::{database_path, open_database};
pub use member_helpers::{
    get_member_by_id, map_member, map_virement_cell, resolve_payment_and_virement, MEMBER_SELECT,
};
pub use migrations::{run_migrations, seed_year};
pub use session::{clear_session_file, load_session_staff_id, save_session_staff_id};
