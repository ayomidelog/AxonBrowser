use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use tokio::time::sleep;

use crate::firefox::{bidi, launch::BrowserFlavor, session, wait};

pub async fn navigate(
    flavor: BrowserFlavor,
    raw_url: &str,
    new_tab: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let url = normalize_url(raw_url)?;

    let title_before = wait::current_title().await.ok();
    let url_before = wait::current_url().await.ok();
    let navigation = if new_tab {
        match flavor {
            BrowserFlavor::Camoufox => {
                let context = bidi::new_tab_via_window_open("about:blank").await?;
                let summary = format!(
                    "camoufox opened real tab {:?} ({}) via window.open",
                    context.title, context.url
                );
                let context = bidi::navigate_context(&context.context, &url).await?;
                format!(
                    "{} | bidi navigated tab {:?} to {:?}",
                    summary, context.title, url
                )
            }
            BrowserFlavor::Firefox => {
                let context = bidi::new_context(&url).await?;
                format!("bidi created tab {:?} for {:?}", context.title, url)
            }
        }
    } else {
        let context = bidi::navigate(&url, title_before.as_deref(), url_before.as_deref()).await?;
        format!("bidi navigated tab {:?} to {:?}", context.title, url)
    };
    let page_change = wait_for_page_change(
        title_before.as_deref(),
        url_before.as_deref(),
        Some(&url),
        timeout_ms,
        poll_ms,
    )
    .await?;
    let _ = session::remember_browser_url(&url);

    Ok(if new_tab {
        format!(
            "firefox goto {:?} in new tab | {} | {}",
            url, navigation, page_change
        )
    } else {
        format!("firefox goto {:?} | {} | {}", url, navigation, page_change)
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
    let expected_url = expected_url.map(canonical_url);

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

        if let (Some(expected), Some(after)) = (expected_url.as_deref(), current_url.as_deref())
            && canonical_url(after) == expected
        {
            return Ok(format!(
                "firefox url reached {:?} after {}ms ({} attempts)",
                after,
                start.elapsed().as_millis(),
                attempts
            ));
        }

        if let (Some(before), Some(after)) = (title_baseline.as_deref(), current_title.as_deref())
            && after != before
        {
            return Ok(format!(
                "firefox title changed from {:?} to {:?} after {}ms ({} attempts)",
                before,
                after,
                start.elapsed().as_millis(),
                attempts
            ));
        }

        if let (Some(before), Some(after)) = (url_baseline.as_deref(), current_url.as_deref())
            && canonical_url(after) != canonical_url(before)
        {
            return Ok(format!(
                "firefox url changed from {:?} to {:?} after {}ms ({} attempts)",
                before,
                after,
                start.elapsed().as_millis(),
                attempts
            ));
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

pub async fn open_new_tab(
    flavor: BrowserFlavor,
    _timeout_ms: u64,
    _poll_ms: u64,
) -> Result<String> {
    match flavor {
        BrowserFlavor::Firefox => {
            let context = bidi::new_context("about:blank").await?;
            Ok(format!(
                "opened firefox new tab {:?} ({})",
                context.title, context.url
            ))
        }
        BrowserFlavor::Camoufox => {
            let context = bidi::new_tab_via_window_open("about:blank").await?;
            Ok(format!(
                "opened camoufox new tab {:?} ({})",
                context.title, context.url
            ))
        }
    }
}

fn canonical_url(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("about:")
        || lower.starts_with("firefox:")
        || lower.starts_with("moz-extension:")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
    {
        lower
    } else if let Some(rest) = lower.strip_prefix("http://") {
        rest.to_string()
    } else if let Some(rest) = lower.strip_prefix("https://") {
        rest.to_string()
    } else {
        lower
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
        || trimmed.starts_with("firefox:")
        || trimmed.starts_with("moz-extension:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
    {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}
