use std::{fs, path::Path};

use anyhow::{Context, Result, anyhow};

use super::{devtools, window};

#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMode {
    Window,
    Active,
}

pub async fn capture(output_path: &str, query_override: Option<&str>) -> Result<String> {
    capture_mode(output_path, query_override, ScreenshotMode::Window).await
}

pub async fn capture_mode(
    output_path: &str,
    query_override: Option<&str>,
    mode: ScreenshotMode,
) -> Result<String> {
    if matches!(mode, ScreenshotMode::Window)
        && query_override.is_none()
        && let Ok(bytes) = devtools::capture_screenshot().await
    {
        let output = Path::new(output_path);

        if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }

        fs::write(output, bytes)
            .with_context(|| format!("failed to write screenshot {}", output.display()))?;
        return Ok(format!(
            "saved {} from Chrome DevTools page target",
            output.display()
        ));
    }

    let browser_window = match mode {
        ScreenshotMode::Window => window::find_browser_window(query_override)?,
        ScreenshotMode::Active => {
            let active_id = crate::window::active_window_id()?;
            crate::window::list_visible_windows()?
                .into_iter()
                .find(|candidate| candidate.id == active_id)
                .ok_or_else(|| anyhow!("active window {} is not visible", active_id))?
        }
    };
    let output = Path::new(output_path);

    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    let capture = std::process::Command::new("import")
        .args(["-window", &browser_window.id, output_path])
        .output()
        .context("failed to run import")?;

    if !capture.status.success() {
        return Err(anyhow!(
            "import failed for window {}: {}",
            browser_window.id,
            String::from_utf8_lossy(&capture.stderr).trim()
        ));
    }

    Ok(format!(
        "saved {} from window {} ({}, {}x{} at {},{})",
        output.display(),
        browser_window.id,
        browser_window.name,
        browser_window.width,
        browser_window.height,
        browser_window.x,
        browser_window.y
    ))
}
