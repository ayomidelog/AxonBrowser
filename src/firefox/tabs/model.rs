use anyhow::Result;

use crate::firefox::bidi;

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub is_current: bool,
}

pub async fn resolve_tabs_with_current() -> Result<Vec<TabInfo>> {
    resolve_tabs_with_current_for_window(None).await
}

pub async fn resolve_tabs_with_current_for_window(window_id: Option<&str>) -> Result<Vec<TabInfo>> {
    if let Some(window_id) = window_id {
        let _ = crate::firefox::session::remember_browser_window_target(window_id);
    }
    let contexts = bidi::list_contexts().await?;
    Ok(contexts
        .into_iter()
        .enumerate()
        .map(|(index, context)| TabInfo {
            index,
            title: context.title,
            is_current: context.is_current,
        })
        .collect())
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
    use super::{TabInfo, render_tabs};

    #[test]
    fn renders_without_current_marker_when_unknown() {
        let tabs = vec![TabInfo {
            index: 0,
            title: "lonely".into(),
            is_current: false,
        }];
        let rendered = render_tabs(&tabs);
        assert_eq!(rendered, "- tab 0: lonely\n");
    }
}
