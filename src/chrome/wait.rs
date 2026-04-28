use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::time::sleep;

use crate::{
    chrome::{locators, retry},
    live_access,
};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_POLL_MS: u64 = 200;

pub fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

pub fn default_poll_ms() -> u64 {
    DEFAULT_POLL_MS
}

pub async fn wait_for_locator(locator_raw: &str, timeout_ms: u64, poll_ms: u64) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    let last_error = loop {
        attempts += 1;
        match locators::locate(locator_raw).await {
            Ok(located) => {
                return Ok(format!(
                    "chrome locator {} matched {} after {}ms ({} attempts)",
                    located.locator.canonical_name(),
                    located.node.line_label(),
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
        "timed out after {}ms waiting for chrome locator {:?}: {}",
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
                        "chrome {} changed from {:?} to {:?} after {}ms ({} attempts)",
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
        "timed out after {}ms waiting for chrome {} change from {:?}: {}",
        timeout_ms,
        label,
        baseline,
        last_error
    ))
}

pub async fn current_title() -> Result<String> {
    retry::with_transient_retry(|| async {
        let node = locators::resolve(locators::ChromeLocator::CurrentTab).await?;
        let fallback = node.line_label();
        Ok(node
            .name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback))
    })
    .await
}

pub async fn current_url() -> Result<String> {
    retry::with_transient_retry(|| async {
        let node = locators::resolve(locators::ChromeLocator::AddressBar).await?;
        if let Some(text) = live_access::read_text(&node).await? {
            if !text.is_empty() {
                return Ok(text);
            }
        }

        if let Some(name) = node.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() && trimmed != "Address and search bar" {
                return Ok(trimmed.to_string());
            }
        }

        Err(anyhow!("could not read chrome address bar text"))
    })
    .await
}
