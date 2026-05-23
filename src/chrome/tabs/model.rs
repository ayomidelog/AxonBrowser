use anyhow::{Result, anyhow};

use crate::chrome::{devtools, retry, session};

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub id: String,
    pub title: String,
    pub is_current: bool,
}

pub async fn resolve_tabs_with_current() -> Result<Vec<TabInfo>> {
    retry::with_transient_retry(|| async {
        let tabs = devtools::list_pages_for_session()?;
        if tabs.is_empty() {
            return Err(anyhow!("no chrome tabs matched"));
        }
        let current_tab = devtools::current_page()?;
        Ok(build_tab_infos(
            &tabs,
            current_tab.as_ref().map(|page| page.id.as_str()),
        ))
    })
    .await
}

fn build_tab_infos(tabs: &[devtools::PageInfo], current_id: Option<&str>) -> Vec<TabInfo> {
    let remembered_id = session::read_browser_tab_target();
    let mut ordered = tabs.to_vec();
    ordered.sort_by(|left, right| sort_key(left, current_id, remembered_id.as_deref()).cmp(&sort_key(right, current_id, remembered_id.as_deref())));

    ordered
        .iter()
        .enumerate()
        .map(|(index, tab)| TabInfo {
            index,
            id: tab.id.clone(),
            title: if tab.title.trim().is_empty() {
                "<untitled>".to_string()
            } else {
                tab.title.clone()
            },
            is_current: current_id.map(|id| id == tab.id).unwrap_or(false),
        })
        .collect()
}

fn sort_key(
    tab: &devtools::PageInfo,
    current_id: Option<&str>,
    remembered_id: Option<&str>,
) -> (u8, u8, String, String) {
    let is_current = current_id.map(|id| id == tab.id).unwrap_or(false);
    let is_remembered = remembered_id.map(|id| id == tab.id).unwrap_or(false);
    (
        if is_current { 0 } else { 1 },
        if is_remembered { 0 } else { 1 },
        tab.title.to_ascii_lowercase(),
        tab.id.clone(),
    )
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
    use super::{TabInfo, build_tab_infos, render_tabs};
    use crate::chrome::devtools::PageInfo;

    fn fake_tab(id: &str, title: &str, url: &str) -> PageInfo {
        PageInfo {
            id: id.into(),
            title: title.into(),
            url: url.into(),
            web_socket_debugger_url: "ws://127.0.0.1/devtools/page/test".into(),
        }
    }

    #[test]
    fn marks_current_tab() {
        let first = fake_tab("one", "one", "https://one");
        let second = fake_tab("two", "two", "https://two");
        let tabs = build_tab_infos(&[first.clone(), second.clone()], Some("two"));
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
            id: "lonely".into(),
            title: "lonely".into(),
            is_current: false,
        }];
        let rendered = render_tabs(&tabs);
        assert_eq!(rendered, "- tab 0: lonely\n");
    }
}
