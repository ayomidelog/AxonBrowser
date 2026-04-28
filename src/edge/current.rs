use anyhow::Result;
use serde::Serialize;

use super::{tabs, url, wait, window};

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
        Some(id) => window::find_edge_window_by_id(id)?,
        None => window::find_edge_window(None)?,
    };
    if window_id.is_some() {
        let _ = crate::edge::session::remember_target(&window.id);
    }
    let tabs = tabs::model::resolve_tabs_with_current_for_window(Some(&window.id))
        .await
        .ok();
    let (current_tab_title, current_tab_index, tab_count) = if let Some(tabs) = tabs {
        let current = tabs
            .iter()
            .find(|tab| tab.is_current)
            .cloned()
            .unwrap_or_else(|| tabs[0].clone());
        (current.title, current.index, tabs.len())
    } else {
        (infer_tab_title(&window.name), 0, 1)
    };

    let url = read_current_url_with_fallback(&current_tab_title, Some(&window.id)).await;

    Ok(EdgeCurrentState {
        current_tab_title,
        window_title: window.name,
        current_tab_index,
        tab_count,
        url,
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

fn infer_tab_title(window_title: &str) -> String {
    let mut trimmed = window_title.trim().to_string();
    for suffix in [" - Microsoft Edge", " - Microsoft\u{200b} Edge"] {
        if let Some(stripped) = trimmed.strip_suffix(suffix) {
            trimmed = stripped.trim().to_string();
            break;
        }
    }

    if let Some((prefix, profile)) = trimmed.rsplit_once(" - ") {
        let profile_normalized = profile.trim().to_ascii_lowercase();
        if profile_normalized.starts_with("profile ") || profile_normalized == "inprivate" {
            trimmed = prefix.trim().to_string();
        }
    }

    trimmed
}

async fn read_current_url_with_fallback(
    current_tab_title: &str,
    window_id: Option<&str>,
) -> String {
    if let Some(id) = window_id {
        if let Ok(window) = window::find_edge_window_by_id(id) {
            let _ = crate::edge::session::remember_target(&window.id);
        }
    }
    if let Ok(url) = wait::current_url().await {
        return url;
    }

    let fallback = current_tab_title.trim();
    if url::is_plausible_url(fallback) {
        fallback.to_string()
    } else {
        "<unavailable>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::url::is_plausible_url;

    #[test]
    fn detects_url_like_tab_titles() {
        assert!(is_plausible_url("about:blank"));
        assert!(is_plausible_url("https://example.com"));
        assert!(!is_plausible_url("New Tab"));
    }
}
