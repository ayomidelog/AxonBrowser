use anyhow::{Result, anyhow};

use crate::firefox::bidi;

use super::model::{TabInfo, resolve_tabs_with_current};

#[derive(Debug, Clone)]
pub enum TabSwitchTarget {
    Index(usize),
    TitleContains(String),
}

pub async fn switch(target: TabSwitchTarget) -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    let target = resolve_target(&tabs, &target)?;

    let settled = match target {
        TabSwitchTarget::Index(index) => bidi::activate_context_by_index(index).await?,
        TabSwitchTarget::TitleContains(needle) => {
            bidi::activate_context_by_title_contains(&needle).await?
        }
    };

    Ok(format!(
        "switched to firefox tab {:?} ({:?})",
        settled.title, settled.url
    ))
}

fn resolve_target<'a>(tabs: &'a [TabInfo], target: &TabSwitchTarget) -> Result<TabSwitchTarget> {
    match target {
        TabSwitchTarget::Index(index) => {
            if tabs.iter().any(|tab| tab.index == *index) {
                Ok(TabSwitchTarget::Index(*index))
            } else {
                Err(anyhow!("firefox tab index {} is out of range", index))
            }
        }
        TabSwitchTarget::TitleContains(needle) => {
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

            Ok(TabSwitchTarget::TitleContains(needle.to_string()))
        }
    }
}
