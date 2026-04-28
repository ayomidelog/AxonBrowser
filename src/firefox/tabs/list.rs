use anyhow::Result;

use super::model::{render_tabs, resolve_tabs_with_current};

pub async fn list_tabs() -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    Ok(render_tabs(&tabs))
}

pub async fn list_tabs_for_window(window_id: Option<&str>) -> Result<String> {
    let tabs = super::model::resolve_tabs_with_current_for_window(window_id).await?;
    Ok(render_tabs(&tabs))
}
