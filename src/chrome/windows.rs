use anyhow::Result;
use serde::Serialize;

use super::{session, window as chrome_window};

#[derive(Debug, Clone, Serialize)]
pub struct ChromeWindowInfo {
    pub window_id: String,
    pub window_title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_active: bool,
    pub is_target: bool,
}

pub fn list() -> Result<String> {
    let windows = snapshot()?;
    let mut out = String::new();
    for window in windows {
        let mut markers = Vec::new();
        if window.is_active {
            markers.push("active");
        }
        if window.is_target {
            markers.push("target");
        }
        let marker_text = if markers.is_empty() {
            "-".to_string()
        } else {
            format!("[{}]", markers.join(","))
        };
        out.push_str(&format!(
            "{} {} {} ({}x{} at {},{})\n",
            marker_text,
            window.window_id,
            window.window_title,
            window.width,
            window.height,
            window.x,
            window.y
        ));
    }
    Ok(out)
}

pub fn snapshot() -> Result<Vec<ChromeWindowInfo>> {
    let target_id = session::read_browser_window_target();
    let active_id = crate::window::active_window_id().ok();
    let windows = chrome_window::list_browser_windows(None)?;
    Ok(windows
        .into_iter()
        .map(|window| ChromeWindowInfo {
            window_id: window.id.clone(),
            window_title: window.name,
            x: window.x,
            y: window.y,
            width: window.width,
            height: window.height,
            is_active: active_id.as_deref() == Some(window.id.as_str()),
            is_target: target_id.as_deref() == Some(window.id.as_str()),
        })
        .collect())
}
