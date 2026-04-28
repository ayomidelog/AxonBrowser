use anyhow::Result;

use crate::{chrome::retry, edge::actions, inspect, model::LiveNode, window};

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub node: LiveNode,
    pub is_current: bool,
}

pub async fn resolve_tabs_with_current() -> Result<Vec<TabInfo>> {
    resolve_tabs_with_current_for_window(None).await
}

pub async fn resolve_tabs_with_current_for_window(window_id: Option<&str>) -> Result<Vec<TabInfo>> {
    retry::with_transient_retry(|| async {
        let tabs = match resolve_tab_nodes(window_id).await {
            Ok(tabs) => tabs,
            Err(_) => {
                let window = match window_id {
                    Some(id) => crate::edge::window::find_edge_window_by_id(id)?,
                    None => crate::edge::window::find_edge_window(None)?,
                };
                return Ok(vec![TabInfo {
                    index: 0,
                    title: infer_tab_title(&window.name),
                    node: synthetic_tab_node(&window.name),
                    is_current: true,
                }]);
            }
        };
        if tabs.is_empty() {
            let window = match window_id {
                Some(id) => crate::edge::window::find_edge_window_by_id(id)?,
                None => crate::edge::window::find_edge_window(None)?,
            };
            return Ok(vec![TabInfo {
                index: 0,
                title: infer_tab_title(&window.name),
                node: synthetic_tab_node(&window.name),
                is_current: true,
            }]);
        }

        let current_tab = resolve_current_tab_for_window(window_id).await.ok();
        Ok(build_tab_infos(&tabs, current_tab.as_ref()))
    })
    .await
}

async fn resolve_tab_nodes(window_id: Option<&str>) -> Result<Vec<LiveNode>> {
    let target_window = match window_id {
        Some(id) => crate::edge::window::find_edge_window_by_id(id)?,
        None => crate::edge::window::find_edge_window(None)?,
    };

    if let Ok(tab_strip) = resolve_locator_for_window("tab-strip", window_id).await {
        let chain = crate::selector::parse_selector_chain(&["Page Tab".to_string()])?;
        let within_strip = crate::inspect::resolve_within_scope(&tab_strip, &chain).await?;
        if !within_strip.is_empty() {
            let filtered = scope_nodes_to_window(within_strip, &target_window).await;
            if !filtered.is_empty() {
                return Ok(filtered);
            }
        }
    }

    let fallback =
        crate::edge::actions::click::resolve_chain_scoped(&["Page Tab List", "Page Tab"]).await?;
    Ok(scope_nodes_to_window(fallback, &target_window).await)
}

async fn resolve_current_tab_for_window(window_id: Option<&str>) -> Result<LiveNode> {
    resolve_locator_for_window("current-tab", window_id).await
}

async fn resolve_locator_for_window(locator: &str, window_id: Option<&str>) -> Result<LiveNode> {
    if let Some(window_id) = window_id {
        let window = crate::edge::window::find_edge_window_by_id(window_id)?;
        let _ = crate::edge::session::remember_target(&window.id);
    }
    actions::click::resolve_locator(locator).await
}

async fn scope_nodes_to_window(
    nodes: Vec<LiveNode>,
    target_window: &crate::window::WindowMatch,
) -> Vec<LiveNode> {
    let path_filtered = nodes
        .iter()
        .filter(|node| node_belongs_to_window_title(node, &target_window.name))
        .cloned()
        .collect::<Vec<_>>();
    if !path_filtered.is_empty() {
        return path_filtered;
    }

    let mut point_filtered = Vec::new();
    for node in nodes.iter().cloned() {
        let (screen_x, screen_y) = match inspect::clickable_point(&node).await {
            Ok(point) => point,
            Err(_) => continue,
        };
        let matched_window = match window::find_window_at_point(screen_x, screen_y) {
            Ok(found) => found,
            Err(_) => continue,
        };
        if matched_window.id == target_window.id {
            point_filtered.push(node);
        }
    }

    point_filtered
}

fn node_belongs_to_window_title(node: &LiveNode, window_title: &str) -> bool {
    let needle = crate::edge::window::normalize(window_title);
    node.path.iter().any(|segment| {
        let hay = crate::edge::window::normalize(segment);
        hay.contains(&needle) || needle.contains(&hay)
    })
}

fn infer_tab_title(window_title: &str) -> String {
    let mut trimmed = window_title.trim().to_string();
    for suffix in [" - Microsoft Edge", " - Microsoft\u{200b} Edge"] {
        if let Some(stripped) = trimmed.strip_suffix(suffix) {
            trimmed = stripped.trim().to_string();
            break;
        }
    }

    if let Some((prefix, profile)) = trimmed.rsplit_once(" - ") {
        let profile_normalized = profile.trim().to_ascii_lowercase();
        if profile_normalized.starts_with("profile ") || profile_normalized == "inprivate" {
            trimmed = prefix.trim().to_string();
        }
    }

    trimmed
}

fn synthetic_tab_node(window_title: &str) -> LiveNode {
    LiveNode {
        object_ref: atspi::ObjectRefOwned::from_static_str_unchecked(
            "org.a11y.atspi.Registry",
            "/org/a11y/atspi/accessible/null",
        ),
        role: "Page Tab".to_string(),
        name: Some(infer_tab_title(window_title)),
        path: vec![format!("Frame:{}", window_title)],
    }
}

pub fn build_tab_infos(tabs: &[LiveNode], current: Option<&LiveNode>) -> Vec<TabInfo> {
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
