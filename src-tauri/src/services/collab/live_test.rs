#[cfg(test)]
mod collab_live_tests {
    use crate::db::{open_database, run_migrations};
    use crate::services::collab;
    use rusqlite::Connection;
    use std::sync::Mutex;

    // Serialize live tests — they share remote HEAD.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn probe_aiven_from_env() {
        let _g = LOCK.lock().unwrap();
        let Some(uri) = collab::default_database_url() else {
            eprintln!("skip: no DATABASE_URL in .env");
            return;
        };
        let result = collab::probe_remote(&uri);
        match result {
            Ok(p) => {
                assert!(p.ok);
                eprintln!("probe ok: {}", p.message);
            }
            Err(e) => panic!("probe failed: {e:#}"),
        }
    }

    #[test]
    fn push_pull_roundtrip_against_aiven() {
        let _g = LOCK.lock().unwrap();
        let Some(uri) = collab::default_database_url() else {
            eprintln!("skip: no DATABASE_URL");
            return;
        };

        // Fresh in-memory-like temp local DB path isn't easy — use open_database then push.
        // Mark a distinctive setting so we can verify after "pull" of the same snapshot.
        let mut conn = open_database().expect("open local db");
        run_migrations(&conn).expect("migrate");
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('collab_test_marker', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            ["roundtrip-ok"],
        )
        .expect("marker");

        collab::connect_and_pull_if_needed(&mut conn, &uri, false).expect("connect");
        let push = collab::push_snapshot(
            &conn,
            "mcp-live-test roundtrip",
            Some("Auto Test".into()),
            None,
        )
        .expect("push");
        eprintln!("pushed commit {}", push.commit_id);

        // Clear marker locally, then pull head back.
        conn.execute("DELETE FROM app_settings WHERE key = 'collab_test_marker'", [])
            .ok();
        collab::pull_head(&mut conn).expect("pull");

        let marker: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'collab_test_marker'",
                [],
                |r| r.get(0),
            )
            .expect("marker after pull");
        assert_eq!(marker, "roundtrip-ok");
        eprintln!("pull restored marker OK");

        let commits = collab::list_commits().expect("list");
        assert!(!commits.is_empty());
        assert!(commits.iter().any(|c| c.id == push.commit_id && c.is_head));
        eprintln!("list_commits OK ({} versions)", commits.len());
    }
}
