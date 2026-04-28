use anyhow::{Context, Result, anyhow};
use atspi::proxy::{accessible::ObjectRefExt, proxy_ext::ProxyExt};

use crate::{inspect, model::LiveNode, window};

use super::context::{self, ActionTarget};

pub async fn click(locator_raw: &str) -> Result<String> {
    let target = context::resolve_target(locator_raw).await?;
    click_target(&target).await
}

pub async fn click_target(target: &ActionTarget) -> Result<String> {
    click_target_node(&target.node, &target.label, &target.path).await
}

pub async fn click_target_node(node: &LiveNode, label: &str, path: &str) -> Result<String> {
    if invoke_default_action(node).await? {
        return Ok(format!("clicked {} via AT-SPI action ({})", label, path));
    }

    let (screen_x, screen_y) = inspect::clickable_point(node).await?;
    let browser_window = window::find_window_at_point(screen_x, screen_y)?;
    let relative_x = screen_x - browser_window.x;
    let relative_y = screen_y - browser_window.y;
    if relative_x < 0 || relative_y < 0 {
        return Err(anyhow!(
            "resolved click point landed outside the target window"
        ));
    }

    let activation_note = context::activate_window_note(&browser_window.id);
    window::mousemove_click(&browser_window.id, relative_x, relative_y)?;

    Ok(format!(
        "clicked {} via X11 at {},{} in window {} ({}, {})",
        label, relative_x, relative_y, browser_window.id, path, activation_note
    ))
}

async fn invoke_default_action(node: &LiveNode) -> Result<bool> {
    let connection = atspi::AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for action lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;

    let action = match proxies.action().await {
        Ok(action) => action,
        Err(_) => return Ok(false),
    };

    match action.do_action(0).await {
        Ok(invoked) => Ok(invoked),
        Err(_) => Ok(false),
    }
}
