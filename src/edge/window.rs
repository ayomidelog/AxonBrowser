use anyhow::{Result, anyhow};

use crate::window::{self, WindowMatch};

use super::session;

const EDGE_NEEDLES: &[&str] = &["microsoft edge", "microsoft\u{200b} edge", "msedge"];

pub fn find_edge_window(query_override: Option<&str>) -> Result<WindowMatch> {
    if let Some(query) = query_override {
        return list_edge_windows(Some(query))?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no visible Microsoft Edge window found"));
    }

    if let Some(target_id) = session::read_target() {
        if let Ok(window) = find_edge_window_by_id(&target_id) {
            return Ok(window);
        }
    }

    list_edge_windows(None)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no visible Microsoft Edge window found"))
}

pub fn find_edge_window_by_id(window_id: &str) -> Result<WindowMatch> {
    let wanted = window_id.trim();
    list_edge_windows(None)?
        .into_iter()
        .find(|window| window.id == wanted)
        .ok_or_else(|| anyhow!("no visible Microsoft Edge window found for id {}", wanted))
}

pub fn list_edge_windows(query_override: Option<&str>) -> Result<Vec<WindowMatch>> {
    let mut windows = window::list_visible_windows()?;

    if let Some(query) = query_override {
        let needle = normalize(query);
        windows.retain(|window| normalize(&window.name).contains(&needle));
        return Ok(windows);
    }

    windows.retain(|window| is_edge_window_name(&window.name));
    windows.sort_by_key(sort_key);
    Ok(windows)
}

fn sort_key(window: &WindowMatch) -> (u8, u8, String) {
    let active_id = window::active_window_id().ok();
    let target_id = session::read_target();
    let is_active = active_id.as_deref() == Some(window.id.as_str());
    let is_target = target_id.as_deref() == Some(window.id.as_str());
    (
        if is_target { 0 } else { 1 },
        if is_active { 0 } else { 1 },
        window.id.clone(),
    )
}

fn is_edge_window_name(name: &str) -> bool {
    let normalized = normalize(name);
    EDGE_NEEDLES
        .iter()
        .any(|needle| normalized.contains(needle))
}

pub fn normalize(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
