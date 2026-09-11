//! URI display helpers (never show DB passwords in the UI).

/// Hide the password segment of a Postgres URI: `user:***@host`.
pub fn mask_postgres_uri(uri: &str) -> String {
    let uri = uri.trim();
    if let Some(scheme_end) = uri.find("://") {
        let rest = &uri[scheme_end + 3..];
        if let Some(at) = rest.find('@') {
            let creds = &rest[..at];
            let host = &rest[at..];
            if let Some(colon) = creds.find(':') {
                let user = &creds[..colon];
                return format!("{}{}:***{}", &uri[..scheme_end + 3], user, host);
            }
        }
    }
    uri.to_string()
}
