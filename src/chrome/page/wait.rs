use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use atspi::State;
use tokio::time::{Instant, sleep};

use crate::{chrome::wait as chrome_wait, model::UiNode, selector};

use super::find;
use super::root;
use super::root::PageScope;

/// Wait conditions for page-level assertions.
#[derive(Debug, Clone, Copy)]
pub struct PageWaitConditions<'a> {
    pub text: Option<&'a str>,
    pub title_contains: Option<&'a str>,
    pub url_contains: Option<&'a str>,
    pub disappear: bool,
}

/// Polling controls for page waits.
#[derive(Debug, Clone, Copy)]
pub struct PageWaitTiming {
    pub timeout_ms: u64,
    pub poll_ms: u64,
}

pub async fn wait_for_target(
    scope: &PageScope,
    raw_selectors: &[String],
    conditions: PageWaitConditions<'_>,
    timing: PageWaitTiming,
) -> Result<String> {
    let target = classify_wait(
        raw_selectors,
        conditions.text,
        conditions.title_contains,
        conditions.url_contains,
        false,
    )?
    .expect("allow_empty=false guarantees a target");
    wait_for_kind(
        scope,
        target,
        conditions.disappear,
        timing.timeout_ms,
        timing.poll_ms,
    )
    .await
}

pub async fn wait_for_state(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
    state: PageStateWait,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match find::find_nth(scope, raw_selectors, nth).await {
            Ok(node) => {
                let states = crate::inspect::read_state_set(&node).await?;
                if state.matches(states) {
                    return Ok(format!(
                        "page state {:?} satisfied for {} after {}ms ({} attempts)",
                        state,
                        node.line_label(),
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page state {:?}: {}",
                        timeout_ms,
                        state,
                        err
                    ));
                }
            }
        }

        if start.elapsed() >= timeout {
            return Err(anyhow!(
                "timed out after {}ms waiting for page state {:?}",
                timeout_ms,
                state
            ));
        }
        sleep(interval).await;
    }
}

pub async fn wait_for_optional_target(
    scope: &PageScope,
    raw_selectors: &[String],
    conditions: PageWaitConditions<'_>,
    timing: PageWaitTiming,
) -> Result<Option<String>> {
    let Some(target) = classify_wait(
        raw_selectors,
        conditions.text,
        conditions.title_contains,
        conditions.url_contains,
        true,
    )?
    else {
        return Ok(None);
    };
    wait_for_kind(
        scope,
        target,
        conditions.disappear,
        timing.timeout_ms,
        timing.poll_ms,
    )
    .await
    .map(Some)
}

enum PageWaitTarget {
    Text(String),
    Selector(Vec<selector::Selector>),
    TitleContains(String),
    UrlContains(String),
}

#[derive(Debug, Clone, Copy)]
pub enum PageStateWait {
    Focused,
    Checked,
    Enabled,
    Disabled,
    Expanded,
    Collapsed,
}

impl PageStateWait {
    fn matches(self, states: atspi::StateSet) -> bool {
        match self {
            Self::Focused => states.contains(State::Focused),
            Self::Checked => {
                states.contains(State::Checked)
                    || states.contains(State::Selected)
                    || states.contains(State::Pressed)
            }
            Self::Enabled => states.contains(State::Enabled) && states.contains(State::Sensitive),
            Self::Disabled => {
                !states.contains(State::Enabled) || !states.contains(State::Sensitive)
            }
            Self::Expanded => states.contains(State::Expanded),
            Self::Collapsed => states.contains(State::Collapsed),
        }
    }
}

fn classify_wait(
    raw_selectors: &[String],
    text: Option<&str>,
    title_contains: Option<&str>,
    url_contains: Option<&str>,
    allow_empty: bool,
) -> Result<Option<PageWaitTarget>> {
    let normalized_text = text.map(normalize_text).filter(|value| !value.is_empty());
    let normalized_title = title_contains
        .map(normalize_fragment)
        .filter(|value| !value.is_empty());
    let normalized_url = url_contains
        .map(normalize_fragment)
        .filter(|value| !value.is_empty());

    let text_shorthand = if normalized_text.is_none()
        && normalized_title.is_none()
        && normalized_url.is_none()
        && raw_selectors.len() == 1
    {
        raw_selectors[0]
            .split_once(':')
            .and_then(|(prefix, value)| {
                prefix
                    .trim()
                    .eq_ignore_ascii_case("text")
                    .then(|| normalize_text(value))
            })
            .filter(|value| !value.is_empty())
    } else {
        None
    };

    let has_selector_chain = !raw_selectors.is_empty() && text_shorthand.is_none();
    let mut kinds = 0;
    if has_selector_chain {
        kinds += 1;
    }
    if normalized_text.is_some() || text_shorthand.is_some() {
        kinds += 1;
    }
    if normalized_title.is_some() {
        kinds += 1;
    }
    if normalized_url.is_some() {
        kinds += 1;
    }

    if kinds == 0 {
        if allow_empty {
            return Ok(None);
        }
        bail!(
            "page wait needs a selector chain, --text <contains>, --title-contains <text>, or --url-contains <text>"
        );
    }

    if kinds > 1 {
        bail!(
            "page wait needs exactly one of selector chain, --text, --title-contains, or --url-contains"
        );
    }

    if let Some(value) = normalized_text.or(text_shorthand) {
        return Ok(Some(PageWaitTarget::Text(value)));
    }
    if let Some(value) = normalized_title {
        return Ok(Some(PageWaitTarget::TitleContains(value)));
    }
    if let Some(value) = normalized_url {
        return Ok(Some(PageWaitTarget::UrlContains(value)));
    }
    if has_selector_chain {
        return Ok(Some(PageWaitTarget::Selector(
            selector::parse_selector_chain(raw_selectors)?,
        )));
    }

    unreachable!("kinds>0 guarantees one wait target")
}

async fn wait_for_kind(
    scope: &PageScope,
    target: PageWaitTarget,
    disappear: bool,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    match target {
        PageWaitTarget::Text(needle) => {
            if disappear {
                wait_for_text_disappear(scope, &needle, timeout_ms, poll_ms).await
            } else {
                wait_for_text_appear(scope, &needle, timeout_ms, poll_ms).await
            }
        }
        PageWaitTarget::Selector(selectors) => {
            if disappear {
                wait_for_selector_disappear(scope, &selectors, timeout_ms, poll_ms).await
            } else {
                wait_for_selector_appear(scope, &selectors, timeout_ms, poll_ms).await
            }
        }
        PageWaitTarget::TitleContains(needle) => {
            wait_for_browser_fragment("title", &needle, disappear, timeout_ms, poll_ms, || async {
                chrome_wait::current_title()
                    .await
                    .map(|value| normalize_fragment(&value))
            })
            .await
        }
        PageWaitTarget::UrlContains(needle) => {
            wait_for_browser_fragment("url", &needle, disappear, timeout_ms, poll_ms, || async {
                chrome_wait::current_url()
                    .await
                    .map(|value| normalize_fragment(&canonical_browser_url(&value)))
            })
            .await
        }
    }
}

async fn wait_for_selector_appear(
    scope: &PageScope,
    selectors: &[selector::Selector],
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match root::resolve_in_page_scope(scope, selectors).await {
            Ok(matches) if !matches.is_empty() => {
                let label = selector_chain_label(selectors);
                return Ok(format!(
                    "page selector {} appeared after {}ms ({} attempts)",
                    label,
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
            Ok(_) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page selector {}",
                        timeout_ms,
                        selector_chain_label(selectors)
                    ));
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page selector {}: {}",
                        timeout_ms,
                        selector_chain_label(selectors),
                        err
                    ));
                }
            }
        }

        sleep(interval).await;
    }
}

async fn wait_for_selector_disappear(
    scope: &PageScope,
    selectors: &[selector::Selector],
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match root::resolve_in_page_scope(scope, selectors).await {
            Ok(matches) if matches.is_empty() => {
                return Ok(format!(
                    "page selector {} disappeared after {}ms ({} attempts)",
                    selector_chain_label(selectors),
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
            Ok(_) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page selector {} to disappear",
                        timeout_ms,
                        selector_chain_label(selectors)
                    ));
                }
            }
            Err(_) => {
                return Ok(format!(
                    "page selector {} disappeared after {}ms ({} attempts)",
                    selector_chain_label(selectors),
                    start.elapsed().as_millis(),
                    attempts
                ));
            }
        }

        sleep(interval).await;
    }
}

async fn wait_for_text_appear(
    scope: &PageScope,
    needle: &str,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match root::inspect_page_in_scope(scope).await {
            Ok(tree) => {
                if page_contains_text(&tree, needle) {
                    return Ok(format!(
                        "page text {:?} appeared after {}ms ({} attempts)",
                        needle,
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page text {:?}: text not present",
                        timeout_ms,
                        needle
                    ));
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page text {:?}: {}",
                        timeout_ms,
                        needle,
                        err
                    ));
                }
            }
        }

        sleep(interval).await;
    }
}

async fn wait_for_text_disappear(
    scope: &PageScope,
    needle: &str,
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match root::inspect_page_in_scope(scope).await {
            Ok(tree) => {
                if !page_contains_text(&tree, needle) {
                    return Ok(format!(
                        "page text {:?} disappeared after {}ms ({} attempts)",
                        needle,
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page text {:?} to disappear",
                        timeout_ms,
                        needle
                    ));
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page text {:?} to disappear: {}",
                        timeout_ms,
                        needle,
                        err
                    ));
                }
            }
        }

        sleep(interval).await;
    }
}

async fn wait_for_browser_fragment<F, Fut>(
    label: &str,
    needle: &str,
    disappear: bool,
    timeout_ms: u64,
    poll_ms: u64,
    mut read_current: F,
) -> Result<String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(poll_ms.max(1));
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        match read_current().await {
            Ok(current) => {
                let contains = current.contains(needle);
                let done = if disappear { !contains } else { contains };
                if done {
                    return Ok(format!(
                        "page {} {:?} {} after {}ms ({} attempts)",
                        label,
                        needle,
                        if disappear { "disappeared" } else { "appeared" },
                        start.elapsed().as_millis(),
                        attempts
                    ));
                }

                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page {} fragment {:?} (current {:?})",
                        timeout_ms,
                        label,
                        needle,
                        current
                    ));
                }
            }
            Err(err) => {
                if start.elapsed() >= timeout {
                    return Err(anyhow!(
                        "timed out after {}ms waiting for page {} fragment {:?}: {}",
                        timeout_ms,
                        label,
                        needle,
                        err
                    ));
                }
            }
        }

        sleep(interval).await;
    }
}

fn page_contains_text(node: &UiNode, needle: &str) -> bool {
    subtree_text(node).contains(needle)
}

fn subtree_text(node: &UiNode) -> String {
    let mut chunks = Vec::new();
    collect_text(node, &mut chunks);
    normalize_text(&chunks.join(" "))
}

fn collect_text(node: &UiNode, chunks: &mut Vec<String>) {
    if let Some(name) = node.name.as_deref() {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }
    }

    for child in &node.children {
        collect_text(child, chunks);
    }
}

fn selector_chain_label(selectors: &[selector::Selector]) -> String {
    selectors
        .iter()
        .map(|selector| match (&selector.role, &selector.name) {
            (Some(role), Some(selector::NameMatch::Exact(name))) => format!("{}:{}", role, name),
            (Some(role), Some(selector::NameMatch::Contains(name))) => format!("{}~{}", role, name),
            (Some(role), None) => role.clone(),
            (None, Some(selector::NameMatch::Exact(name))) => format!(":{}", name),
            (None, Some(selector::NameMatch::Contains(name))) => format!("~{}", name),
            (None, None) => "*".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" > ")
}

fn normalize_text(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_fragment(input: &str) -> String {
    normalize_text(input)
}

fn canonical_browser_url(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("about:")
        || lower.starts_with("chrome:")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
    {
        lower
    } else if let Some(rest) = lower.strip_prefix("http://") {
        rest.to_string()
    } else if let Some(rest) = lower.strip_prefix("https://") {
        rest.to_string()
    } else {
        lower
    }
}
