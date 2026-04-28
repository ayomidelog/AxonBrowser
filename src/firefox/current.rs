use anyhow::Result;
use serde::Serialize;
use tokio::time::{Duration, timeout};

use super::{tabs, url, wait, window};
const FIREFOX_GENERIC_WINDOW_TITLE: &str = "Mozilla Firefox";
const STABLE_WINDOW_TIMEOUT_MS: u64 = 4_000;
const STABLE_WINDOW_POLL_MS: u64 = 150;
const SNAPSHOT_READ_TIMEOUT_MS: u64 = 1_500;

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
    let window = resolve_stable_window(window_id).await?;
    if window_id.is_some() {
        let _ = crate::firefox::session::remember_target(&window.id);
    }
    let tabs = timeout(
        Duration::from_millis(SNAPSHOT_READ_TIMEOUT_MS),
        tabs::model::resolve_tabs_with_current_for_window(Some(&window.id)),
    )
    .await
    .ok()
    .and_then(Result::ok);
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

    let url = timeout(
        Duration::from_millis(SNAPSHOT_READ_TIMEOUT_MS),
        read_current_url_with_fallback(&current_tab_title, Some(&window.id)),
    )
    .await
    .unwrap_or_else(|_| "<unavailable>".to_string());

    Ok(FirefoxCurrentState {
        current_tab_title,
        window_title: window.name,
        current_tab_index,
        tab_count,
        url,
    })
}

async fn resolve_stable_window(window_id: Option<&str>) -> Result<crate::window::WindowMatch> {
    let timeout = std::time::Duration::from_millis(STABLE_WINDOW_TIMEOUT_MS);
    let poll = std::time::Duration::from_millis(STABLE_WINDOW_POLL_MS);
    let started = std::time::Instant::now();

    loop {
        let candidate = match window_id {
            Some(id) => window::find_firefox_window_by_id(id)?,
            None => window::find_firefox_window(None)?,
        };
        let inferred = infer_tab_title(&candidate.name);
        if inferred != FIREFOX_GENERIC_WINDOW_TITLE || started.elapsed() >= timeout {
            return Ok(candidate);
        }
        tokio::time::sleep(poll).await;
    }
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

fn infer_tab_title(window_title: &str) -> String {
    let mut trimmed = window_title.trim().to_string();
    if let Some(stripped) = trimmed.strip_suffix(" - Mozilla Firefox") {
        trimmed = stripped.trim().to_string();
    }

    if let Some((prefix, profile)) = trimmed.rsplit_once(" - ") {
        let profile_normalized = profile.trim().to_ascii_lowercase();
        if profile_normalized.starts_with("profile ") || profile_normalized.contains("private") {
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
        if let Ok(window) = window::find_firefox_window_by_id(id) {
            let _ = crate::firefox::session::remember_target(&window.id);
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
