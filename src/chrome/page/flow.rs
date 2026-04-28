use anyhow::{Result, bail};

use crate::chrome::{goto, wait as chrome_wait};

use super::{actions, root::PageScope, wait};

pub async fn click_and_wait(
    action_scope: &PageScope,
    action_selectors: &[String],
    wait_scope: &PageScope,
    wait_selectors: &[String],
    text: Option<&str>,
    title_contains: Option<&str>,
    url_contains: Option<&str>,
    disappear: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let title_before = chrome_wait::current_title().await.ok();
    let url_before = chrome_wait::current_url().await.ok();
    let action_summary = actions::click(action_scope, action_selectors, None).await?;
    let wait_summary = wait_after_action(
        wait_scope,
        wait_selectors,
        text,
        title_contains,
        url_contains,
        disappear,
        title_before.as_deref(),
        url_before.as_deref(),
        timeout_ms,
        poll_ms,
    )
    .await?;

    Ok(format!(
        "page click-and-wait | {} | {}",
        action_summary, wait_summary
    ))
}

pub async fn submit_and_wait(
    action_scope: &PageScope,
    action_selectors: &[String],
    wait_scope: &PageScope,
    wait_selectors: &[String],
    text: Option<&str>,
    title_contains: Option<&str>,
    url_contains: Option<&str>,
    disappear: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let title_before = chrome_wait::current_title().await.ok();
    let url_before = chrome_wait::current_url().await.ok();
    let action_summary = actions::press_enter(action_scope, action_selectors).await?;
    let wait_summary = wait_after_action(
        wait_scope,
        wait_selectors,
        text,
        title_contains,
        url_contains,
        disappear,
        title_before.as_deref(),
        url_before.as_deref(),
        timeout_ms,
        poll_ms,
    )
    .await?;

    Ok(format!(
        "page submit-and-wait | {} | {}",
        action_summary, wait_summary
    ))
}

async fn wait_after_action(
    wait_scope: &PageScope,
    wait_selectors: &[String],
    text: Option<&str>,
    title_contains: Option<&str>,
    url_contains: Option<&str>,
    disappear: bool,
    title_before: Option<&str>,
    url_before: Option<&str>,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    if let Some(summary) = wait::wait_for_optional_target(
        wait_scope,
        wait_selectors,
        text,
        title_contains,
        url_contains,
        disappear,
        timeout_ms,
        poll_ms,
    )
    .await?
    {
        return Ok(summary);
    }

    if disappear {
        bail!(
            "--disappear needs --text, --title-contains, --url-contains, or one or more --wait-selector values"
        );
    }

    goto::wait_for_page_change(title_before, url_before, None, timeout_ms, poll_ms).await
}
