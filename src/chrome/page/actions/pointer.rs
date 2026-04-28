use anyhow::Result;

use crate::window;

use super::{physical, target::PageActionTarget};
use crate::chrome::page::root::PageScope;

#[derive(Debug, Clone, Copy)]
pub enum PointerClickKind {
    Double,
    Secondary,
}

pub async fn hover(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    let mut notes = Vec::new();
    if target.scroll_into_view().await? {
        notes.push("scrolled into view first".to_string());
    }

    let (window_match, relative_x, relative_y) = physical::window_relative_point(&target).await?;
    let activation_note = crate::chrome::actions::context::activate_window_note(&window_match.id);
    window::mousemove(&window_match.id, relative_x, relative_y)?;

    Ok(attach_notes(
        format!(
            "hovered {} via X11 at {},{} in window {} ({}, {})",
            target.label, relative_x, relative_y, window_match.id, target.path, activation_note
        ),
        &notes,
    ))
}

pub async fn click_kind(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
    kind: PointerClickKind,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    let mut notes = Vec::new();
    if target.scroll_into_view().await? {
        notes.push("scrolled into view first".to_string());
    }

    let summary = match kind {
        PointerClickKind::Double => physical::mouse_click_target_button(&target, 1, 2).await?,
        PointerClickKind::Secondary => physical::mouse_click_target_button(&target, 3, 1).await?,
    };

    Ok(attach_notes(summary, &notes))
}

fn attach_notes(summary: String, notes: &[String]) -> String {
    if notes.is_empty() {
        summary
    } else {
        format!("{} | {}", summary, notes.join(", "))
    }
}
