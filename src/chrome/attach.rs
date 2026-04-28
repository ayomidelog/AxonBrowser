use anyhow::Result;
use serde::Serialize;

use crate::chrome::{
    current::{self, ChromeCurrentState},
    session, window as chrome_window,
};

#[derive(Debug, Clone, Serialize)]
pub struct ChromeAttachState {
    pub window_id: String,
    pub window_title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub current: ChromeCurrentState,
}

pub async fn attach(json: bool, window_id: Option<&str>) -> Result<String> {
    let state = snapshot(window_id).await?;
    if json {
        Ok(format!("{}\n", serde_json::to_string_pretty(&state)?))
    } else {
        Ok(format!(
            concat!("attached chrome window {} ({}, {}x{} at {},{})\n", "{}",),
            state.window_id,
            state.window_title,
            state.width,
            state.height,
            state.x,
            state.y,
            current::render_text(&state.current),
        ))
    }
}

pub async fn snapshot(window_id: Option<&str>) -> Result<ChromeAttachState> {
    let window = match window_id {
        Some(window_id) => chrome_window::find_browser_window_by_id(window_id)?,
        None => chrome_window::find_browser_window(None)?,
    };
    session::remember_browser_window_target(&window.id)?;
    let current = current::snapshot().await?;

    Ok(ChromeAttachState {
        window_id: window.id,
        window_title: window.name,
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
        current,
    })
}
