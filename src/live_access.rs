use anyhow::{Context, Result};
use atspi::{
    AccessibilityConnection, CoordType, ScrollType,
    proxy::{accessible::ObjectRefExt, proxy_ext::ProxyExt},
};

use crate::model::LiveNode;

pub async fn grab_focus(node: &LiveNode) -> Result<bool> {
    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for focus lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;
    let component = match proxies.component().await {
        Ok(component) => component,
        Err(_) => return Ok(false),
    };

    match component.grab_focus().await {
        Ok(focused) => Ok(focused),
        Err(_) => Ok(false),
    }
}

pub async fn scroll_into_view(node: &LiveNode) -> Result<bool> {
    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for scroll lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;

    if let Ok(component) = proxies.component().await {
        if let Ok(scrolled) = component.scroll_to(ScrollType::Anywhere).await {
            if scrolled {
                return Ok(true);
            }
        }

        if let Ok((x, y, width, height)) = component.get_extents(CoordType::Screen).await {
            let target_x = x + (width / 2).max(1);
            let target_y = y + (height / 2).max(1);
            if let Ok(scrolled) = component
                .scroll_to_point(CoordType::Screen, target_x, target_y)
                .await
            {
                if scrolled {
                    return Ok(true);
                }
            }
        }
    }

    if let Ok(text) = proxies.text().await {
        if let Ok(count) = text.character_count().await {
            if count > 0 {
                if let Ok(scrolled) = text.scroll_substring_to(0, count, 6).await {
                    if scrolled {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

pub async fn read_text(node: &LiveNode) -> Result<Option<String>> {
    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for text lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;
    let text = match proxies.text().await {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };

    let count = text
        .character_count()
        .await
        .context("failed to read text character count")?;
    let value = text
        .get_text(0, count)
        .await
        .context("failed to read text contents from AT-SPI text interface")?;
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }

    Ok(Some(normalized))
}

pub async fn set_text(node: &LiveNode, text: &str) -> Result<bool> {
    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for editable-text lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;
    let editable = match proxies.editable_text().await {
        Ok(editable) => editable,
        Err(_) => return Ok(false),
    };

    let replaced = editable
        .set_text_contents(text)
        .await
        .context("failed to set AT-SPI editable text contents")?;
    if replaced {
        return Ok(true);
    }

    let text_proxy = match proxies.text().await {
        Ok(text_proxy) => text_proxy,
        Err(_) => return Ok(false),
    };

    let count = text_proxy
        .character_count()
        .await
        .context("failed to read text length before insert")?;
    let inserted = editable
        .insert_text(
            count,
            text,
            i32::try_from(text.chars().count()).unwrap_or(i32::MAX),
        )
        .await
        .context("failed to append AT-SPI editable text contents")?;

    Ok(inserted)
}
