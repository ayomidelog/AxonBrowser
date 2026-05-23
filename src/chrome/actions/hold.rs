use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use crate::{
    chrome::{locators, window as chrome_window},
    window,
};

use super::{context, focus};

pub async fn hold(key: &str, duration_ms: u64, locator_raw: Option<&str>) -> Result<String> {
    let browser_window = match locator_raw {
        Some(locator) => match focus::focus(locator).await {
            Ok(_) => match locators::locate(locator).await {
                Ok(located) => context::browser_window_for_node(&located.node).await?,
                Err(_) => chrome_window::find_browser_window(None)?,
            },
            Err(_) => chrome_window::find_browser_window(None)?,
        },
        None => chrome_window::find_browser_window(None)?,
    };

    let activation_note = context::activate_window_note(&browser_window.id);
    let key_window = active_or_browser_window(&browser_window)?;
    window::key_down(&key_window.id, key)?;
    sleep(Duration::from_millis(duration_ms)).await;
    window::key_up(&key_window.id, key)?;

    Ok(format!(
        "held {} in window {} for {}ms ({})",
        key, browser_window.id, duration_ms, activation_note
    ))
}

fn active_or_browser_window(
    browser_window: &crate::window::WindowMatch,
) -> Result<crate::window::WindowMatch> {
    crate::window::active_window_id()
        .ok()
        .and_then(|id| {
            crate::window::list_visible_windows()
                .ok()?
                .into_iter()
                .find(|window| window.id == id && window.id == browser_window.id)
        })
        .or_else(|| Some(browser_window.clone()))
        .ok_or_else(|| anyhow::anyhow!("failed to resolve browser window for key hold"))
}
