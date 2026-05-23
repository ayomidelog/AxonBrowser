use std::{thread, time::Duration};

use anyhow::{Context, Result, anyhow};

use crate::{
    chrome::{locators, retry, window as chrome_window},
    inspect, live_access,
    model::LiveNode,
    window,
};

#[derive(Debug, Clone)]
pub struct ActionTarget {
    pub locator: locators::ChromeLocator,
    pub node: LiveNode,
    pub label: String,
    pub path: String,
}

impl ActionTarget {
    pub async fn resolve(locator_raw: &str) -> Result<Self> {
        let located =
            retry::with_transient_retry(|| async { locators::locate(locator_raw).await }).await?;
        let label = located.node.line_label();
        let path = located.node.path.join(" > ");
        Ok(Self {
            locator: located.locator,
            node: located.node,
            label,
            path,
        })
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

    pub fn is_address_bar(&self) -> bool {
        self.locator == locators::ChromeLocator::AddressBar
    }
}

pub async fn resolve_target(locator_raw: &str) -> Result<ActionTarget> {
    ActionTarget::resolve(locator_raw).await
}

pub async fn browser_window_for_node(node: &LiveNode) -> Result<window::WindowMatch> {
    if let Some(browser_window) = chrome_window::list_browser_windows(None)?
        .into_iter()
        .find(|browser_window| node_belongs_to_window_title(node, &browser_window.name))
    {
        return Ok(browser_window);
    }

    let (screen_x, screen_y) = inspect::clickable_point(node).await?;
    window::find_window_at_point(screen_x, screen_y)
        .or_else(|_| chrome_window::find_browser_window(None))
}

pub fn activate_window_note(window_id: &str) -> String {
    match window::activate_window(window_id) {
        Ok(()) => "activated window first".to_string(),
        Err(_) => "window activation unavailable; targeted directly".to_string(),
    }
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    use std::process::{Command, Stdio};

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
        drop(stdin);
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
    let _ = window::send_key(window_id, "ctrl+a");
    let _ = window::send_key(window_id, "BackSpace");
    window::send_key(window_id, "ctrl+v")?;
    Ok(())
}

pub fn read_from_clipboard() -> Result<String> {
    let output = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .context("failed to read clipboard via xclip")?;

    if !output.status.success() {
        return Err(anyhow!(
            "xclip failed to read clipboard: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn focus_address_bar(window_id: &str) -> Result<()> {
    let _ = activate_window_note(window_id);
    window::send_key(window_id, "ctrl+l")
}

pub fn read_address_bar_via_clipboard(window_id: &str) -> Result<String> {
    let sentinel = format!(
        "__axonbrowser_clipboard_probe_{}__",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    copy_to_clipboard(&sentinel)?;
    focus_address_bar(window_id)?;
    thread::sleep(Duration::from_millis(120));
    let _ = window::send_key(window_id, "ctrl+a");
    thread::sleep(Duration::from_millis(60));
    window::send_key(window_id, "ctrl+c")?;
    thread::sleep(Duration::from_millis(120));
    let clipboard = read_from_clipboard()?;
    if clipboard == sentinel {
        return Err(anyhow!(
            "address bar clipboard read did not update the clipboard"
        ));
    }
    Ok(clipboard)
}

fn node_belongs_to_window_title(node: &LiveNode, window_title: &str) -> bool {
    let needle = chrome_window::normalize(window_title);
    node.path.iter().any(|segment| {
        let hay = chrome_window::normalize(segment);
        hay.contains(&needle) || needle.contains(&hay)
    })
}
