//! Tauri command handlers — thin wrappers around services / SQL.
//!
//! Each command returns `Result<T, String>` so the frontend gets a clear error message.

mod analytics;
mod auth;
mod backup;
mod cotisations;
mod dialogs;
mod excel;
mod member_roles;
mod members;
mod planning;
mod print;
mod roles;
mod settings;
mod staff;
mod updates;

pub use analytics::*;
pub use auth::*;
pub use backup::*;
pub use cotisations::*;
pub use dialogs::*;
pub use excel::*;
pub use member_roles::*;
pub use members::*;
pub use planning::*;
pub use print::*;
pub use roles::*;
pub use settings::*;
pub use staff::*;
pub use updates::*;
