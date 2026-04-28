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

use super::{session, window};

#[derive(Debug, Clone, Serialize)]
pub struct EdgeLaunchState {
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
            "launched edge pid {} with profile {}\n",
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
) -> Result<EdgeLaunchState> {
    let url = normalize_launch_url(initial_url.unwrap_or("about:blank"))?;
    let baseline: HashSet<String> = window::list_edge_windows(None)
        .unwrap_or_default()
        .into_iter()
        .map(|window| window.id)
        .collect();

    let profile_dir = prepare_profile_dir(profile_override)?;
    let log_path = unique_path("axonbrowser-edge-launch", "log");
    let browser_binary = find_edge_binary()?;
    let mut child = spawn_browser(&browser_binary, &profile_dir, &log_path, &url)?;
    let pid = child.id();

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));

    loop {
        if let Some(window_match) = detect_new_window(&baseline)? {
            session::remember_target(&window_match.id)?;
            let _ = crate::window::activate_window(&window_match.id);
            return Ok(EdgeLaunchState {
                pid,
                profile: profile_dir.display().to_string(),
                log_path: log_path.display().to_string(),
                url: url.clone(),
                window_id: window_match.id,
                window_title: window_match.name,
                x: window_match.x,
                y: window_match.y,
                width: window_match.width,
                height: window_match.height,
            });
        }

        if let Some(status) = child.try_wait()? {
            bail!(
                "edge exited before exposing a visible window (status: {}); log: {}",
                status,
                log_path.display()
            );
        }

        if start.elapsed() >= timeout {
            bail!(
                "timed out after {}ms waiting for a new edge window; log: {}",
                timeout_ms,
                log_path.display()
            );
        }

        sleep(interval).await;
    }
}

fn detect_new_window(baseline: &HashSet<String>) -> Result<Option<crate::window::WindowMatch>> {
    for window_match in window::list_edge_windows(None)? {
        if !baseline.contains(&window_match.id) {
            return Ok(Some(window_match));
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
    let stdout = File::create(log_path)
        .with_context(|| format!("failed to create edge launch log at {}", log_path.display()))?;
    let stderr = stdout.try_clone().with_context(|| {
        format!(
            "failed to clone edge launch log handle {}",
            log_path.display()
        )
    })?;

    Command::new(browser_binary)
        .arg(format!("--user-data-dir={}", profile_dir.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
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

fn find_edge_binary() -> Result<String> {
    for candidate in [
        "microsoft-edge",
        "microsoft-edge-stable",
        "/usr/bin/microsoft-edge",
        "/usr/bin/microsoft-edge-stable",
    ] {
        let output = Command::new("sh")
            .args(["-lc", &format!("command -v {}", candidate)])
            .output()
            .with_context(|| format!("failed checking edge binary {candidate}"))?;
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !resolved.is_empty() {
                return Ok(resolved);
            }
        }
    }

    Err(anyhow!(
        "could not find a Microsoft Edge binary in PATH (tried microsoft-edge, microsoft-edge-stable)"
    ))
}

fn prepare_profile_dir(profile_override: Option<&str>) -> Result<PathBuf> {
    let path = profile_override
        .map(PathBuf::from)
        .unwrap_or_else(|| unique_path("axonbrowser-edge-profile", "dir"));
    fs::create_dir_all(&path)
        .with_context(|| format!("failed to create edge profile dir {}", path.display()))?;
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
        || trimmed.starts_with("edge:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}
