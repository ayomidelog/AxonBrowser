use anyhow::{Result, anyhow};

use crate::{inspect, live_access};

use super::target::PageActionTarget;
use crate::chrome::page::root::PageScope;

pub async fn read_text(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    let text = live_access::read_text(&target.node)
        .await?
        .or_else(|| target.node.name.clone())
        .ok_or_else(|| anyhow!("no readable text on {}", target.label))?;
    Ok(text)
}

pub async fn read_value(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    if let Some(text) = live_access::read_text(&target.node).await? {
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let tree = inspect::inspect_live(&target.node).await?;
    let fallback = subtree_text(&tree);
    if fallback.is_empty() {
        return Err(anyhow!("no readable value on {}", target.label));
    }
    Ok(fallback)
}

fn subtree_text(node: &crate::model::UiNode) -> String {
    let mut parts = Vec::new();
    collect(node, &mut parts);
    parts.join(" ").trim().to_string()
}

fn collect(node: &crate::model::UiNode, parts: &mut Vec<String>) {
    if let Some(name) = node.name.as_deref() {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    for child in &node.children {
        collect(child, parts);
    }
}
