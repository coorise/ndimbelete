//! App update checks against GitHub Releases (main = prod, dev = prerelease).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_OWNER: &str = "coorise";
const GITHUB_REPO: &str = "ndimbelete";

/// `main` = production releases · `dev` = prerelease / `dev-*` tags.
fn update_channel() -> &'static str {
    option_env!("NDIMBELENTE_UPDATE_CHANNEL").unwrap_or("main")
}

fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub channel: String,
    pub release_name: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub body: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    content_type: Option<String>,
}

fn normalize_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('v')
        .trim_start_matches("V")
        .trim_start_matches("dev-")
        .trim_start_matches("DEV-")
        .to_string()
}

fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let n = normalize_version(v);
    let mut parts = n.split(|c| c == '-' || c == '+');
    let core = parts.next()?;
    let mut nums = core.split('.');
    let major = nums.next()?.parse().ok()?;
    let minor = nums.next().unwrap_or("0").parse().ok()?;
    let patch = nums.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => normalize_version(latest) != normalize_version(current)
            && !normalize_version(latest).is_empty(),
    }
}

fn pick_windows_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        (n.ends_with(".exe") || n.ends_with(".msi"))
            && !n.contains("setup.exe.sig")
            && !n.ends_with(".sig")
    })
}

fn release_matches_channel(rel: &GhRelease, channel: &str) -> bool {
    if rel.draft {
        return false;
    }
    let tag = rel.tag_name.to_lowercase();
    match channel {
        "dev" => rel.prerelease || tag.starts_with("dev-") || tag.contains("-dev"),
        _ => !rel.prerelease && !tag.starts_with("dev-"),
    }
}

fn fetch_releases() -> Result<Vec<GhRelease>, String> {
    let url = format!(
        "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases?per_page=20"
    );
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("ndimbelente-updater/{}", app_version()))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("Réseau: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "GitHub API {} — {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }
    resp.json().map_err(|e| format!("JSON releases: {e}"))
}

#[tauri::command]
pub fn get_app_version_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "version": app_version(),
        "channel": update_channel(),
    }))
}

#[tauri::command]
pub fn check_for_update() -> Result<UpdateInfo, String> {
    let channel = update_channel().to_string();
    let current = app_version();
    let releases = fetch_releases()?;
    let latest = releases
        .into_iter()
        .find(|r| release_matches_channel(r, &channel));

    let Some(rel) = latest else {
        return Ok(UpdateInfo {
            available: false,
            current_version: current.clone(),
            latest_version: current.clone(),
            channel,
            release_name: String::new(),
            release_url: format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases"),
            download_url: None,
            asset_name: None,
            body: String::new(),
            message: "Aucune publication trouvée pour ce canal.".into(),
        });
    };

    let latest_ver = normalize_version(&rel.tag_name);
    let asset = pick_windows_asset(&rel.assets);
    let available = is_newer(&latest_ver, &current) && asset.is_some();

    let message = if available {
        format!("Nouvelle version {latest_ver} disponible (actuelle : {current}).")
    } else if asset.is_none() && is_newer(&latest_ver, &current) {
        format!(
            "Version {latest_ver} publiée, mais aucun installateur Windows n’est encore disponible."
        )
    } else {
        format!("Vous êtes à jour ({current}).")
    };

    Ok(UpdateInfo {
        available,
        current_version: current,
        latest_version: latest_ver,
        channel,
        release_name: rel.name.unwrap_or_else(|| rel.tag_name.clone()),
        release_url: rel.html_url,
        download_url: asset.map(|a| a.browser_download_url.clone()),
        asset_name: asset.map(|a| a.name.clone()),
        body: rel.body.unwrap_or_default(),
        message,
    })
}

fn download_file(url: &str, dest: &PathBuf) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("ndimbelente-updater/{}", app_version()))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Téléchargement: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement HTTP {}", resp.status()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut buf: Vec<u8> = Vec::new();
    resp.copy_to(&mut buf)
        .map_err(|e| format!("Lecture asset: {e}"))?;
    file.write_all(&buf).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    let info = check_for_update()?;
    if !info.available {
        return Err("Aucune mise à jour à installer.".into());
    }
    let url = info
        .download_url
        .ok_or_else(|| "URL de téléchargement manquante.".to_string())?;
    let name = info
        .asset_name
        .unwrap_or_else(|| "ndimbelente-setup.exe".into());

    let dest = std::env::temp_dir().join(format!("ndimbelente-update-{name}"));
    download_file(&url, &dest)?;

    let path_str = dest
        .to_str()
        .ok_or_else(|| "Chemin installateur invalide.".to_string())?
        .replace('\'', "''");

    // Wait for silent install, then delete the setup to free disk space.
    let ps = format!(
        "Start-Process -FilePath '{path_str}' -ArgumentList '/S' -Wait; \
         Start-Sleep -Seconds 2; \
         Remove-Item -LiteralPath '{path_str}' -Force -ErrorAction SilentlyContinue"
    );

    Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &ps,
        ])
        .spawn()
        .map_err(|e| format!("Lancement installateur: {e}"))?;

    // Quit so the installer can overwrite running files.
    app.exit(0);
    Ok(())
}
