use anyhow::Result;

use crate::{firefox::actions::context, live_access, model::LiveNode, window};

use super::super::{find, root::PageScope};

#[derive(Debug, Clone)]
pub struct PageActionTarget {
    pub node: LiveNode,
    pub label: String,
    pub path: String,
}

impl PageActionTarget {
    pub async fn resolve(scope: &PageScope, raw_selectors: &[String]) -> Result<Self> {
        Self::resolve_nth(scope, raw_selectors, None).await
    }

    pub async fn resolve_nth(
        scope: &PageScope,
        raw_selectors: &[String],
        nth: Option<usize>,
    ) -> Result<Self> {
        let node = find::find_nth(scope, raw_selectors, nth).await?;
        let label = node.line_label();
        let path = node.path.join(" > ");
        Ok(Self { node, label, path })
    }

    pub async fn browser_window(&self) -> Result<window::WindowMatch> {
        context::browser_window_for_node(&self.node).await
    }

    pub async fn try_grab_focus(&self) -> Result<bool> {
        live_access::grab_focus(&self.node).await
    }

    pub async fn try_set_text(&self, text: &str) -> Result<bool> {
        live_access::set_text(&self.node, text).await
    }

    pub async fn scroll_into_view(&self) -> Result<bool> {
        live_access::scroll_into_view(&self.node).await
    }

    pub async fn state_set(&self) -> Result<atspi::StateSet> {
        crate::inspect::read_state_set(&self.node).await
    }
}
