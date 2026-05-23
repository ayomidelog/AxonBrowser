use anyhow::{Result, bail};

use crate::firefox::{goto, wait as firefox_wait};

use super::{actions, root::PageScope, wait};

/// Wait request to apply after a page action completes.
#[derive(Debug, Clone, Copy)]
pub struct PostActionWait<'a> {
    pub scope: &'a PageScope,
    pub selectors: &'a [String],
    pub conditions: wait::PageWaitConditions<'a>,
    pub timing: wait::PageWaitTiming,
}

pub async fn click_and_wait(
    action_scope: &PageScope,
    action_selectors: &[String],
    wait_request: PostActionWait<'_>,
) -> Result<String> {
    let title_before = firefox_wait::current_title().await.ok();
    let url_before = firefox_wait::current_url().await.ok();
    let action_summary = actions::click(action_scope, action_selectors, None).await?;
    let wait_summary =
        wait_after_action(wait_request, title_before.as_deref(), url_before.as_deref()).await?;

    Ok(format!(
        "page click-and-wait | {} | {}",
        action_summary, wait_summary
    ))
}

pub async fn submit_and_wait(
    action_scope: &PageScope,
    action_selectors: &[String],
    wait_request: PostActionWait<'_>,
) -> Result<String> {
    let title_before = firefox_wait::current_title().await.ok();
    let url_before = firefox_wait::current_url().await.ok();
    let action_summary = actions::press_enter(action_scope, action_selectors).await?;
    let wait_summary =
        wait_after_action(wait_request, title_before.as_deref(), url_before.as_deref()).await?;

    Ok(format!(
        "page submit-and-wait | {} | {}",
        action_summary, wait_summary
    ))
}

async fn wait_after_action(
    wait_request: PostActionWait<'_>,
    title_before: Option<&str>,
    url_before: Option<&str>,
) -> Result<String> {
    if let Some(summary) = wait::wait_for_optional_target(
        wait_request.scope,
        wait_request.selectors,
        wait_request.conditions,
        wait_request.timing,
    )
    .await?
    {
        return Ok(summary);
    }

    if wait_request.conditions.disappear {
        bail!(
            "--disappear needs --text, --title-contains, --url-contains, or one or more --wait-selector values"
        );
    }

    goto::wait_for_page_change(
        title_before,
        url_before,
        None,
        wait_request.timing.timeout_ms,
        wait_request.timing.poll_ms,
    )
    .await
}
