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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFlavor {
    Firefox,
    Camoufox,
}

impl BrowserFlavor {
    pub fn label(self) -> &'static str {
        match self {
            Self::Firefox => "firefox",
            Self::Camoufox => "camoufox",
        }
    }

    fn user_profile_prefix(self) -> &'static str {
        match self {
            Self::Firefox => "axonbrowser-firefox-profile",
            Self::Camoufox => "axonbrowser-camoufox-profile",
        }
    }

    fn launch_log_prefix(self) -> &'static str {
        match self {
            Self::Firefox => "axonbrowser-firefox-launch",
            Self::Camoufox => "axonbrowser-camoufox-launch",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FirefoxLaunchState {
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

#[allow(dead_code)]
pub async fn launch(
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    launch_with_flavor(
        BrowserFlavor::Firefox,
        initial_url,
        profile_override,
        timeout_ms,
        poll_ms,
    )
    .await
}

pub async fn launch_with_flavor(
    flavor: BrowserFlavor,
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let state = launch_and_wait_with_flavor(
        flavor,
        initial_url,
        profile_override,
        timeout_ms,
        poll_ms,
    )
    .await?;
    Ok(format!(
        concat!(
            "launched {} pid {} with profile {}\n",
            "window: {} ({}, {}x{} at {},{})\n",
            "url: {}\n",
            "log: {}",
        ),
        flavor.label(),
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

#[allow(dead_code)]
pub async fn launch_and_wait(
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<FirefoxLaunchState> {
    launch_and_wait_with_flavor(
        BrowserFlavor::Firefox,
        initial_url,
        profile_override,
        timeout_ms,
        poll_ms,
    )
    .await
}

pub async fn launch_and_wait_with_flavor(
    flavor: BrowserFlavor,
    initial_url: Option<&str>,
    profile_override: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<FirefoxLaunchState> {
    let url = normalize_launch_url(initial_url.unwrap_or("about:blank"))?;
    let baseline: HashSet<String> = window::list_firefox_windows(None)
        .unwrap_or_default()
        .into_iter()
        .map(|window| window.id)
        .collect();

    let profile_dir = prepare_profile_dir(flavor, profile_override)?;
    let log_path = unique_path(flavor.launch_log_prefix(), "log");
    let browser_binary = find_browser_binary(flavor)?;
    let mut child = spawn_browser(flavor, &browser_binary, &profile_dir, &log_path, &url)?;
    let pid = child.id();

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.max(15000));
    let interval = Duration::from_millis(poll_ms.max(1));

    loop {
        if let Some(window_match) = detect_new_window(&baseline)? {
            session::remember_target(&window_match.id)?;
            let _ = crate::window::activate_window(&window_match.id);
            return Ok(FirefoxLaunchState {
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
                "{} exited before exposing a visible window (status: {}); log: {}",
                flavor.label(),
                status,
                log_path.display()
            );
        }

        if start.elapsed() >= timeout {
            bail!(
                "timed out after {}ms waiting for a new {} window; log: {}",
                timeout_ms,
                flavor.label(),
                log_path.display()
            );
        }

        sleep(interval).await;
    }
}

fn detect_new_window(baseline: &HashSet<String>) -> Result<Option<crate::window::WindowMatch>> {
    for window_match in window::list_firefox_windows(None)? {
        if !baseline.contains(&window_match.id) {
            return Ok(Some(window_match));
        }
    }
    Ok(None)
}

fn spawn_browser(
    flavor: BrowserFlavor,
    browser_binary: &str,
    profile_dir: &PathBuf,
    log_path: &PathBuf,
    url: &str,
) -> Result<Child> {
    let stdout = File::create(log_path).with_context(|| {
        format!(
            "failed to create firefox launch log at {}",
            log_path.display()
        )
    })?;
    let stderr = stdout.try_clone().with_context(|| {
        format!(
            "failed to clone firefox launch log handle {}",
            log_path.display()
        )
    })?;

    Command::new(browser_binary)
        .env_remove("NO_AT_BRIDGE")
        .env("MOZ_ACCESSIBILITY_FORCE_DISABLED", "0")
        .env(
            "DISPLAY",
            std::env::var("DISPLAY").unwrap_or_else(|_| ":99".to_string()),
        )
        .arg("--new-window")
        .arg("--profile")
        .arg(profile_dir)
        .arg(url)
        .args(extra_browser_args(flavor))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to launch {browser_binary}"))
}

fn find_browser_binary(flavor: BrowserFlavor) -> Result<String> {
    let env_override = match flavor {
        BrowserFlavor::Firefox => std::env::var("GUIBOT_FIREFOX_BIN").ok(),
        BrowserFlavor::Camoufox => std::env::var("GUIBOT_CAMOUFOX_BIN").ok(),
    };
    if let Some(path) = env_override {
        if !path.trim().is_empty() {
            return Ok(path);
        }
    }

    let candidates = match flavor {
        BrowserFlavor::Firefox => vec![
            "firefox".to_string(),
            "firefox-esr".to_string(),
            "/usr/bin/firefox".to_string(),
            "/usr/bin/firefox-esr".to_string(),
        ],
        BrowserFlavor::Camoufox => {
            let mut candidates = vec![
                "camoufox".to_string(),
                format!(
                    "{}/.cache/camoufox/camoufox",
                    std::env::var("HOME").unwrap_or_default()
                ),
                format!(
                    "{}/.cache/camoufox/camoufox-bin",
                    std::env::var("HOME").unwrap_or_default()
                ),
            ];
            if let Some(path) = resolve_camoufox_python_binary()? {
                candidates.insert(0, path);
            }
            candidates
        }
    };

    for candidate in candidates {
        if std::path::Path::new(&candidate).exists() {
            return Ok(candidate);
        }
        let output = Command::new("sh")
            .args(["-lc", &format!("command -v {}", candidate)])
            .output()
            .with_context(|| format!("failed checking {} binary {candidate}", flavor.label()))?;
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !resolved.is_empty() {
                return Ok(resolved);
            }
        }
    }

    Err(anyhow!(
        "could not find a {} binary; set GUIBOT_{}_BIN to override",
        flavor.label(),
        flavor.label().to_ascii_uppercase()
    ))
}

fn resolve_camoufox_python_binary() -> Result<Option<String>> {
    for python in ["python3", "python"] {
        let output = Command::new(python)
            .args(["-m", "camoufox", "path"])
            .output();
        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !resolved.is_empty() {
            return Ok(Some(resolved));
        }
    }
    Ok(None)
}

fn prepare_profile_dir(flavor: BrowserFlavor, profile_override: Option<&str>) -> Result<PathBuf> {
    let path = profile_override
        .map(PathBuf::from)
        .unwrap_or_else(|| unique_path(flavor.user_profile_prefix(), "dir"));
    fs::create_dir_all(&path)
        .with_context(|| format!("failed to create firefox profile dir {}", path.display()))?;
    let prefs = path.join("user.js");
    fs::write(
        &prefs,
        [
            "user_pref(\"accessibility.force_disabled\", 0);",
            "user_pref(\"browser.shell.checkDefaultBrowser\", false);",
            "user_pref(\"browser.aboutwelcome.enabled\", false);",
            "user_pref(\"browser.startup.homepage_override.mstone\", \"ignore\");",
            "user_pref(\"startup.homepage_welcome_url\", \"\");",
            "user_pref(\"startup.homepage_welcome_url.additional\", \"\");",
            "user_pref(\"trailhead.firstrun.didSeeAboutWelcome\", true);",
            "user_pref(\"signon.rememberSignons\", true);",
        ]
        .join("\n"),
    )
    .with_context(|| format!("failed to write firefox profile prefs {}", prefs.display()))?;
    Ok(path)
}

fn extra_browser_args(flavor: BrowserFlavor) -> &'static [&'static str] {
    match flavor {
        BrowserFlavor::Firefox => &[],
        BrowserFlavor::Camoufox => &["--no-remote"],
    }
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
        || trimmed.starts_with("firefox:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}
