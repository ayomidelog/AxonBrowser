use std::collections::HashSet;

use anyhow::{Result, anyhow, bail};

use crate::{inspect, model::LiveNode, selector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserOptionPrompt {
    LeaveSite,
    SavePassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserOptionChoice {
    Cancel,
    Leave,
    Never,
    Save,
}

impl BrowserOptionPrompt {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::LeaveSite => "leave-site",
            Self::SavePassword => "save-password",
        }
    }
}

impl BrowserOptionChoice {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Cancel => "cancel",
            Self::Leave => "leave",
            Self::Never => "never",
            Self::Save => "save",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BrowserChoiceSpec {
    pub choice: BrowserOptionChoice,
    pub labels: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct BrowserPromptSpec {
    pub prompt: BrowserOptionPrompt,
    pub titles: &'static [&'static str],
    pub choices: &'static [BrowserChoiceSpec],
}

#[derive(Debug, Clone)]
pub struct BrowserOptionProfile {
    pub browser_name: &'static str,
    pub window_title: String,
    pub query_candidates: Vec<String>,
    pub prompts: &'static [BrowserPromptSpec],
}

pub async fn choose(
    profile: &BrowserOptionProfile,
    prompt: BrowserOptionPrompt,
    choice: BrowserOptionChoice,
) -> Result<LiveNode> {
    let browser_nodes = resolve_browser_nodes(profile).await?;
    let prompt_spec = profile
        .prompts
        .iter()
        .find(|spec| spec.prompt == prompt)
        .ok_or_else(|| {
            anyhow!(
                "{} does not support prompt {}",
                profile.browser_name,
                prompt.canonical_name()
            )
        })?;
    let labels = prompt_spec
        .choices
        .iter()
        .find(|spec| spec.choice == choice)
        .map(|spec| spec.labels)
        .ok_or_else(|| {
            anyhow!(
                "{} option {} does not support {}; supported buttons: {}",
                profile.browser_name,
                prompt.canonical_name(),
                choice.canonical_name(),
                supported_choices(prompt_spec)
            )
        })?;

    let prompt_titles = find_nodes_by_name(&browser_nodes, prompt_spec.titles)?
        .into_iter()
        .filter(|node| !path_is_page_content(&node.path))
        .collect::<Vec<_>>();

    let mut buttons = find_buttons_by_label(&browser_nodes, labels)?
        .into_iter()
        .filter(|node| !path_is_page_content(&node.path))
        .collect::<Vec<_>>();

    if buttons.is_empty() {
        bail!(
            "could not find a {} {} {} button outside page content",
            profile.browser_name,
            prompt.canonical_name(),
            choice.canonical_name()
        );
    }

    if let Some(best) = choose_nearest_button(&buttons, &prompt_titles).await? {
        return Ok(best);
    }

    let mut prompt_scoped = buttons
        .drain(..)
        .filter(|node| path_mentions_prompt(&node.path, prompt_spec.titles))
        .collect::<Vec<_>>();

    match prompt_scoped.len() {
        1 => Ok(prompt_scoped.remove(0)),
        0 => Err(anyhow!(
            "could not disambiguate {} option {} {}; prompt title was not found near any matching browser button",
            profile.browser_name,
            prompt.canonical_name(),
            choice.canonical_name()
        )),
        _ => Err(anyhow!(
            "multiple {} {} {} button candidates matched outside page content: {}",
            profile.browser_name,
            prompt.canonical_name(),
            choice.canonical_name(),
            describe_candidates(&prompt_scoped)
        )),
    }
}

fn find_nodes_by_name(nodes: &[LiveNode], names: &[&str]) -> Result<Vec<LiveNode>> {
    let selectors = names
        .iter()
        .map(|name| selector::Selector::parse(&format!("~{}", name)))
        .collect::<Result<Vec<_>>>()?;
    let mut matches = Vec::new();
    let mut seen = HashSet::new();

    for node in nodes_iter(nodes, &selectors) {
        let key = node_key(&node);
        if seen.insert(key) {
            matches.push(node);
        }
    }

    Ok(matches)
}

fn find_buttons_by_label(nodes: &[LiveNode], labels: &[&str]) -> Result<Vec<LiveNode>> {
    let selectors = labels
        .iter()
        .map(|label| selector::Selector::parse(&format!("Button:{}", label)))
        .collect::<Result<Vec<_>>>()?;
    let mut matches = Vec::new();
    let mut seen = HashSet::new();

    for node in nodes_iter(nodes, &selectors) {
        let key = node_key(&node);
        if seen.insert(key) {
            matches.push(node);
        }
    }

    Ok(matches)
}

fn nodes_iter<'a>(
    nodes: &'a [LiveNode],
    selectors: &'a [selector::Selector],
) -> impl Iterator<Item = LiveNode> + 'a {
    nodes
        .iter()
        .filter(move |node| selectors.iter().any(|selector| selector.matches_live(node)))
        .cloned()
}

async fn resolve_browser_nodes(profile: &BrowserOptionProfile) -> Result<Vec<LiveNode>> {
    let mut failures = Vec::new();

    for query in &profile.query_candidates {
        let root = match inspect::resolve(query, &[]).await {
            Ok(mut roots) if !roots.is_empty() => roots.remove(0),
            Ok(_) => {
                failures.push(format!("{query:?} => no matched root"));
                continue;
            }
            Err(err) => {
                failures.push(format!("{query:?} => {err}"));
                continue;
            }
        };

        let mut nodes = vec![root.clone()];
        match inspect::descendants(&root).await {
            Ok(mut descendants) => nodes.append(&mut descendants),
            Err(err) => {
                failures.push(format!("{query:?} => failed to walk browser tree: {err}"));
                continue;
            }
        }

        let scoped = nodes
            .into_iter()
            .filter(|node| path_belongs_to_window_title(&node.path, &profile.window_title))
            .collect::<Vec<_>>();
        if !scoped.is_empty() {
            return Ok(scoped);
        }

        failures.push(format!("{query:?} => matches belonged to another window"));
    }

    Err(anyhow!(
        "failed to resolve {} browser-popup nodes; tried {}",
        profile.browser_name,
        failures.join("; ")
    ))
}

async fn choose_nearest_button(
    buttons: &[LiveNode],
    prompt_titles: &[LiveNode],
) -> Result<Option<LiveNode>> {
    let title_centers = read_node_centers(prompt_titles).await?;
    if title_centers.is_empty() {
        return Ok(None);
    }

    let button_centers = read_nodes_with_centers(buttons).await?;
    let mut best: Option<(i64, LiveNode)> = None;

    for (button, center) in button_centers {
        let distance = title_centers
            .iter()
            .map(|title| squared_distance(center, *title))
            .min()
            .expect("title centers is non-empty");

        match &best {
            Some((best_distance, _)) if distance >= *best_distance => {}
            _ => best = Some((distance, button)),
        }
    }

    Ok(best.map(|(_, button)| button))
}

async fn read_nodes_with_centers(nodes: &[LiveNode]) -> Result<Vec<(LiveNode, (i32, i32))>> {
    let mut centered = Vec::new();
    for node in nodes.iter().cloned() {
        if let Some(center) = node_center(&node).await? {
            centered.push((node, center));
        }
    }
    Ok(centered)
}

async fn read_node_centers(nodes: &[LiveNode]) -> Result<Vec<(i32, i32)>> {
    let mut centers = Vec::new();
    for node in nodes {
        if let Some(center) = node_center(node).await? {
            centers.push(center);
        }
    }
    Ok(centers)
}

async fn node_center(node: &LiveNode) -> Result<Option<(i32, i32)>> {
    let (x, y, width, height) = match inspect::component_extents(node).await {
        Ok(extents) => extents,
        Err(_) => return Ok(None),
    };

    Ok(Some((x + (width / 2), y + (height / 2))))
}

fn squared_distance(left: (i32, i32), right: (i32, i32)) -> i64 {
    let dx = i64::from(left.0) - i64::from(right.0);
    let dy = i64::from(left.1) - i64::from(right.1);
    (dx * dx) + (dy * dy)
}

fn supported_choices(prompt_spec: &BrowserPromptSpec) -> String {
    prompt_spec
        .choices
        .iter()
        .map(|choice| choice.choice.canonical_name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn path_mentions_prompt(path: &[String], titles: &[&str]) -> bool {
    titles.iter().any(|alias| {
        path.iter()
            .any(|segment| normalize(segment).contains(&normalize(alias)))
    })
}

fn path_belongs_to_window_title(path: &[String], window_title: &str) -> bool {
    let normalized_title = normalize(window_title);
    path.iter().any(|segment| {
        let normalized = normalize(segment);
        normalized.contains(&normalized_title) || normalized_title.contains(&normalized)
    })
}

fn path_is_page_content(path: &[String]) -> bool {
    path.iter().any(|segment| {
        let normalized = normalize(segment);
        normalized.contains("document web") || normalized.contains("root web area")
    })
}

fn describe_candidates(nodes: &[LiveNode]) -> String {
    nodes
        .iter()
        .map(|node| format!("{} ({})", node.line_label(), node.path.join(" > ")))
        .collect::<Vec<_>>()
        .join("; ")
}

fn node_key(node: &LiveNode) -> String {
    format!(
        "{}|{}|{}",
        normalize(&node.role),
        node.name.as_deref().map(normalize).unwrap_or_default(),
        node.path
            .iter()
            .map(|segment| normalize(segment))
            .collect::<Vec<_>>()
            .join(">")
    )
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
    use super::{
        BrowserChoiceSpec, BrowserOptionChoice, BrowserOptionProfile, BrowserOptionPrompt,
        BrowserPromptSpec, path_belongs_to_window_title, path_is_page_content,
        path_mentions_prompt,
    };

    const LEAVE_CHOICES: &[BrowserChoiceSpec] = &[
        BrowserChoiceSpec {
            choice: BrowserOptionChoice::Cancel,
            labels: &["cancel"],
        },
        BrowserChoiceSpec {
            choice: BrowserOptionChoice::Leave,
            labels: &["leave"],
        },
    ];
    const PROMPTS: &[BrowserPromptSpec] = &[BrowserPromptSpec {
        prompt: BrowserOptionPrompt::LeaveSite,
        titles: &["leave site", "leave site?"],
        choices: LEAVE_CHOICES,
    }];

    #[test]
    fn canonical_names_are_stable() {
        assert_eq!(
            BrowserOptionPrompt::LeaveSite.canonical_name(),
            "leave-site"
        );
        assert_eq!(BrowserOptionChoice::Never.canonical_name(), "never");
    }

    #[test]
    fn recognizes_page_content_paths() {
        let path = vec![
            "Frame: \"Google Chrome\"".to_string(),
            "Document Web: \"portal.azure.com\"".to_string(),
            "Push Button: \"Save\"".to_string(),
        ];

        assert!(path_is_page_content(&path));
    }

    #[test]
    fn matches_prompt_titles_in_browser_popup_paths() {
        let path = vec![
            "Frame: \"Google Chrome\"".to_string(),
            "Alert: \"Leave site?\"".to_string(),
            "Push Button: \"Leave\"".to_string(),
        ];

        assert!(path_mentions_prompt(&path, &["leave site", "leave site?"]));
        assert!(!path_mentions_prompt(&path, &["save password"]));
    }

    #[test]
    fn matches_window_title_inside_node_path() {
        let path = vec![
            "Frame: \"Home - Microsoft Azure - Chromium\"".to_string(),
            "Dialog: \"Leave site?\"".to_string(),
        ];

        assert!(path_belongs_to_window_title(
            &path,
            "Home - Microsoft Azure - Chromium"
        ));
        assert!(!path_belongs_to_window_title(&path, "Other Window"));
    }

    #[test]
    fn prompt_profiles_are_constructible() {
        let profile = BrowserOptionProfile {
            browser_name: "chrome",
            window_title: "Home - Microsoft Azure - Chromium".to_string(),
            query_candidates: vec!["chrome".to_string()],
            prompts: PROMPTS,
        };

        assert_eq!(profile.prompts.len(), 1);
        assert_eq!(profile.prompts[0].choices.len(), 2);
    }
}
