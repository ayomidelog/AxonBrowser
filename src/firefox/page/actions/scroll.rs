use anyhow::{Result, bail};

use crate::window;

use super::target::PageActionTarget;
use crate::firefox::page::root::PageScope;

#[derive(Debug, Clone, Copy)]
pub enum PageScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl PageScrollDirection {
    pub fn to_window_direction(self) -> window::ScrollDirection {
        match self {
            Self::Up => window::ScrollDirection::Up,
            Self::Down => window::ScrollDirection::Down,
            Self::Left => window::ScrollDirection::Left,
            Self::Right => window::ScrollDirection::Right,
        }
    }
}

pub async fn scroll_window(
    scope: &PageScope,
    direction: PageScrollDirection,
    amount: u32,
) -> Result<String> {
    let browser_window = if scope.frame_selectors.is_empty() {
        crate::firefox::window::find_firefox_window(None)?
    } else {
        let root = crate::firefox::page::root::resolve_page_scope(scope).await?;
        crate::firefox::actions::context::browser_window_for_node(&root).await?
    };
    let activation_note =
        crate::firefox::actions::context::activate_window_note(&browser_window.id);
    window::scroll(&browser_window.id, direction.to_window_direction(), amount)?;
    Ok(format!(
        "scrolled page window {} {:?} x{} ({})",
        browser_window.id,
        direction,
        amount.max(1),
        activation_note
    ))
}

pub async fn scroll_target_into_view(
    scope: &PageScope,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    if !target.scroll_into_view().await? {
        bail!(
            "page target {} did not expose scroll-into-view",
            target.label
        );
    }
    Ok(format!(
        "scrolled {} into view ({})",
        target.label, target.path
    ))
}
