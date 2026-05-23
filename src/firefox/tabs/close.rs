use anyhow::Result;

use crate::firefox::bidi;

use super::model::resolve_tabs_with_current;

pub async fn close(index: Option<usize>) -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    let target_index = index.unwrap_or_else(|| {
        tabs.iter()
            .find(|tab| tab.is_current)
            .map(|tab| tab.index)
            .unwrap_or(0)
    });

    bidi::close_context_by_index(target_index).await
}
