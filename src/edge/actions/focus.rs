use anyhow::Result;

use crate::chrome::locators;

use super::{click, context};

pub async fn focus(locator_raw: &str) -> Result<String> {
    if locators::ChromeLocator::parse(locator_raw)? == locators::ChromeLocator::AddressBar {
        let browser_window = crate::edge::window::find_edge_window(None)?;
        context::focus_address_bar(&browser_window.id)?;
        return Ok(format!(
            "focused address bar in edge window {} ({})",
            browser_window.id, browser_window.name
        ));
    }

    let target = context::resolve_target(locator_raw).await?;

    if locators::ChromeLocator::parse(locator_raw)? == locators::ChromeLocator::Window {
        let browser_window = crate::edge::window::find_edge_window(None)?;
        let activation_note = context::activate_window_note(&browser_window.id);
        return Ok(format!(
            "focused browser window {} ({}, {})",
            browser_window.id, browser_window.name, activation_note
        ));
    }

    if target.try_grab_focus().await? {
        return Ok(format!(
            "focused {} via AT-SPI grab-focus ({})",
            target.label, target.path
        ));
    }

    click::click_target(&target).await.map(|summary| {
        format!(
            "focused {} via click fallback ({}) | {}",
            target.label, target.path, summary
        )
    })
}
