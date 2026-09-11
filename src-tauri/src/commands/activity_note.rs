//! Best-effort activity notes for management mutations.

use rusqlite::Connection;

use crate::db::{self, resolve_actor_name};
use crate::state::AppState;

pub(crate) fn note(
    state: &AppState,
    conn: &Connection,
    area: &str,
    action: &str,
    summary: impl AsRef<str>,
) {
    let staff_id = state.session_staff_id.lock().clone();
    let name = staff_id
        .as_deref()
        .and_then(|id| resolve_actor_name(conn, id));
    let _ = db::log_activity(
        conn,
        staff_id.as_deref(),
        name.as_deref(),
        area,
        action,
        summary.as_ref(),
    );
}
