//! Smoke test: connect + push + list against DATABASE_URL from .env
//!
//!   cargo run -p ndimbelente --example collab_smoke

fn main() {
    let uri = ndimbelente_lib::collab::default_database_url()
        .expect("DATABASE_URL missing — create .env at repo root");
    println!("URI (masked): {}", ndimbelente_lib::collab::mask_uri(&uri));

    let probe = ndimbelente_lib::collab::probe_remote(&uri).expect("probe");
    println!("probe: {} (commits={})", probe.message, probe.commit_count);

    let mut conn = ndimbelente_lib::open_database().expect("open db");
    ndimbelente_lib::run_migrations(&conn).expect("migrate");
    ndimbelente_lib::collab::connect_and_pull_if_needed(&mut conn, &uri, false).expect("connect");

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into());
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('collab_smoke', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [&stamp],
    )
    .expect("marker");

    let push = ndimbelente_lib::collab::push_snapshot(
        &conn,
        "smoke: collab push from example",
        Some("Smoke".into()),
        None,
    )
    .expect("push");
    println!("pushed {}", push.commit_id);

    let commits = ndimbelente_lib::collab::list_commits().expect("list");
    println!("remote versions: {}", commits.len());
    for c in commits.iter().take(5) {
        println!(
            "  {} {} {}",
            if c.is_head { "HEAD" } else { "   " },
            &c.id[..8.min(c.id.len())],
            c.message
        );
    }
    println!("OK");
}
