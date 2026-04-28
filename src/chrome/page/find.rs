use anyhow::{Result, anyhow};

use crate::{
    model::{LiveNode, UiNode},
    selector,
};

use super::root::{self, PageScope};

pub async fn inspect(scope: &PageScope) -> Result<UiNode> {
    root::inspect_page_in_scope(scope).await
}

pub async fn find(scope: &PageScope, raw_selectors: &[String]) -> Result<Vec<LiveNode>> {
    let selectors = selector::parse_selector_chain(raw_selectors)?;
    root::resolve_in_page_scope(scope, &selectors).await
}

pub async fn find_nth(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<LiveNode> {
    let matches = find(scope, raw_selectors).await?;
    select_nth(matches, nth, "page")
}

pub async fn count(scope: &PageScope, raw_selectors: &[String]) -> Result<usize> {
    Ok(find(scope, raw_selectors).await?.len())
}

pub fn select_nth(matches: Vec<LiveNode>, nth: Option<usize>, label: &str) -> Result<LiveNode> {
    if matches.is_empty() {
        return Err(anyhow!("no {} matches", label));
    }

    let index = nth.unwrap_or(0);
    matches.into_iter().nth(index).ok_or_else(|| {
        anyhow!(
            "{} match index {} out of range ({} matches)",
            label,
            index,
            index + 1
        )
    })
}

pub async fn frames(scope: &PageScope) -> Result<Vec<LiveNode>> {
    let frames = root::list_frames(scope).await?;
    if frames.is_empty() && scope.frame_selectors.is_empty() {
        let inferred = root::infer_frames_from_tree(scope).await?;
        if !inferred.is_empty() {
            return Ok(inferred);
        }
        return Err(anyhow!("no frame matches in {}", scope.describe()));
    }
    Ok(frames)
}
