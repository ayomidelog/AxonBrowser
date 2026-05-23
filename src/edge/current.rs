use anyhow::Result;
use serde::Serialize;

use super::{session, tabs, wait, window};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EdgeCurrentState {
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

pub async fn snapshot() -> Result<EdgeCurrentState> {
    snapshot_for_window(None).await
}

pub async fn snapshot_for_window(window_id: Option<&str>) -> Result<EdgeCurrentState> {
    let window = match window_id {
        Some(id) => {
            let window = window::find_edge_window_by_id(id)?;
            let _ = crate::edge::session::remember_browser_window_target(&window.id);
            let _ = crate::edge::discovery::remember_profile_from_window(&window.id);
            window
        }
        None => {
            let window = window::find_edge_window(None)?;
            let _ = crate::edge::discovery::remember_profile_from_window(&window.id);
            window
        }
    };
    let tabs = tabs::model::resolve_tabs_with_current().await?;
    let current = tabs
        .iter()
        .find(|tab| tab.is_current)
        .cloned()
        .unwrap_or_else(|| tabs[0].clone());

    Ok(EdgeCurrentState {
        window_title: window.name,
        current_tab_title: current.title,
        current_tab_index: current.index,
        tab_count: tabs.len(),
        url: read_current_url().await,
    })
}

pub fn render_text(state: &EdgeCurrentState) -> String {
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

async fn read_current_url() -> String {
    if let Ok(url) = wait::current_url().await {
        return url;
    }

    session::read_browser_url().unwrap_or_else(|| "<unavailable>".to_string())
}
