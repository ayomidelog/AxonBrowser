use anyhow::Result;

use super::{model::resolve_tabs_with_current, recovery};
use crate::chrome::devtools;

pub async fn close(index: Option<usize>) -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    let previous_count = tabs.len();
    let target_index = index.unwrap_or_else(|| {
        tabs.iter()
            .find(|tab| tab.is_current)
            .map(|tab| tab.index)
            .unwrap_or(0)
    });

    let target = match tabs.iter().find(|tab| tab.index == target_index) {
        Some(tab) => tab,
        None => {
            return Err(anyhow::anyhow!(
                "chrome tab index {} is out of range",
                target_index
            ));
        }
    };

    let fallback = if target.is_current {
        tabs.iter()
            .find(|tab| tab.index != target.index)
            .map(|tab| tab.id.clone())
    } else {
        None
    };

    devtools::close_page(&target.id).await?;
    if let Some(fallback_id) = fallback {
        let _ = devtools::activate_page(&fallback_id).await;
    }
    let recovery_summary =
        recovery::wait_for_close_recovery(&target.id, previous_count, 4_000, 150).await?;

    Ok(format!(
        "closed chrome tab {} ({:?}) | {}",
        target_index, target.title, recovery_summary
    ))
}
