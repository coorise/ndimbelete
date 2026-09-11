//! PostgreSQL client (URI-agnostic) + remote collaboration schema.

use anyhow::{anyhow, Context, Result};
use native_tls::TlsConnector;
use postgres::{Client, NoTls};
use postgres_native_tls::MakeTlsConnector;
use serde::Serialize;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS collab_commits (
    id TEXT PRIMARY KEY,
    message TEXT NOT NULL,
    author_name TEXT,
    author_staff_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    parent_id TEXT,
    snapshot BYTEA NOT NULL
);

CREATE TABLE IF NOT EXISTS collab_head (
    id INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    commit_id TEXT REFERENCES collab_commits(id)
);

INSERT INTO collab_head (id, commit_id)
VALUES (1, NULL)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS collab_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProbe {
    pub ok: bool,
    pub has_head: bool,
    pub head_commit_id: Option<String>,
    pub commit_count: i64,
    pub message: String,
}

/// Open a blocking Postgres client. Supports `sslmode=require` (Aiven, etc.).
pub fn open_remote(uri: &str) -> Result<Client> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Err(anyhow!("URI PostgreSQL vide"));
    }

    let wants_tls = uri.contains("sslmode=require")
        || uri.contains("sslmode=verify-ca")
        || uri.contains("sslmode=verify-full")
        || uri.contains("ssl=true");

    if wants_tls {
        connect_tls(uri)
    } else {
        // Try plain first; if server requires SSL, retry with TLS.
        match Client::connect(uri, NoTls) {
            Ok(c) => Ok(c),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("ssl") || msg.contains("tls") {
                    connect_tls(uri)
                } else {
                    Err(e).context(
                        "Connexion PostgreSQL échouée. Vérifiez l'URI et que le service est réveillé.",
                    )
                }
            }
        }
    }
}

fn connect_tls(uri: &str) -> Result<Client> {
    let connector = TlsConnector::builder()
        .build()
        .context("Échec de la configuration TLS")?;
    let tls = MakeTlsConnector::new(connector);
    match Client::connect(uri, tls) {
        Ok(c) => Ok(c),
        Err(first) => {
            // Some hosts (e.g. Aiven) ship a custom CA; fall back so free-tier works
            // without requiring a downloaded certificate file.
            let connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .context("Échec de la configuration TLS")?;
            let tls = MakeTlsConnector::new(connector);
            Client::connect(uri, tls).with_context(|| {
                format!(
                    "Connexion PostgreSQL échouée (TLS). Service peut être en veille (~20s–1min). Détail: {first}"
                )
            })
        }
    }
}

pub fn ensure_remote_schema(client: &mut Client) -> Result<()> {
    client
        .batch_execute(SCHEMA_SQL)
        .context("Échec de l'initialisation du schéma distant")?;
    Ok(())
}

pub fn probe_remote(uri: &str) -> Result<RemoteProbe> {
    let mut client = open_remote(uri)?;
    ensure_remote_schema(&mut client)?;

    let head: Option<String> = client
        .query_opt("SELECT commit_id FROM collab_head WHERE id = 1", &[])?
        .and_then(|row| row.get(0));

    let count: i64 = client.query_one("SELECT COUNT(*)::bigint FROM collab_commits", &[])?.get(0);

    Ok(RemoteProbe {
        ok: true,
        has_head: head.is_some(),
        head_commit_id: head,
        commit_count: count,
        message: if count == 0 {
            "Base distante vide — prêt pour le premier envoi.".into()
        } else {
            format!("{count} version(s) distante(s).")
        },
    })
}
