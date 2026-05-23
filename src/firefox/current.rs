use anyhow::Result;
use serde::Serialize;

use super::{bidi, session, window};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirefoxCurrentState {
    pub window_title: String,
    pub current_tab_title: String,
    pub current_tab_index: usize,
    pub tab_count: usize,
    pub url: String,
}

pub async fn read(json: bool) -> Result<String> {
    let state = snapshot().await?;
    if json {
        Ok(format!("{}\n", serde_json::to_string_pretty(&state)?))
    } else {
        Ok(render_text(&state))
    }
}

pub async fn read_for_window(json: bool, window_id: Option<&str>) -> Result<String> {
    let state = snapshot_for_window(window_id).await?;
    if json {
        Ok(format!("{}\n", serde_json::to_string_pretty(&state)?))
    } else {
        Ok(render_text(&state))
    }
}

pub async fn snapshot() -> Result<FirefoxCurrentState> {
    snapshot_for_window(None).await
}

pub async fn snapshot_for_window(window_id: Option<&str>) -> Result<FirefoxCurrentState> {
    let window = match window_id {
        Some(id) => {
            let window = window::find_firefox_window_by_id(id)?;
            let _ = crate::firefox::session::remember_browser_window_target(&window.id);
            window
        }
        None => window::find_firefox_window(None)?,
    };
    let tabs = bidi::list_contexts().await?;
    let current = tabs
        .iter()
        .find(|tab| tab.is_current)
        .cloned()
        .unwrap_or_else(|| tabs[0].clone());

    Ok(FirefoxCurrentState {
        window_title: window.name,
        current_tab_title: current.title,
        current_tab_index: tabs.iter().position(|tab| tab.is_current).unwrap_or(0),
        tab_count: tabs.len(),
        url: bidi::current_url()
            .await?
            .or_else(session::read_browser_url)
            .unwrap_or_else(|| "<unavailable>".to_string()),
    })
}

pub fn render_text(state: &FirefoxCurrentState) -> String {
    format!(
        concat!(
            "window: {}\n",
            "current tab: {}\n",
            "current tab index: {}\n",
            "tab count: {}\n",
            "url: {}\n",
        ),
        state.window_title,
        state.current_tab_title,
        state.current_tab_index,
        state.tab_count,
        state.url,
    )
}
