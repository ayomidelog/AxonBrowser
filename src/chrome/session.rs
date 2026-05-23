use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

const TARGET_WINDOW_FILE: &str = "axonbrowser-target-window";
const TARGET_URL_FILE: &str = "axonbrowser-target-url";
const TARGET_PROFILE_FILE: &str = "axonbrowser-target-profile";
const TARGET_TAB_FILE: &str = "axonbrowser-target-tab";

pub fn remember_browser_window_target(window_id: &str) -> Result<()> {
    let path = target_window_file();
    fs::write(&path, window_id.trim()).with_context(|| {
        format!(
            "failed to write target browser window file {}",
            path.display()
        )
    })
}

pub fn read_browser_window_target() -> Option<String> {
    let path = target_window_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn remember_browser_url(url: &str) -> Result<()> {
    let path = target_url_file();
    fs::write(&path, url.trim()).with_context(|| {
        format!(
            "failed to write target browser url file {}",
            path.display()
        )
    })
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
            "failed to write target browser profile file {}",
            path.display()
        )
    })
}

pub fn read_browser_profile() -> Option<String> {
    let path = target_profile_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn remember_browser_tab_target(target_id: &str) -> Result<()> {
    let path = target_tab_file();
    fs::write(&path, target_id.trim()).with_context(|| {
        format!(
            "failed to write target browser tab file {}",
            path.display()
        )
    })
}

pub fn read_browser_tab_target() -> Option<String> {
    let path = target_tab_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn clear_browser_session_state() {
    let _ = fs::remove_file(target_window_file());
    let _ = fs::remove_file(target_url_file());
    let _ = fs::remove_file(target_profile_file());
    let _ = fs::remove_file(target_tab_file());
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

fn target_tab_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_TAB_FILE)
}
