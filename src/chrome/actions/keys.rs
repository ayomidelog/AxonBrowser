use anyhow::{Context, Result};

use crate::{chrome::window as chrome_window, window};

use super::{context, focus};

pub async fn press_key(locator_raw: &str, key: &str) -> Result<String> {
    let target = context::resolve_target(locator_raw).await?;
    let focus_summary = focus::focus(locator_raw).await?;
    let browser_window = target.browser_window().await?;
    let activation_note = context::activate_window_note(&browser_window.id);
    window::send_key(&browser_window.id, key)?;

    Ok(format!(
        "pressed {} on {} in window {} ({}, {}, focus: {})",
        key, target.label, browser_window.id, target.path, activation_note, focus_summary
    ))
}

pub async fn press_enter(locator_raw: Option<&str>) -> Result<String> {
    let target = locator_raw.unwrap_or("address-bar");
    match press_key(target, "Return").await {
        Ok(summary) => Ok(summary),
        Err(primary_err) => {
            let browser_window = chrome_window::find_browser_window(None)
                .or_else(|_| window::find_window_by_title_contains("google chrome"))
                .with_context(|| {
                    format!(
                        "failed to resolve {target:?} for Enter and could not find a browser window fallback"
                    )
                })?;
            let activation_note = context::activate_window_note(&browser_window.id);
            window::send_key(&browser_window.id, "Return")?;
            Ok(format!(
                "pressed Return via browser-window fallback {} ({}, original locator path failed: {})",
                browser_window.id, activation_note, primary_err
            ))
        }
    }
}
