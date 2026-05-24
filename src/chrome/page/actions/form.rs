use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use atspi::State;
use tokio::time::sleep;

use crate::{
    chrome::{actions::click::click_target_node, page::root::PageScope},
    selector,
};

use super::target::PageActionTarget;

pub async fn check(scope: &PageScope, raw_selectors: &[String]) -> Result<String> {
    set_toggle(scope, raw_selectors, true).await
}

pub async fn uncheck(scope: &PageScope, raw_selectors: &[String]) -> Result<String> {
    set_toggle(scope, raw_selectors, false).await
}

pub async fn select_option(
    scope: &PageScope,
    raw_selectors: &[String],
    option: &str,
) -> Result<String> {
    let target = PageActionTarget::resolve(scope, raw_selectors).await?;
    let mut notes = Vec::new();
    if target.scroll_into_view().await? {
        notes.push("scrolled into view first".to_string());
    }

    let open_summary = click_target_node(&target.node, &target.label, &target.path).await?;
    let option_selector = selector::Selector::parse(&format!("~{}", option))?;
    let option_node = crate::chrome::page::root::resolve_in_page_scope(scope, &[option_selector])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no page option matched {:?}", option))?;
    let option_label = option_node.line_label();
    let option_path = option_node.path.join(" > ");
    let select_summary = click_target_node(&option_node, &option_label, &option_path).await?;

    Ok(attach_notes(
        format!(
            "selected option {:?} via {} | {}",
            option, open_summary, select_summary
        ),
        &notes,
    ))
}

async fn set_toggle(
    scope: &PageScope,
    raw_selectors: &[String],
    desired_checked: bool,
) -> Result<String> {
    let target = PageActionTarget::resolve(scope, raw_selectors).await?;
    let mut notes = Vec::new();
    if target.scroll_into_view().await? {
        notes.push("scrolled into view first".to_string());
    }

    let states = target.state_set().await?;
    let currently_checked = is_checked(states);
    if currently_checked == desired_checked {
        let state = if desired_checked {
            "already checked"
        } else {
            "already unchecked"
        };
        return Ok(attach_notes(
            format!("{} {} ({})", state, target.label, target.path),
            &notes,
        ));
    }

    let action_summary = click_target_node(&target.node, &target.label, &target.path).await?;
    if !wait_for_checked_state(scope, raw_selectors, desired_checked).await? {
        bail!(
            "toggle state for {} did not change to {}",
            target.label,
            if desired_checked {
                "checked"
            } else {
                "unchecked"
            }
        );
    }

    Ok(attach_notes(
        format!(
            "set {} {} ({}) | {}",
            if desired_checked {
                "checked"
            } else {
                "unchecked"
            },
            target.label,
            target.path,
            action_summary
        ),
        &notes,
    ))
}

fn is_checked(states: atspi::StateSet) -> bool {
    states.contains(State::Checked)
        || states.contains(State::Selected)
        || states.contains(State::Pressed)
}

async fn wait_for_checked_state(
    scope: &PageScope,
    raw_selectors: &[String],
    desired_checked: bool,
) -> Result<bool> {
    for _ in 0..15 {
        let target = PageActionTarget::resolve(scope, raw_selectors).await?;
        if is_checked(target.state_set().await?) == desired_checked {
            return Ok(true);
        }
        sleep(Duration::from_millis(100)).await;
    }
    Ok(false)
}

fn attach_notes(summary: String, notes: &[String]) -> String {
    if notes.is_empty() {
        summary
    } else {
        format!("{} | {}", summary, notes.join(", "))
    }
}
