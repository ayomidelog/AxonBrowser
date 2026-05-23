use anyhow::Result;
use serde::Serialize;

use crate::chrome::{retry, session, tabs, wait};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChromeCurrentState {
    pub window_title: String,
    pub current_tab_title: String,
    pub current_tab_index: usize,
    pub tab_count: usize,
    pub url: String,
}

pub async fn read(json: bool) -> Result<String> {
    let state = snapshot().await?;
    if json {
        render_json(&state)
    } else {
        Ok(render_text(&state))
    }
}

pub async fn snapshot() -> Result<ChromeCurrentState> {
    retry::with_transient_retry(|| async {
        let tabs = tabs::model::resolve_tabs_with_current().await?;
        let current = tabs
            .iter()
            .find(|tab| tab.is_current)
            .cloned()
            .unwrap_or_else(|| tabs[0].clone());
        let window_title = if current.title.trim().is_empty() {
            "Google Chrome".to_string()
        } else {
            format!("{} - Google Chrome", current.title)
        };

        Ok(ChromeCurrentState {
            window_title,
            current_tab_title: current.title,
            current_tab_index: current.index,
            tab_count: tabs.len(),
            url: read_current_url().await,
        })
    })
    .await
}

pub fn render_text(state: &ChromeCurrentState) -> String {
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

pub fn render_json(state: &ChromeCurrentState) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(state)?))
}

async fn read_current_url() -> String {
    match wait::current_url().await {
        Ok(url) => url,
        Err(_) => session::read_browser_url().unwrap_or_else(|| "<unavailable>".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{ChromeCurrentState, render_json, render_text};

    #[test]
    fn renders_current_state() {
        let state = ChromeCurrentState {
            window_title: "Example Domain - Google Chrome".into(),
            current_tab_title: "Example Domain".into(),
            current_tab_index: 1,
            tab_count: 3,
            url: "https://example.com/".into(),
        };

        let rendered = render_text(&state);
        assert!(rendered.contains("window: Example Domain - Google Chrome"));
        assert!(rendered.contains("current tab: Example Domain"));
        assert!(rendered.contains("current tab index: 1"));
        assert!(rendered.contains("tab count: 3"));
        assert!(rendered.contains("url: https://example.com/"));
    }

    #[test]
    fn renders_unavailable_url() {
        let state = ChromeCurrentState {
            window_title: "New Tab - Google Chrome".into(),
            current_tab_title: "New Tab".into(),
            current_tab_index: 0,
            tab_count: 1,
            url: "<unavailable>".into(),
        };

        let rendered = render_text(&state);
        assert!(rendered.contains("url: <unavailable>"));
    }

    #[test]
    fn renders_json_output() {
        let state = ChromeCurrentState {
            window_title: "Example Domain - Google Chrome".into(),
            current_tab_title: "Example Domain".into(),
            current_tab_index: 1,
            tab_count: 3,
            url: "https://example.com/".into(),
        };

        let rendered = render_json(&state).unwrap();
        assert!(rendered.contains("\"window_title\": \"Example Domain - Google Chrome\""));
        assert!(rendered.ends_with("\n"));
    }
}
