use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::time::sleep;

use crate::edge::window as edge_window;

use super::model::{TabInfo, resolve_tabs_with_current};

pub async fn wait_for_current_tab(
    target_id: &str,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<TabInfo> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));

    let last_state = loop {
        match resolve_tabs_with_current().await {
            Ok(tabs) => {
                if let Some(tab) = tabs
                    .iter()
                    .find(|tab| tab.id == target_id && tab.is_current)
                {
                    return Ok(tab.clone());
                }

                if start.elapsed() >= timeout {
                    break render_tabs_state(&tabs, target_id);
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
        "timed out after {}ms waiting for edge tab {} to become current: {}",
        timeout_ms,
        target_id,
        last_state
    ))
}

pub async fn wait_for_close_recovery(
    closed_target_id: &str,
    previous_count: usize,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    let last_state = loop {
        attempts += 1;
        match resolve_tabs_with_current().await {
            Ok(tabs) => {
                if tabs.iter().all(|tab| tab.id != closed_target_id) || tabs.len() != previous_count
                {
                    return Ok(format!(
                        "edge tabs recovered after close; {} tabs visible after {}ms ({} attempts)",
                        tabs.len(),
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if start.elapsed() >= timeout {
                    break format!(
                        "target {} still present; tab count still {}",
                        closed_target_id,
                        tabs.len()
                    );
                }
            }
            Err(err) => {
                if edge_window::find_edge_window(None).is_err() {
                    return Ok(format!(
                        "edge window closed after tab close after {}ms ({} attempts)",
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if err.to_string().contains("no edge tabs matched")
                    || err.to_string().contains("no chrome tabs matched")
                {
                    if start.elapsed() >= timeout {
                        break err.to_string();
                    }
                    sleep(interval).await;
                    continue;
                }

                if start.elapsed() >= timeout {
                    break err.to_string();
                }
            }
        }

        sleep(interval).await;
    };

    Err(anyhow!(
        "timed out after {}ms waiting for edge tabs to recover after close: {}",
        timeout_ms,
        last_state
    ))
}

fn render_tabs_state(tabs: &[TabInfo], target_id: &str) -> String {
    let summary = tabs
        .iter()
        .map(|tab| {
            format!(
                "{}{}:{:?}({})",
                if tab.is_current { "*" } else { "" },
                tab.index,
                tab.title,
                tab.id
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("target {} not current; saw [{}]", target_id, summary)
}
