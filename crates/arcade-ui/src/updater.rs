//! Secure version checks and staged Windows desktop updates from GitHub Releases.

use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient, HttpRequestExt, Request, StatusCode};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const RELEASE_API: &str =
    "https://api.github.com/repos/indoctrinatedrecluse/ArcadeEdit/releases/latest";
const WINDOWS_DESKTOP_ASSET: &str = "arcade-desktop-windows-x86_64.exe";
const MAX_RELEASE_METADATA_BYTES: u64 = 2 * 1024 * 1024;
const MAX_UPDATE_BYTES: u64 = 150 * 1024 * 1024;

/// Release information for an update newer than the running application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateInfo {
    /// New release version.
    pub version: Version,
    /// Release page URL.
    pub release_url: String,
    /// Release notes in Markdown.
    pub notes: String,
    /// Direct download URL for the standalone Windows desktop executable.
    pub asset_url: String,
    /// SHA-256 digest reported by the GitHub release asset API.
    pub sha256: String,
    /// Expected size, when reported by GitHub.
    pub asset_size: u64,
}

/// UI-visible states for automatic and manual update checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateState {
    /// No request has started yet.
    Idle,
    /// Checking the latest published release.
    Checking,
    /// The running build is current.
    UpToDate,
    /// A newer Windows desktop release is available.
    Available(UpdateInfo),
    /// Downloading and verifying the update asset.
    Downloading(Version),
    /// The verified update is staged and the app is about to restart.
    Restarting(Version),
    /// The check or install failed with a user-readable message.
    Failed(String),
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}

/// Checks the current GitHub release and returns update information only if it is newer.
pub async fn check_for_update(
    client: &Arc<dyn HttpClient>,
    current_version: &str,
) -> Result<Option<UpdateInfo>, String> {
    let request = Request::get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("ArcadeEdit/{current_version}"))
        .timeout(Duration::from_secs(20))
        .body(AsyncBody::empty())
        .map_err(|error| format!("Could not create release check request: {error}"))?;

    let response = client
        .send(request)
        .await
        .map_err(|error| format!("Could not check GitHub Releases: {error}"))?;
    if response.status() != StatusCode::OK {
        return Err(format!(
            "GitHub Releases returned HTTP {}",
            response.status()
        ));
    }

    let mut bytes = Vec::new();
    response
        .into_body()
        .take(MAX_RELEASE_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("Could not read release metadata: {error}"))?;
    if bytes.len() as u64 > MAX_RELEASE_METADATA_BYTES {
        return Err("Release metadata exceeded the 2 MiB safety limit".into());
    }
    let release: ReleaseResponse = serde_json::from_slice(&bytes)
        .map_err(|error| format!("GitHub returned invalid release metadata: {error}"))?;
    if release.prerelease {
        return Ok(None);
    }

    let version_text = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let latest = Version::parse(version_text).map_err(|error| {
        format!(
            "Release tag {:?} is not a semantic version: {error}",
            release.tag_name
        )
    })?;
    let current = Version::parse(current_version)
        .map_err(|error| format!("Running version is invalid: {error}"))?;
    if latest <= current {
        return Ok(None);
    }

    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == WINDOWS_DESKTOP_ASSET)
        .ok_or_else(|| format!("Release {} has no Windows desktop update asset", latest))?;
    validate_asset_url(&asset.browser_download_url)?;
    if asset.size == 0 || asset.size > MAX_UPDATE_BYTES {
        return Err(format!(
            "Update asset size {} is outside the supported limit",
            asset.size
        ));
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "GitHub did not provide a valid SHA-256 digest for the update".to_string())?
        .to_ascii_lowercase();

    Ok(Some(UpdateInfo {
        version: latest,
        release_url: release.html_url,
        notes: release.body,
        asset_url: asset.browser_download_url,
        sha256,
        asset_size: asset.size,
    }))
}

fn validate_asset_url(url: &str) -> Result<(), String> {
    let parsed = gpui::http_client::Url::parse(url)
        .map_err(|error| format!("Release asset URL is invalid: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed
            .path()
            .starts_with("/indoctrinatedrecluse/ArcadeEdit/releases/download/")
    {
        return Err("Release asset URL is outside the trusted GitHub project".into());
    }
    Ok(())
}

/// Downloads and verifies the release executable, then writes it beside the installed app.
pub async fn download_and_stage(
    client: &Arc<dyn HttpClient>,
    update: &UpdateInfo,
    executable_path: &Path,
) -> Result<PathBuf, String> {
    validate_asset_url(&update.asset_url)?;
    let request = Request::get(update.asset_url.as_str())
        .header("Accept", "application/octet-stream")
        .header(
            "User-Agent",
            format!("ArcadeEdit/{}", env!("CARGO_PKG_VERSION")),
        )
        .timeout(Duration::from_secs(120))
        .follow_redirects(gpui::http_client::RedirectPolicy::FollowAll)
        .body(AsyncBody::empty())
        .map_err(|error| format!("Could not create update download request: {error}"))?;

    let response = client
        .send(request)
        .await
        .map_err(|error| format!("Could not download update: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Update download returned HTTP {}",
            response.status()
        ));
    }

    let mut bytes = Vec::new();
    response
        .into_body()
        .take(MAX_UPDATE_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("Could not read update download: {error}"))?;
    if bytes.len() as u64 > MAX_UPDATE_BYTES {
        return Err("Update download exceeded the 150 MiB safety limit".into());
    }
    if bytes.len() as u64 != update.asset_size {
        return Err(format!(
            "Update size mismatch: expected {} bytes, received {}",
            update.asset_size,
            bytes.len()
        ));
    }
    let actual_digest = format!("{:x}", Sha256::digest(&bytes));
    if actual_digest != update.sha256 {
        return Err("Update SHA-256 verification failed; the application was not changed".into());
    }

    let parent = executable_path
        .parent()
        .ok_or_else(|| "Could not find the application directory".to_string())?;
    for attempt in 0..32 {
        let staged = parent.join(format!(
            ".arcade-desktop-update-{}-{attempt}.exe",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
        {
            Ok(mut file) => {
                use std::io::Write as _;
                if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                    let _ = std::fs::remove_file(&staged);
                    return Err(format!("Could not stage verified update: {error}"));
                }
                return Ok(staged);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not stage verified update: {error}")),
        }
    }
    Err("Could not allocate a unique update staging file".into())
}

/// Starts a small PowerShell handoff that replaces the closed executable and restarts it.
#[cfg(windows)]
pub fn launch_installer(
    staged_executable: &Path,
    installed_executable: &Path,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let parent = installed_executable
        .parent()
        .ok_or_else(|| "Could not locate the application directory".to_string())?;
    let script_path = parent.join(format!(".arcade-update-{}.ps1", std::process::id()));
    let log_path = parent.join("arcade-update.log");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n$targetPid = {}\n$deadline = [DateTime]::UtcNow.AddSeconds(60)\nwhile (Get-Process -Id $targetPid -ErrorAction SilentlyContinue) {{ if ([DateTime]::UtcNow -gt $deadline) {{ throw 'ArcadeEdit did not close in time' }}; Start-Sleep -Milliseconds 250 }}\ntry {{ Move-Item -LiteralPath '{}' -Destination '{}' -Force; Start-Process -FilePath '{}' }} catch {{ Set-Content -LiteralPath '{}' -Value $_.ToString(); exit 1 }}\nRemove-Item -LiteralPath $PSCommandPath -ErrorAction SilentlyContinue\n",
        std::process::id(),
        escape_powershell_path(staged_executable),
        escape_powershell_path(installed_executable),
        escape_powershell_path(installed_executable),
        escape_powershell_path(&log_path),
    );
    std::fs::write(&script_path, script)
        .map_err(|error| format!("Could not create update handoff: {error}"))?;

    match Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&script_path);
            let _ = std::fs::remove_file(staged_executable);
            Err(format!("Could not start update handoff: {error}"))
        }
    }
}

#[cfg(windows)]
fn escape_powershell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_project_release_asset_urls() {
        assert!(validate_asset_url(
            "https://github.com/indoctrinatedrecluse/ArcadeEdit/releases/download/v1.3.0/arcade-desktop-windows-x86_64.exe"
        )
        .is_ok());
        assert!(validate_asset_url(
            "http://github.com/indoctrinatedrecluse/ArcadeEdit/releases/download/v1/update.exe"
        )
        .is_err());
        assert!(validate_asset_url(
            "https://evil.example/indoctrinatedrecluse/ArcadeEdit/releases/download/v1/update.exe"
        )
        .is_err());
    }

    #[test]
    fn compares_semantic_versions_instead_of_lexically() {
        assert!(Version::parse("1.10.0").unwrap() > Version::parse("1.9.9").unwrap());
        assert!(Version::parse("1.3.0").unwrap() > Version::parse("1.2.1").unwrap());
    }
}
