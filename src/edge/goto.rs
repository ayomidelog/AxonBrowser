use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use tokio::time::sleep;

use crate::edge::{actions, wait};
use crate::{edge::window, window as x11_window};

pub async fn navigate(
    raw_url: &str,
    new_tab: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let url = normalize_url(raw_url)?;
    let locator_status = wait::wait_for_locator("address-bar", timeout_ms, poll_ms)
        .await
        .unwrap_or_else(|err| format!("address-bar locator unavailable ({})", err));

    let tab_summary = if new_tab {
        Some(match actions::click("new-tab").await {
            Ok(summary) => summary,
            Err(err) => open_new_tab_via_keyboard(&err.to_string())?,
        })
    } else {
        None
    };

    let title_before = wait::current_title().await.ok();
    let url_before = wait::current_url().await.ok();

    let navigation = match actions::type_text("address-bar", &url).await {
        Ok(type_summary) => {
            let enter_summary = actions::press_enter(Some("address-bar")).await?;
            let page_change = wait_for_page_change(
                title_before.as_deref(),
                url_before.as_deref(),
                Some(&url),
                timeout_ms,
                poll_ms,
            )
            .await?;
            format!("{} | {} | {}", type_summary, enter_summary, page_change)
        }
        Err(err) => navigate_via_keyboard(&url, timeout_ms, poll_ms, &err.to_string()).await?,
    };

    Ok(match tab_summary {
        Some(tab_summary) => format!(
            "edge goto {:?} in new tab | {} | {} | {}",
            url, locator_status, tab_summary, navigation
        ),
        None => format!("edge goto {:?} | {} | {}", url, locator_status, navigation),
    })
}

pub async fn wait_for_page_change(
    title_before: Option<&str>,
    url_before: Option<&str>,
    expected_url: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let title_baseline = title_before.map(str::to_owned);
    let url_baseline = url_before.map(str::to_owned);
    let expected_url = expected_url.map(normalize_observed_url);

    if title_baseline.is_none() && url_baseline.is_none() && expected_url.is_none() {
        bail!("could not capture a baseline page title or URL before navigation")
    }

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;
    loop {
        attempts += 1;

        let current_title = wait::current_title().await.ok();
        let current_url = wait::current_url().await.ok();

        if let (Some(expected), Some(after)) = (expected_url.as_deref(), current_url.as_deref()) {
            if normalize_observed_url(after) == expected {
                return Ok(format!(
                    "edge url reached {:?} after {}ms ({} attempts)",
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if let (Some(before), Some(after)) = (title_baseline.as_deref(), current_title.as_deref()) {
            if after != before {
                return Ok(format!(
                    "edge title changed from {:?} to {:?} after {}ms ({} attempts)",
                    before,
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if let (Some(before), Some(after)) = (url_baseline.as_deref(), current_url.as_deref()) {
            if after != before {
                return Ok(format!(
                    "edge url changed from {:?} to {:?} after {}ms ({} attempts)",
                    before,
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if start.elapsed() >= timeout {
            let title_now = current_title.as_deref().unwrap_or("<unavailable>");
            let url_now = current_url.as_deref().unwrap_or("<unavailable>");
            return Err(anyhow!(
                "timed out after {}ms waiting for edge page title/url change: title={:?}, url={:?}",
                timeout_ms,
                title_now,
                url_now
            ));
        }

        sleep(interval).await;
    }
}

fn normalize_observed_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.contains("://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("edge:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    }
}

fn normalize_url(raw_url: &str) -> Result<String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        bail!("url must not be empty");
    }
    if trimmed.contains(char::is_whitespace) {
        bail!("url must not contain whitespace: {:?}", raw_url);
    }
    if trimmed.contains("://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("edge:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}

fn open_new_tab_via_keyboard(reason: &str) -> Result<String> {
    let edge_window = window::find_edge_window(None)?;
    let activation_note = crate::edge::actions::context::activate_window_note(&edge_window.id);
    x11_window::send_key(&edge_window.id, "ctrl+t")?;
    Ok(format!(
        "opened new tab via keyboard fallback ctrl+t in window {} ({}, locator click failed: {})",
        edge_window.id, activation_note, reason
    ))
}

async fn navigate_via_keyboard(
    url: &str,
    timeout_ms: u64,
    poll_ms: u64,
    reason: &str,
) -> Result<String> {
    let edge_window = window::find_edge_window(None)?;
    let title_before = edge_window.name.clone();
    let url_before = crate::edge::wait::current_url().await.ok();
    let activation_note = crate::edge::actions::context::activate_window_note(&edge_window.id);
    x11_window::send_key(&edge_window.id, "ctrl+l")?;
    x11_window::type_text(&edge_window.id, url)?;
    x11_window::send_key(&edge_window.id, "Return")?;
    let page_change = wait_for_page_change(
        Some(&title_before),
        url_before.as_deref(),
        Some(url),
        timeout_ms,
        poll_ms,
    )
    .await?;
    Ok(format!(
        "typed via keyboard fallback in window {} ({}, locator typing failed: {}) | pressed Enter on targeted window | {}",
        edge_window.id, activation_note, reason, page_change
    ))
}

#[cfg(test)]
mod tests {
    use super::{normalize_observed_url, normalize_url};

    #[test]
    fn prefixes_https_for_bare_hosts() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com");
    }

    #[test]
    fn preserves_existing_scheme() {
        assert_eq!(
            normalize_url("https://example.com").unwrap(),
            "https://example.com"
        );
        assert_eq!(normalize_url("about:blank").unwrap(), "about:blank");
        assert_eq!(normalize_url("edge://settings").unwrap(), "edge://settings");
    }

    #[test]
    fn normalizes_observed_address_bar_urls() {
        assert_eq!(
            normalize_observed_url("127.0.0.1:8124/index.html?run=abc"),
            "http://127.0.0.1:8124/index.html?run=abc"
        );
    }
}
