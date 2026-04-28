use std::{fs, path::Path};

use anyhow::{Context, Result, anyhow};

use super::window;

#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMode {
    Window,
    Active,
}

pub fn capture(output_path: &str, query_override: Option<&str>) -> Result<String> {
    capture_mode(output_path, query_override, ScreenshotMode::Window)
}

pub fn capture_mode(
    output_path: &str,
    query_override: Option<&str>,
    mode: ScreenshotMode,
) -> Result<String> {
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

    let xwd = std::process::Command::new("xwd")
        .args(["-silent", "-id", &browser_window.id])
        .output()
        .with_context(|| format!("failed to run xwd for window {}", browser_window.id))?;

    if !xwd.status.success() {
        return Err(anyhow!(
            "xwd failed for window {}: {}",
            browser_window.id,
            String::from_utf8_lossy(&xwd.stderr).trim()
        ));
    }

    let mut convert = std::process::Command::new("convert")
        .args(["xwd:-", output_path])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("failed to start ImageMagick convert")?;

    use std::io::Write;
    if let Some(stdin) = convert.stdin.as_mut() {
        stdin
            .write_all(&xwd.stdout)
            .context("failed to write screenshot data into convert stdin")?;
    }

    let convert_output = convert
        .wait_with_output()
        .context("failed waiting for convert process")?;

    if !convert_output.status.success() {
        return Err(anyhow!(
            "convert failed: {}",
            String::from_utf8_lossy(&convert_output.stderr).trim()
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
