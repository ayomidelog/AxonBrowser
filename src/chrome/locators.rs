use anyhow::{Result, anyhow};
use atspi::State;

use crate::{
    chrome::{retry, window as chrome_window},
    inspect,
    model::LiveNode,
    selector, window,
};

const BROWSER_QUERY: &str = "chrome";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeLocator {
    AddressBar,
    Back,
    Forward,
    Reload,
    NewTab,
    TabStrip,
    CurrentTab,
    Window,
}

#[derive(Debug, Clone)]
pub struct LocatedChromeNode {
    pub locator: ChromeLocator,
    pub node: LiveNode,
}

impl ChromeLocator {
    pub fn parse(raw: &str) -> Result<Self> {
        let normalized = normalize(raw);
        let locator = match normalized.as_str() {
            "address" | "address bar" | "addressbar" | "address-bar" | "omnibox" => {
                Self::AddressBar
            }
            "back" => Self::Back,
            "forward" => Self::Forward,
            "reload" | "refresh" => Self::Reload,
            "new tab" | "new-tab" | "newtab" => Self::NewTab,
            "tab strip" | "tab-strip" | "tabstrip" | "tabs" => Self::TabStrip,
            "current tab" | "current-tab" | "currenttab" | "active tab" | "active-tab" => {
                Self::CurrentTab
            }
            "window" | "browser window" | "browser-window" => Self::Window,
            _ => {
                return Err(anyhow!(
                    "unknown chrome locator {:?}; expected one of: {}",
                    raw,
                    Self::supported_names().join(", ")
                ));
            }
        };

        Ok(locator)
    }

    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::AddressBar => "address-bar",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Reload => "reload",
            Self::NewTab => "new-tab",
            Self::TabStrip => "tab-strip",
            Self::CurrentTab => "current-tab",
            Self::Window => "window",
        }
    }

    pub fn supported_names() -> &'static [&'static str] {
        &[
            "address-bar",
            "omnibox",
            "back",
            "forward",
            "reload",
            "new-tab",
            "tab-strip",
            "current-tab",
            "window",
        ]
    }
}

pub async fn locate(raw: &str) -> Result<LocatedChromeNode> {
    retry::with_transient_retry(|| async {
        let locator = ChromeLocator::parse(raw)?;
        let node = resolve_once(locator).await?;
        Ok(LocatedChromeNode { locator, node })
    })
    .await
}

pub async fn resolve(locator: ChromeLocator) -> Result<LiveNode> {
    retry::with_transient_retry(|| async { resolve_once(locator).await }).await
}

pub async fn resolve_chain_for_testing(chain: &[&str]) -> Result<Vec<LiveNode>> {
    resolve_chain(chain).await
}

async fn resolve_once(locator: ChromeLocator) -> Result<LiveNode> {
    match locator {
        ChromeLocator::AddressBar => {
            resolve_first_from_chains(&[&["Tool Bar", "Entry~address"]]).await
        }
        ChromeLocator::Back => {
            resolve_first_from_chains(&[&["Tool Bar", "Push Button:Back"]]).await
        }
        ChromeLocator::Forward => {
            resolve_first_from_chains(&[&["Tool Bar", "Push Button:Forward"]]).await
        }
        ChromeLocator::Reload => {
            resolve_first_from_chains(&[&["Tool Bar", "Push Button:Reload"]]).await
        }
        ChromeLocator::NewTab => {
            resolve_first_from_chains(&[
                &["Page Tab List", "Push Button:New Tab"],
                &["Tool Bar", "Push Button:New tab"],
            ])
            .await
        }
        ChromeLocator::TabStrip => resolve_first_from_chains(&[&["Page Tab List"]]).await,
        ChromeLocator::CurrentTab => resolve_current_tab().await,
        ChromeLocator::Window => resolve_window().await,
    }
}

#[cfg(test)]
pub fn tab_selector_chain() -> Vec<selector::Selector> {
    selector::parse_selector_chain(&["Page Tab List".to_string(), "Page Tab".to_string()])
        .expect("static chrome tab selector chain parses")
}

async fn resolve_window() -> Result<LiveNode> {
    let candidates = resolve_chain(&["Frame"]).await?;
    choose_by_states(candidates, &[State::Active, State::Focused], "window").await
}

async fn resolve_current_tab() -> Result<LiveNode> {
    let candidates = resolve_chain(&["Page Tab List", "Page Tab"]).await?;
    choose_by_states(
        candidates,
        &[State::Selected, State::Active, State::Focused],
        "current tab",
    )
    .await
}

async fn resolve_first_from_chains(chains: &[&[&str]]) -> Result<LiveNode> {
    let mut failures = Vec::new();

    for chain in chains {
        match resolve_chain(chain).await {
            Ok(nodes) if !nodes.is_empty() => {
                return Ok(nodes.into_iter().next().expect("non-empty result"));
            }
            Ok(_) => failures.push(format!("{} => no matches", format_chain(chain))),
            Err(err) => failures.push(format!("{} => {}", format_chain(chain), err)),
        }
    }

    Err(anyhow!(
        "failed to resolve chrome locator; tried {}",
        failures.join("; ")
    ))
}

async fn resolve_chain(chain: &[&str]) -> Result<Vec<LiveNode>> {
    let raw = chain
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>();
    let selectors = selector::parse_selector_chain(&raw)?;
    let query = browser_root_query();
    let nodes = inspect::resolve(&query, &selectors).await?;
    scope_nodes_to_active_window(nodes).await
}

fn browser_root_query() -> String {
    chrome_window::find_browser_window(None)
        .map(|window| window.name)
        .unwrap_or_else(|_| BROWSER_QUERY.to_string())
}

async fn choose_by_states(
    candidates: Vec<LiveNode>,
    states: &[State],
    label: &str,
) -> Result<LiveNode> {
    if candidates.is_empty() {
        return Err(anyhow!("no chrome {} candidates matched", label));
    }

    let mut fallback = None;
    for candidate in candidates {
        if fallback.is_none() {
            fallback = Some(candidate.clone());
        }

        let state_set = inspect::read_state_set(&candidate)
            .await
            .unwrap_or_default();
        if states.iter().any(|state| state_set.contains(*state)) {
            return Ok(candidate);
        }
    }

    Ok(fallback.expect("fallback set when candidates is not empty"))
}

async fn scope_nodes_to_active_window(nodes: Vec<LiveNode>) -> Result<Vec<LiveNode>> {
    let Some(target_window) = chrome_window::find_browser_window(None).ok() else {
        return Ok(nodes);
    };

    let path_filtered = nodes
        .iter()
        .filter(|node| node_belongs_to_window_title(node, &target_window.name))
        .cloned()
        .collect::<Vec<_>>();
    if !path_filtered.is_empty() {
        return Ok(path_filtered);
    }

    let mut filtered = Vec::new();
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
            filtered.push(node);
        }
    }

    if filtered.is_empty() {
        Ok(nodes)
    } else {
        Ok(filtered)
    }
}

fn node_belongs_to_window_title(node: &LiveNode, window_title: &str) -> bool {
    let needle = normalize(window_title);
    node.path.iter().any(|segment| {
        let hay = normalize(segment);
        hay.contains(&needle) || needle.contains(&hay)
    })
}

fn format_chain(chain: &[&str]) -> String {
    chain.join(" > ")
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{ChromeLocator, tab_selector_chain};

    #[test]
    fn parses_aliases() {
        assert_eq!(
            ChromeLocator::parse("omnibox").unwrap(),
            ChromeLocator::AddressBar
        );
        assert_eq!(
            ChromeLocator::parse("new-tab").unwrap(),
            ChromeLocator::NewTab
        );
        assert_eq!(
            ChromeLocator::parse("active tab").unwrap(),
            ChromeLocator::CurrentTab
        );
        assert_eq!(
            ChromeLocator::parse("browser window").unwrap(),
            ChromeLocator::Window
        );
    }

    #[test]
    fn builds_tab_selector_chain() {
        let chain = tab_selector_chain();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].role.as_deref(), Some("page tab list"));
        assert_eq!(chain[1].role.as_deref(), Some("page tab"));
    }
}
