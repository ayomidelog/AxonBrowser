use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::time::sleep;

use crate::{chrome::retry, firefox::actions, firefox::url, live_access};

const ADDRESS_BAR_PLACEHOLDER_NAMES: &[&str] = &[
    "search or enter web address",
    "address and search bar",
    "address bar",
];

pub async fn wait_for_locator(locator_raw: &str, timeout_ms: u64, poll_ms: u64) -> Result<String> {
    if matches!(
        locator_raw.trim().to_ascii_lowercase().as_str(),
        "address" | "address bar" | "addressbar" | "address-bar" | "omnibox"
    ) {
        let window = crate::firefox::window::find_firefox_window(None)?;
        return Ok(format!(
            "firefox locator {:?} available via browser shortcut in window {} ({})",
            locator_raw, window.id, window.name
        ));
    }

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    let last_error = loop {
        attempts += 1;
        match actions::click::resolve_locator(locator_raw).await {
            Ok(node) => {
                return Ok(format!(
                    "firefox locator {:?} matched {} after {}ms ({} attempts)",
                    locator_raw,
                    node.line_label(),
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    break err.to_string();
                }
            }
        }

        sleep(interval).await;
    };

    Err(anyhow!(
        "timed out after {}ms waiting for firefox locator {:?}: {}",
        timeout_ms,
        locator_raw,
        last_error
    ))
}

pub async fn wait_for_title_change(
    from: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let baseline = match from {
        Some(value) => value.to_string(),
        None => current_title().await?,
    };
    wait_for_change("title", &baseline, timeout_ms, poll_ms, current_title).await
}

pub async fn wait_for_url_change(
    from: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let baseline = match from {
        Some(value) => value.to_string(),
        None => current_url().await?,
    };
    wait_for_change("url", &baseline, timeout_ms, poll_ms, current_url).await
}

async fn wait_for_change<F, Fut>(
    label: &str,
    baseline: &str,
    timeout_ms: u64,
    poll_ms: u64,
    mut read_current: F,
) -> Result<String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    let last_error = loop {
        attempts += 1;
        match read_current().await {
            Ok(current) => {
                if current != baseline {
                    return Ok(format!(
                        "firefox {} changed from {:?} to {:?} after {}ms ({} attempts)",
                        label,
                        baseline,
                        current,
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if start.elapsed() >= timeout {
                    break format!("{} stayed at {:?}", label, current);
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    break err.to_string();
                }
            }
        }

        sleep(interval).await;
    };

    Err(anyhow!(
        "timed out after {}ms waiting for firefox {} change from {:?}: {}",
        timeout_ms,
        label,
        baseline,
        last_error
    ))
}

pub async fn current_title() -> Result<String> {
    let window = crate::firefox::window::find_firefox_window(None)?;
    Ok(infer_tab_title_from_window(&window.name))
}

pub async fn current_url() -> Result<String> {
    retry::with_transient_retry(|| async {
        let window = crate::firefox::window::find_firefox_window(None)?;
        if let Ok(copied) = actions::context::read_address_bar_via_clipboard(&window.id) {
            let trimmed = copied.trim();
            let lowered = trimmed.to_ascii_lowercase();
            let looks_placeholder = ADDRESS_BAR_PLACEHOLDER_NAMES
                .iter()
                .any(|value| lowered == *value);
            if !trimmed.is_empty() && !looks_placeholder && url::is_plausible_url(trimmed) {
                return Ok(trimmed.to_string());
            }
        }

        let node = match actions::click::resolve_locator("address-bar").await {
            Ok(node) => node,
            Err(_) => {
                let inferred = infer_tab_title_from_window(&window.name);
                if url::is_plausible_url(&inferred) {
                    return Ok(inferred);
                }
                return Err(anyhow!("could not resolve firefox address bar locator"));
            }
        };
        let role = node.role.trim().to_ascii_lowercase();
        if let Some(text) = live_access::read_text(&node).await? {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let lowered = trimmed.to_ascii_lowercase();
                let looks_placeholder = ADDRESS_BAR_PLACEHOLDER_NAMES
                    .iter()
                    .any(|value| lowered == *value);
                if !looks_placeholder {
                    return Ok(trimmed.to_string());
                }
            }
        }

        if role == "entry" {
            for segment in node.path.iter().rev() {
                let trimmed = segment.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some((_, value)) = trimmed.split_once(':') {
                    let candidate = value.trim().trim_matches('"');
                    if candidate.is_empty() {
                        continue;
                    }
                    if url::is_plausible_url(candidate) {
                        return Ok(candidate.to_string());
                    }
                }
            }
        }

        if let Some(name) = node.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                let lowered = trimmed.to_ascii_lowercase();
                let looks_placeholder = ADDRESS_BAR_PLACEHOLDER_NAMES
                    .iter()
                    .any(|value| lowered == *value);
                if !looks_placeholder && url::is_plausible_url(trimmed) {
                    return Ok(trimmed.to_string());
                }
            }
        }

        if let Ok(current_tab) = actions::click::resolve_locator("current-tab").await {
            if let Some(name) = current_tab.name {
                let trimmed = name.trim();
                if url::is_plausible_url(trimmed) {
                    return Ok(trimmed.to_string());
                }
            }
        }

        if let Ok(tabs) = crate::firefox::tabs::model::resolve_tabs_with_current().await {
            let current = tabs
                .iter()
                .find(|tab| tab.is_current)
                .or_else(|| tabs.first());
            if let Some(tab) = current {
                let candidate = tab.title.trim();
                if url::is_plausible_url(candidate) {
                    return Ok(candidate.to_string());
                }
            }
        }

        let window = crate::firefox::window::find_firefox_window(None)?;
        let inferred = infer_tab_title_from_window(&window.name);
        if url::is_plausible_url(&inferred) {
            return Ok(inferred);
        }

        Err(anyhow!("could not read firefox address bar text"))
    })
    .await
    .or_else(|_| {
        crate::firefox::window::find_firefox_window(None).map(|window| {
            let inferred = infer_tab_title_from_window(&window.name);
            if url::is_plausible_url(&inferred) {
                inferred
            } else {
                "<unavailable>".to_string()
            }
        })
    })
}

fn infer_tab_title_from_window(window_title: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::infer_tab_title_from_window;

    #[test]
    fn infers_tab_title_from_window_title() {
        assert_eq!(
            infer_tab_title_from_window("about:blank - Profile 1 - Mozilla Firefox"),
            "about:blank"
        );
    }
}
