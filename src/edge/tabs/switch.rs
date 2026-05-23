use anyhow::{Result, anyhow};

use crate::edge::devtools;

use super::{
    model::{TabInfo, resolve_tabs_with_current},
    recovery,
};

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
            "edge tab {} already active ({:?})",
            target.index, target.title
        ));
    }

    devtools::activate_page(&target.id).await?;
    let recovered = recovery::wait_for_current_tab(&target.id, 4_000, 150).await?;

    Ok(format!(
        "switched to edge tab {} ({:?}) | settled on {:?}",
        target.index, target.title, recovered.title
    ))
}

fn resolve_target<'a>(tabs: &'a [TabInfo], target: &TabSwitchTarget) -> Result<&'a TabInfo> {
    match target {
        TabSwitchTarget::Index(index) => tabs
            .iter()
            .find(|tab| tab.index == *index)
            .ok_or_else(|| anyhow!("edge tab index {} is out of range", index)),
        TabSwitchTarget::TitleContains(needle) => resolve_by_title_contains(tabs, needle),
    }
}

fn resolve_by_title_contains<'a>(tabs: &'a [TabInfo], needle: &str) -> Result<&'a TabInfo> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Err(anyhow!("edge tab title query cannot be empty"));
    }

    let normalized = needle.to_ascii_lowercase();
    let mut matches = tabs
        .iter()
        .filter(|tab| tab.title.to_ascii_lowercase().contains(&normalized));

    let first = matches
        .next()
        .ok_or_else(|| anyhow!("no edge tab title contains {:?}", needle))?;

    if let Some(second) = matches.next() {
        return Err(anyhow!(
            "edge tab title query {:?} is ambiguous; matched {:?} and {:?}",
            needle,
            first.title,
            second.title
        ));
    }

    Ok(first)
}
