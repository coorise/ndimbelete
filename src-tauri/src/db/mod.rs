//! Database layer — connection, migrations, and first-run seed.

mod connection;
mod migrations;
mod session;

pub use connection::{database_path, open_database};
pub use migrations::{run_migrations, seed_year};
pub use session::{clear_session_file, load_session_staff_id, save_session_staff_id};
