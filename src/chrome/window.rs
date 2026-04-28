use anyhow::{Result, anyhow};

use crate::window::{self, WindowMatch};

use super::session;

const DEFAULT_BROWSER_QUERIES: &[&str] = &[
    "google chrome",
    "chromium",
    "chrome",
    "chromium-browser",
    "welcome to google chrome",
];
const EDGE_BROWSER_QUERIES: &[&str] = &["microsoft edge", "microsoft\u{200b} edge", "msedge"];

pub fn find_browser_window(query_override: Option<&str>) -> Result<WindowMatch> {
    if let Some(query) = query_override {
        return list_browser_windows(Some(query))?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no visible Chrome/Chromium window found"));
    }

    if let Some(target_id) = session::read_browser_window_target() {
        if let Ok(window) = find_browser_window_by_id(&target_id) {
            return Ok(window);
        }
    }

    list_browser_windows(None)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no visible Chrome/Chromium window found"))
}

pub fn find_browser_window_by_id(window_id: &str) -> Result<WindowMatch> {
    let wanted = window_id.trim();
    window::list_visible_windows()?
        .into_iter()
        .find(|window| window.id == wanted && is_browser_window_name(&window.name))
        .ok_or_else(|| anyhow!("no visible Chrome/Chromium window found for id {}", wanted))
}

pub fn list_browser_windows(query_override: Option<&str>) -> Result<Vec<WindowMatch>> {
    let mut windows = window::list_visible_windows()?;

    if let Some(query) = query_override {
        let needle = normalize(query);
        windows.retain(|window| normalize(&window.name).contains(&needle));
        return Ok(windows);
    }

    windows.retain(|window| is_browser_window_name(&window.name));
    windows.sort_by_key(|window| sort_key(window));
    Ok(windows)
}

fn sort_key(window: &WindowMatch) -> (u8, u8, u8, String) {
    let active_id = window::active_window_id().ok();
    let target_id = session::read_browser_window_target();
    let is_active = active_id
        .as_deref()
        .map(|id| id == window.id)
        .unwrap_or(false);
    let is_target = target_id
        .as_deref()
        .map(|id| id == window.id)
        .unwrap_or(false);
    let is_welcome = normalize(&window.name).contains("welcome to google chrome");
    (
        if is_target { 0 } else { 1 },
        if is_active { 0 } else { 1 },
        if is_welcome { 1 } else { 0 },
        window.id.clone(),
    )
}

fn is_browser_window_name(name: &str) -> bool {
    let normalized = normalize(name);
    current_browser_queries()
        .iter()
        .any(|needle| normalized.contains(needle))
}

fn current_browser_queries() -> &'static [&'static str] {
    if std::env::var("GUIBOT_BROWSER_WINDOW_MODE").ok().as_deref() == Some("edge") {
        EDGE_BROWSER_QUERIES
    } else {
        DEFAULT_BROWSER_QUERIES
    }
}

pub fn normalize(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
