use anyhow::{Result, anyhow};

use crate::{
    chrome::{locators::ChromeLocator, retry},
    model::LiveNode,
};

use super::super::locators;

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub node: LiveNode,
    pub is_current: bool,
}

pub async fn resolve_tabs_with_current() -> Result<Vec<TabInfo>> {
    retry::with_transient_retry(|| async {
        let tabs = locators::resolve_chain_for_testing(&["Page Tab List", "Page Tab"]).await?;
        if tabs.is_empty() {
            return Err(anyhow!("no chrome tabs matched"));
        }

        let current_tab = locators::resolve(ChromeLocator::CurrentTab).await.ok();
        Ok(build_tab_infos(&tabs, current_tab.as_ref()))
    })
    .await
}

fn build_tab_infos(tabs: &[LiveNode], current: Option<&LiveNode>) -> Vec<TabInfo> {
    tabs.iter()
        .enumerate()
        .map(|(index, tab)| TabInfo {
            index,
            title: tab.name.as_deref().unwrap_or("<unnamed>").to_string(),
            node: tab.clone(),
            is_current: current.map(|node| node.path == tab.path).unwrap_or(false),
        })
        .collect()
}

pub fn render_tabs(tabs: &[TabInfo]) -> String {
    let mut out = String::new();
    for tab in tabs {
        let marker = if tab.is_current { "*" } else { "-" };
        out.push_str(&format!("{} tab {}: {}\n", marker, tab.index, tab.title));
    }
    out
}

#[cfg(test)]
mod tests {
    use atspi::ObjectRefOwned;

    use crate::model::LiveNode;

    use super::{TabInfo, build_tab_infos, render_tabs};

    fn fake_tab(name: &str, path: &[&str]) -> LiveNode {
        LiveNode {
            object_ref: ObjectRefOwned::from_static_str_unchecked(
                "org.a11y.atspi.Registry",
                "/org/a11y/atspi/accessible/null",
            ),
            role: "Page Tab".into(),
            name: Some(name.into()),
            path: path.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn marks_current_tab() {
        let first = fake_tab("one", &["Window", "Tab Strip", "one"]);
        let second = fake_tab("two", &["Window", "Tab Strip", "two"]);
        let tabs = build_tab_infos(&[first.clone(), second.clone()], Some(&second));
        assert!(!tabs[0].is_current);
        assert!(tabs[1].is_current);
        let rendered = render_tabs(&tabs);
        assert!(rendered.contains("- tab 0: one"));
        assert!(rendered.contains("* tab 1: two"));
    }

    #[test]
    fn renders_without_current_marker_when_unknown() {
        let tabs = vec![TabInfo {
            index: 0,
            title: "lonely".into(),
            node: fake_tab("lonely", &["Window", "Tab Strip", "lonely"]),
            is_current: false,
        }];
        let rendered = render_tabs(&tabs);
        assert_eq!(rendered, "- tab 0: lonely\n");
    }
}
