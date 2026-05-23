use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

const TARGET_WINDOW_FILE: &str = "axonbrowser-target-window-firefox";
const TARGET_URL_FILE: &str = "axonbrowser-target-url-firefox";
const TARGET_PROFILE_FILE: &str = "axonbrowser-target-profile-firefox";
const TARGET_PORT_FILE: &str = "axonbrowser-target-port-firefox";

pub fn remember_target(window_id: &str) -> Result<()> {
    let path = target_window_file();
    fs::write(&path, window_id.trim()).with_context(|| {
        format!(
            "failed to write firefox target window file {}",
            path.display()
        )
    })?;
    crate::chrome::session::remember_browser_window_target(window_id)
}

pub fn remember_browser_window_target(window_id: &str) -> Result<()> {
    remember_target(window_id)
}

pub fn read_target() -> Option<String> {
    let path = target_window_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn remember_browser_url(url: &str) -> Result<()> {
    let path = target_url_file();
    fs::write(&path, url.trim())
        .with_context(|| format!("failed to write firefox target URL file {}", path.display()))
}

pub fn read_browser_url() -> Option<String> {
    let path = target_url_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn remember_browser_profile(profile: &str) -> Result<()> {
    let path = target_profile_file();
    fs::write(&path, profile.trim()).with_context(|| {
        format!(
            "failed to write firefox target profile file {}",
            path.display()
        )
    })
}

pub fn remember_browser_port(port: u16) -> Result<()> {
    let path = target_port_file();
    fs::write(&path, port.to_string()).with_context(|| {
        format!(
            "failed to write firefox target port file {}",
            path.display()
        )
    })
}

pub fn read_browser_port() -> Option<u16> {
    let path = target_port_file();
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
}

pub fn clear_browser_session_state() {
    let _ = fs::remove_file(target_window_file());
    let _ = fs::remove_file(target_url_file());
    let _ = fs::remove_file(target_profile_file());
    let _ = fs::remove_file(target_port_file());
}

fn target_window_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_WINDOW_FILE)
}

fn target_url_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_URL_FILE)
}

fn target_profile_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_PROFILE_FILE)
}

fn target_port_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_PORT_FILE)
}
