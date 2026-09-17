fn main() {
    tauri_build::build();

    // Rebuild when CI / local env for the default remote URI changes.
    println!("cargo:rerun-if-env-changed=NDIMBELENTE_BAKED_DATABASE_URL");
    println!("cargo:rerun-if-env-changed=DATABASE_URL");

    // Prefer an explicit bake var (set from GitHub secrets in workflows).
    // Fall back to DATABASE_URL so local `cargo tauri build` can also embed `.env`.
    let baked = std::env::var("NDIMBELENTE_BAKED_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(url) = baked {
        // Single-line only — rustc-env cannot contain newlines.
        let escaped = url.replace(['\n', '\r'], "");
        println!("cargo:rustc-env=NDIMBELENTE_BAKED_DATABASE_URL={escaped}");
    }
}
