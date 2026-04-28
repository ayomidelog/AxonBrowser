use anyhow::Result;

use crate::{chrome::actions, window};

use super::{model::resolve_tabs_with_current, recovery, switch};

pub async fn close(index: Option<usize>) -> Result<String> {
    let tabs = resolve_tabs_with_current().await?;
    let previous_count = tabs.len();
    let target_index = index.unwrap_or_else(|| {
        tabs.iter()
            .find(|tab| tab.is_current)
            .map(|tab| tab.index)
            .unwrap_or(0)
    });

    let preface = match tabs.iter().find(|tab| tab.index == target_index) {
        Some(tab) if tab.is_current => {
            format!("closing current chrome tab {} ({:?})", tab.index, tab.title)
        }
        Some(tab) => switch::switch(switch::TabSwitchTarget::Index(tab.index)).await?,
        None => {
            return Err(anyhow::anyhow!(
                "chrome tab index {} is out of range",
                target_index
            ));
        }
    };

    let focus_summary = actions::focus("window").await?;
    window::send_key_active("ctrl+w")?;
    let recovery_summary = recovery::wait_for_close_recovery(previous_count, 4_000, 150).await?;

    Ok(format!(
        "{} | {} | sent ctrl+w to active chrome window for tab {} | {}",
        preface, focus_summary, target_index, recovery_summary
    ))
}
