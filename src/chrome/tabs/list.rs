use anyhow::Result;

use super::model::{render_tabs, resolve_tabs_with_current};

pub async fn list_tabs() -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    Ok(render_tabs(&tabs))
}
