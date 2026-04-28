use anyhow::{Result, anyhow};

use crate::firefox::actions;

use super::model::{TabInfo, resolve_tabs_with_current};

#[derive(Debug, Clone)]
pub enum TabSwitchTarget {
    Index(usize),
    TitleContains(String),
}

pub async fn switch(target: TabSwitchTarget) -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    let target = resolve_target(&tabs, &target)?;

    if target.is_current {
        return Ok(format!(
            "firefox tab {} already active ({:?})",
            target.index, target.title
        ));
    }

    let summary = actions::click::click_target_node(
        &target.node,
        &format!("Page Tab: {:?}", target.title),
        &target.node.path.join(" > "),
    )
    .await?;

    Ok(format!(
        "switched to firefox tab {} ({:?}) | {}",
        target.index, target.title, summary
    ))
}

fn resolve_target<'a>(tabs: &'a [TabInfo], target: &TabSwitchTarget) -> Result<&'a TabInfo> {
    match target {
        TabSwitchTarget::Index(index) => tabs
            .iter()
            .find(|tab| tab.index == *index)
            .ok_or_else(|| anyhow!("firefox tab index {} is out of range", index)),
        TabSwitchTarget::TitleContains(needle) => resolve_by_title_contains(tabs, needle),
    }
}

fn resolve_by_title_contains<'a>(tabs: &'a [TabInfo], needle: &str) -> Result<&'a TabInfo> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Err(anyhow!("firefox tab title query cannot be empty"));
    }

    let normalized = needle.to_ascii_lowercase();
    let mut matches = tabs
        .iter()
        .filter(|tab| tab.title.to_ascii_lowercase().contains(&normalized));

    let first = matches
        .next()
        .ok_or_else(|| anyhow!("no firefox tab title contains {:?}", needle))?;

    if let Some(second) = matches.next() {
        return Err(anyhow!(
            "firefox tab title query {:?} is ambiguous; matched {:?} and {:?}",
            needle,
            first.title,
            second.title
        ));
    }

    Ok(first)
}
