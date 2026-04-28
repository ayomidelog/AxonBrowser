use anyhow::Result;

use crate::window;

use super::{context, focus};

pub async fn type_text(locator_raw: &str, text: &str) -> Result<String> {
    let target = context::resolve_target(locator_raw).await?;
    let focus_summary = focus::focus(locator_raw).await?;
    if target.try_set_text(text).await? {
        return Ok(format!(
            "typed into {} via AT-SPI editable text ({}) | {}",
            target.label, target.path, focus_summary
        ));
    }

    let browser_window = target.browser_window().await?;
    let activation_note = context::activate_window_note(&browser_window.id);
    let input_mode = if target.is_address_bar() {
        context::type_via_clipboard(&browser_window.id, text)?;
        "clipboard paste"
    } else {
        window::type_text(&browser_window.id, text)?;
        "X11 key injection"
    };

    Ok(format!(
        "typed into {} via {} in window {} ({}, {}, focus: {})",
        target.label, input_mode, browser_window.id, target.path, activation_note, focus_summary
    ))
}
