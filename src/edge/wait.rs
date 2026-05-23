use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::time::sleep;

use crate::{chrome::retry, edge::actions, edge::devtools};

pub async fn wait_for_locator(locator_raw: &str, timeout_ms: u64, poll_ms: u64) -> Result<String> {
    if matches!(
        locator_raw.trim().to_ascii_lowercase().as_str(),
        "address" | "address bar" | "addressbar" | "address-bar" | "omnibox"
    ) {
        let window = crate::edge::window::find_edge_window(None)?;
        return Ok(format!(
            "edge locator {:?} available via browser shortcut in window {} ({})",
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
                    "edge locator {:?} matched {} after {}ms ({} attempts)",
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
        "timed out after {}ms waiting for edge locator {:?}: {}",
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
                        "edge {} changed from {:?} to {:?} after {}ms ({} attempts)",
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
        "timed out after {}ms waiting for edge {} change from {:?}: {}",
        timeout_ms,
        label,
        baseline,
        last_error
    ))
}

pub async fn current_title() -> Result<String> {
    retry::with_transient_retry(|| async {
        if let Ok(Some(title)) = devtools::current_title() {
            let trimmed = title.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        Err(anyhow!("could not read edge title from DevTools page list"))
    })
    .await
}

pub async fn current_url() -> Result<String> {
    retry::with_transient_retry(|| async {
        if let Ok(Some(url)) = devtools::current_url() {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        Err(anyhow!("could not read edge URL from DevTools page list"))
    })
    .await
}
