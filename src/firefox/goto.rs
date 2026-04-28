use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use tokio::time::sleep;

use crate::firefox::{actions, wait};
use crate::{firefox::window, window as x11_window};

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

    let window_before = window::find_firefox_window(None).ok();
    let title_before = wait::current_title().await.ok();
    let url_before = wait::current_url().await.ok();

    let navigation = match actions::type_text("address-bar", &url).await {
        Ok(type_summary) => {
            let enter_summary = actions::press_enter(Some("address-bar")).await?;
            let page_change = wait_for_page_change(
                window_before.as_ref(),
                title_before.as_deref(),
                url_before.as_deref(),
                Some(&url),
                timeout_ms,
                poll_ms,
            )
            .await?;
            let cleanup = dismiss_address_bar_dropdown().await?;
            format!(
                "{} | {} | {} | {}",
                type_summary, enter_summary, page_change, cleanup
            )
        }
        Err(err) => navigate_via_keyboard(&url, timeout_ms, poll_ms, &err.to_string()).await?,
    };

    Ok(match tab_summary {
        Some(tab_summary) => format!(
            "firefox goto {:?} in new tab | {} | {} | {}",
            url, locator_status, tab_summary, navigation
        ),
        None => format!(
            "firefox goto {:?} | {} | {}",
            url, locator_status, navigation
        ),
    })
}

pub async fn wait_for_page_change(
    window_before: Option<&crate::window::WindowMatch>,
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
            if urls_equivalent(&normalize_observed_url(after), expected) {
                return Ok(format!(
                    "firefox url reached {:?} after {}ms ({} attempts)",
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if let (Some(before), Some(after)) = (title_baseline.as_deref(), current_title.as_deref()) {
            if after != before {
                return Ok(format!(
                    "firefox title changed from {:?} to {:?} after {}ms ({} attempts)",
                    before,
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if let Some(before) = window_before {
            if let Ok(after_window) = window::find_firefox_window(None) {
                if after_window.name != before.name {
                    return Ok(format!(
                        "firefox window title changed from {:?} to {:?} after {}ms ({} attempts)",
                        before.name,
                        after_window.name,
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }
            }
        }

        if let (Some(before), Some(after)) = (url_baseline.as_deref(), current_url.as_deref()) {
            if after != before {
                return Ok(format!(
                    "firefox url changed from {:?} to {:?} after {}ms ({} attempts)",
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
                "timed out after {}ms waiting for firefox page title/url change: title={:?}, url={:?}",
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
        || trimmed.starts_with("firefox:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    }
}

fn urls_equivalent(observed: &str, expected: &str) -> bool {
    if observed == expected {
        return true;
    }

    fn canonical(url: &str) -> String {
        let trimmed = url.trim().trim_end_matches('/').to_ascii_lowercase();
        let without_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .unwrap_or(trimmed.as_str());
        without_scheme
            .strip_prefix("www.")
            .unwrap_or(without_scheme)
            .to_string()
    }

    canonical(observed) == canonical(expected)
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
        || trimmed.starts_with("firefox:")
        || trimmed.starts_with("moz-extension:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}

fn open_new_tab_via_keyboard(reason: &str) -> Result<String> {
    let firefox_window = window::find_firefox_window(None)?;
    let activation_note =
        crate::firefox::actions::context::activate_window_note(&firefox_window.id);
    x11_window::send_key(&firefox_window.id, "ctrl+t")?;
    Ok(format!(
        "opened new tab via keyboard fallback ctrl+t in window {} ({}, locator click failed: {})",
        firefox_window.id, activation_note, reason
    ))
}

async fn navigate_via_keyboard(
    url: &str,
    timeout_ms: u64,
    poll_ms: u64,
    reason: &str,
) -> Result<String> {
    let firefox_window = window::find_firefox_window(None)?;
    let title_before = firefox_window.name.clone();
    let url_before = crate::firefox::wait::current_url().await.ok();
    let activation_note =
        crate::firefox::actions::context::activate_window_note(&firefox_window.id);
    x11_window::send_key(&firefox_window.id, "ctrl+l")?;
    x11_window::type_text(&firefox_window.id, url)?;
    x11_window::send_key(&firefox_window.id, "Return")?;
    let page_change = wait_for_page_change(
        None,
        Some(&title_before),
        url_before.as_deref(),
        Some(url),
        timeout_ms,
        poll_ms,
    )
    .await?;
    let cleanup = dismiss_address_bar_dropdown().await?;
    Ok(format!(
        "typed via keyboard fallback in window {} ({}, locator typing failed: {}) | pressed Enter on targeted window | {} | {}",
        firefox_window.id, activation_note, reason, page_change, cleanup
    ))
}

async fn dismiss_address_bar_dropdown() -> Result<String> {
    let firefox_window = window::find_firefox_window(None)?;
    let activation_note =
        crate::firefox::actions::context::activate_window_note(&firefox_window.id);
    x11_window::send_key(&firefox_window.id, "Escape")?;
    sleep(Duration::from_millis(120)).await;
    Ok(format!(
        "dismissed Firefox address-bar dropdown in window {} ({})",
        firefox_window.id, activation_note
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
        assert_eq!(
            normalize_url("firefox://settings").unwrap(),
            "firefox://settings"
        );
    }

    #[test]
    fn normalizes_observed_address_bar_urls() {
        assert_eq!(
            normalize_observed_url("127.0.0.1:8124/index.html?run=abc"),
            "http://127.0.0.1:8124/index.html?run=abc"
        );
    }

    #[test]
    fn accepts_moz_extension_urls() {
        assert_eq!(
            normalize_url("moz-extension://abc/options.html").unwrap(),
            "moz-extension://abc/options.html"
        );
    }

    #[test]
    fn treats_www_and_trailing_slash_as_equivalent() {
        assert!(super::urls_equivalent(
            "https://www.google.com/",
            "https://google.com"
        ));
    }
}
