use std::{
    collections::HashSet,
    fs::{self, File},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use tokio::time::sleep;

use crate::chrome::window as chrome_window;

#[derive(Debug, Clone, Serialize)]
pub struct ChromeLaunchState {
    pub pid: u32,
    pub profile: String,
    pub log_path: String,
    pub url: String,
    pub window_id: String,
    pub window_title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub async fn launch(
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let state = launch_and_wait(initial_url, profile_override, timeout_ms, poll_ms).await?;
    Ok(format!(
        concat!(
            "launched chrome pid {} with profile {}\n",
            "window: {} ({}, {}x{} at {},{})\n",
            "url: {}\n",
            "log: {}",
        ),
        state.pid,
        state.profile,
        state.window_id,
        state.window_title,
        state.width,
        state.height,
        state.x,
        state.y,
        state.url,
        state.log_path,
    ))
}

pub async fn launch_and_wait(
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<ChromeLaunchState> {
    let url = normalize_launch_url(initial_url.unwrap_or("about:blank"))?;
    let baseline: HashSet<String> = chrome_window::list_browser_windows(None)
        .unwrap_or_default()
        .into_iter()
        .map(|window| window.id)
        .collect();

    let profile_dir = prepare_profile_dir(profile_override)?;
    let log_path = unique_path("axonbrowser-chrome-launch", "log");
    let browser_binary = find_browser_binary()?;
    let mut child = spawn_browser(&browser_binary, &profile_dir, &log_path, &url)?;
    let pid = child.id();

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));

    loop {
        if let Some(window) = detect_new_window(&baseline)? {
            let _ = crate::window::activate_window(&window.id);
            return Ok(ChromeLaunchState {
                pid,
                profile: profile_dir.display().to_string(),
                log_path: log_path.display().to_string(),
                url: url.clone(),
                window_id: window.id,
                window_title: window.name,
                x: window.x,
                y: window.y,
                width: window.width,
                height: window.height,
            });
        }

        if let Some(status) = child.try_wait()? {
            let log_note = log_path.display().to_string();
            bail!(
                "chrome exited before exposing a visible window (status: {}); log: {}",
                status,
                log_note
            );
        }

        if start.elapsed() >= timeout {
            bail!(
                "timed out after {}ms waiting for a new chrome window; log: {}",
                timeout_ms,
                log_path.display()
            );
        }

        sleep(interval).await;
    }
}

fn detect_new_window(baseline: &HashSet<String>) -> Result<Option<crate::window::WindowMatch>> {
    for window in chrome_window::list_browser_windows(None)? {
        if !baseline.contains(&window.id) {
            return Ok(Some(window));
        }
    }
    Ok(None)
}

fn spawn_browser(
    browser_binary: &str,
    profile_dir: &PathBuf,
    log_path: &PathBuf,
    url: &str,
) -> Result<Child> {
    let stdout = File::create(log_path).with_context(|| {
        format!(
            "failed to create chrome launch log at {}",
            log_path.display()
        )
    })?;
    let stderr = stdout.try_clone().with_context(|| {
        format!(
            "failed to clone chrome launch log handle {}",
            log_path.display()
        )
    })?;

    Command::new(browser_binary)
        .arg(format!("--user-data-dir={}", profile_dir.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-search-engine-choice-screen",
            "--force-renderer-accessibility",
            "--new-window",
            "--window-size=1280,900",
            "--disable-background-networking",
            "--disable-sync",
            "--disable-component-update",
            "--disable-breakpad",
            "--no-sandbox",
        ])
        .arg(url)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to launch {browser_binary}"))
}

fn find_browser_binary() -> Result<String> {
    for candidate in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ] {
        let output = Command::new("which")
            .arg(candidate)
            .output()
            .with_context(|| format!("failed checking browser binary {candidate}"))?;
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !resolved.is_empty() {
                return Ok(resolved);
            }
        }
    }

    Err(anyhow!(
        "could not find a Chrome/Chromium binary in PATH (tried google-chrome, google-chrome-stable, chromium, chromium-browser)"
    ))
}

fn prepare_profile_dir(profile_override: Option<&str>) -> Result<PathBuf> {
    let path = profile_override
        .map(PathBuf::from)
        .unwrap_or_else(|| unique_path("axonbrowser-chrome-profile", "dir"));
    fs::create_dir_all(&path)
        .with_context(|| format!("failed to create chrome profile dir {}", path.display()))?;
    Ok(path)
}

fn unique_path(prefix: &str, suffix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("{prefix}-{stamp}.{suffix}"))
}

fn normalize_launch_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("launch URL must not be empty");
    }
    if trimmed.contains(char::is_whitespace) {
        bail!("launch URL must not contain whitespace: {:?}", raw);
    }
    if trimmed.contains("://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("chrome:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}

#[cfg(test)]
mod tests {
    use super::normalize_launch_url;

    #[test]
    fn preserves_special_urls() {
        assert_eq!(normalize_launch_url("about:blank").unwrap(), "about:blank");
        assert_eq!(
            normalize_launch_url("https://example.com").unwrap(),
            "https://example.com"
        );
    }

    #[test]
    fn prefixes_https_for_bare_hosts() {
        assert_eq!(
            normalize_launch_url("example.com").unwrap(),
            "https://example.com"
        );
    }
}
