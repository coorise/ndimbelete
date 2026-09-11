//! Collaboration: local SQLite ↔ remote PostgreSQL (URI-agnostic).
//!
//! Remote stores versioned gzipped SQLite snapshots (Git-like commits).
//! Push/pull overwrite the other side; rollback restores a prior commit.

mod config;
mod listen;
mod remote;
mod snapshot;
mod sync;

pub use config::{default_database_url, load_config, save_config};
pub use listen::{set_app_handle, start_listener};
pub use remote::{probe_remote, RemoteProbe};
pub use sync::{
    cleanup_old_commits, clear_remote_database, connect_and_pull_if_needed, connect_remote,
    default_uri_for_ui, disconnect, list_commits, mask_uri, pull_head, push_snapshot,
    rollback_to_commit, status, CollabCommitInfo, CollabConnectResult, CollabStatus, PushResult,
};

#[cfg(test)]
#[path = "live_test.rs"]
mod live_test;
