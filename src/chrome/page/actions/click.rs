use anyhow::Result;

use crate::chrome::actions::click::click_target_node;

use super::{physical, target::PageActionTarget};
use crate::chrome::page::root::PageScope;

pub async fn click(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    let mut notes = Vec::new();
    if target.scroll_into_view().await? {
        notes.push("scrolled into view first".to_string());
    }

    let summary = if physical::looks_like_text_input(&target.node.role) {
        physical::mouse_click_target(&target).await?
    } else {
        click_target_node(&target.node, &target.label, &target.path).await?
    };

    if notes.is_empty() {
        Ok(summary)
    } else {
        Ok(format!("{} | {}", summary, notes.join(", ")))
    }
}
