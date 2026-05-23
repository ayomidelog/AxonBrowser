use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};

use crate::{
    chrome::retry, edge::window as edge_window, inspect, live_access, model::LiveNode, window,
};

#[derive(Debug, Clone)]
pub struct ActionTarget {
    pub node: LiveNode,
    pub label: String,
    pub path: String,
}

impl ActionTarget {
    pub async fn resolve(locator_raw: &str) -> Result<Self> {
        let node = retry::with_transient_retry(|| async {
            super::click::resolve_locator(locator_raw).await
        })
        .await?;
        let label = node.line_label();
        let path = node.path.join(" > ");
        Ok(Self { node, label, path })
    }

    pub async fn browser_window(&self) -> Result<window::WindowMatch> {
        browser_window_for_node(&self.node).await
    }

    pub async fn try_grab_focus(&self) -> Result<bool> {
        live_access::grab_focus(&self.node).await
    }

    pub async fn try_set_text(&self, text: &str) -> Result<bool> {
        live_access::set_text(&self.node, text).await
    }
}

pub async fn resolve_target(locator_raw: &str) -> Result<ActionTarget> {
    ActionTarget::resolve(locator_raw).await
}

pub async fn browser_window_for_node(node: &LiveNode) -> Result<window::WindowMatch> {
    let (screen_x, screen_y) = inspect::clickable_point(node).await?;
    window::find_window_at_point(screen_x, screen_y)
        .or_else(|_| edge_window::find_edge_window(None))
}

pub fn activate_window_note(window_id: &str) -> String {
    match window::activate_window(window_id) {
        Ok(()) => "activated window first".to_string(),
        Err(_) => "window activation unavailable; targeted directly".to_string(),
    }
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut copy = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .context("failed to launch xclip for clipboard paste")?;

    if let Some(mut stdin) = copy.stdin.take() {
        use std::io::Write as _;
        stdin
            .write_all(text.as_bytes())
            .context("failed to write clipboard payload")?;
    }

    let status = copy
        .wait()
        .context("failed waiting for xclip clipboard write")?;
    if !status.success() {
        return Err(anyhow!("xclip failed to populate the clipboard"));
    }

    Ok(())
}

pub fn type_via_clipboard(window_id: &str, text: &str) -> Result<()> {
    copy_to_clipboard(text)?;
    let _ = activate_window_note(window_id);
    window::send_key(window_id, "ctrl+l")?;
    window::send_key(window_id, "ctrl+v")?;
    Ok(())
}

pub fn focus_address_bar(window_id: &str) -> Result<()> {
    let _ = activate_window_note(window_id);
    window::send_key(window_id, "ctrl+l")
}
