use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use tokio::time::sleep;

use crate::chrome::{actions, wait};

pub async fn navigate(
    raw_url: &str,
    new_tab: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let url = normalize_url(raw_url)?;

    let tab_summary = if new_tab {
        Some(actions::click("new-tab").await?)
    } else {
        None
    };

    let title_before = wait::current_title().await.ok();
    let url_before = wait::current_url().await.ok();

    let type_summary = actions::type_text("address-bar", &url).await?;
    let enter_summary = actions::press_enter(Some("address-bar")).await?;
    let page_change = wait_for_page_change(
        title_before.as_deref(),
        url_before.as_deref(),
        Some(&url),
        timeout_ms,
        poll_ms,
    )
    .await?;

    Ok(match tab_summary {
        Some(tab_summary) => format!(
            "chrome goto {:?} in new tab | {} | {} | {} | {}",
            url, tab_summary, type_summary, enter_summary, page_change
        ),
        None => format!(
            "chrome goto {:?} | {} | {} | {}",
            url, type_summary, enter_summary, page_change
        ),
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
                    "chrome url reached {:?} after {}ms ({} attempts)",
                    after,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        if let (Some(before), Some(after)) = (title_baseline.as_deref(), current_title.as_deref()) {
            if after != before {
                return Ok(format!(
                    "chrome title changed from {:?} to {:?} after {}ms ({} attempts)",
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
                    "chrome url changed from {:?} to {:?} after {}ms ({} attempts)",
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
                "timed out after {}ms waiting for chrome page title/url change: title={:?}, url={:?}",
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
        || trimmed.starts_with("chrome:")
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
        || trimmed.starts_with("chrome:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
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
    }

    #[test]
    fn normalizes_observed_address_bar_urls() {
        assert_eq!(
            normalize_observed_url("127.0.0.1:8124/index.html?run=abc"),
            "http://127.0.0.1:8124/index.html?run=abc"
        );
    }
}
