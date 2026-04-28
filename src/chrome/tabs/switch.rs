use anyhow::{Result, anyhow};

use crate::chrome::actions;

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
            "chrome tab {} already active ({:?})",
            target.index, target.title
        ));
    }

    let summary = actions::click::click_target_node(
        &target.node,
        &format!("Page Tab: {:?}", target.title),
        &target.node.path.join(" > "),
    )
    .await?;
    let recovered = recovery::wait_for_current_tab(target.index, 4_000, 150).await?;

    Ok(format!(
        "switched to chrome tab {} ({:?}) | {} | settled on {:?}",
        target.index, target.title, summary, recovered.title
    ))
}

fn resolve_target<'a>(tabs: &'a [TabInfo], target: &TabSwitchTarget) -> Result<&'a TabInfo> {
    match target {
        TabSwitchTarget::Index(index) => tabs
            .iter()
            .find(|tab| tab.index == *index)
            .ok_or_else(|| anyhow!("chrome tab index {} is out of range", index)),
        TabSwitchTarget::TitleContains(needle) => resolve_by_title_contains(tabs, needle),
    }
}

fn resolve_by_title_contains<'a>(tabs: &'a [TabInfo], needle: &str) -> Result<&'a TabInfo> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Err(anyhow!("chrome tab title query cannot be empty"));
    }

    let normalized = needle.to_ascii_lowercase();
    let mut matches = tabs
        .iter()
        .filter(|tab| tab.title.to_ascii_lowercase().contains(&normalized));

    let first = matches
        .next()
        .ok_or_else(|| anyhow!("no chrome tab title contains {:?}", needle))?;

    if let Some(second) = matches.next() {
        return Err(anyhow!(
            "chrome tab title query {:?} is ambiguous; matched {:?} and {:?}",
            needle,
            first.title,
            second.title
        ));
    }

    Ok(first)
}

#[cfg(test)]
mod tests {
    use atspi::ObjectRefOwned;

    use crate::model::LiveNode;

    use super::{TabSwitchTarget, resolve_target};
    use crate::chrome::tabs::model::TabInfo;

    fn fake_tab(index: usize, title: &str, is_current: bool) -> TabInfo {
        TabInfo {
            index,
            title: title.into(),
            node: LiveNode {
                object_ref: ObjectRefOwned::from_static_str_unchecked(
                    "org.a11y.atspi.Registry",
                    "/org/a11y/atspi/accessible/null",
                ),
                role: "Page Tab".into(),
                name: Some(title.into()),
                path: vec![format!("Page Tab: {title}")],
            },
            is_current,
        }
    }

    #[test]
    fn resolves_switch_target_by_index() {
        let tabs = vec![
            fake_tab(0, "New Tab", true),
            fake_tab(1, "Example Domain", false),
        ];
        let resolved = resolve_target(&tabs, &TabSwitchTarget::Index(1)).unwrap();
        assert_eq!(resolved.title, "Example Domain");
    }

    #[test]
    fn resolves_switch_target_by_title_contains_case_insensitive() {
        let tabs = vec![
            fake_tab(0, "New Tab", true),
            fake_tab(1, "Example Domain", false),
        ];
        let resolved =
            resolve_target(&tabs, &TabSwitchTarget::TitleContains("example".into())).unwrap();
        assert_eq!(resolved.index, 1);
    }

    #[test]
    fn rejects_ambiguous_title_queries() {
        let tabs = vec![
            fake_tab(0, "Example Domain", true),
            fake_tab(1, "Another Example", false),
        ];
        let err = resolve_target(&tabs, &TabSwitchTarget::TitleContains("example".into()))
            .unwrap_err()
            .to_string();
        assert!(err.contains("ambiguous"));
    }

    #[test]
    fn rejects_missing_title_queries() {
        let tabs = vec![fake_tab(0, "New Tab", true)];
        let err = resolve_target(&tabs, &TabSwitchTarget::TitleContains("example".into()))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no chrome tab title contains"));
    }
}
