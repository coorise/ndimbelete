//! Database layer — connection, migrations, and first-run seed.

mod connection;
mod member_helpers;
mod migrations;
mod session;

pub use connection::{database_path, open_database};
pub use member_helpers::{
    get_member_by_id, map_member, map_virement_cell, resolve_payment_and_virement, MEMBER_SELECT,
};
pub use migrations::{run_migrations, seed_year};
pub use session::{clear_session_file, load_session_staff_id, save_session_staff_id};
