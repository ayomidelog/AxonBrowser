use anyhow::Result;

pub use crate::chrome::screenshot::ScreenshotMode;

pub fn capture(output_path: &str, query_override: Option<&str>) -> Result<String> {
    capture_mode(output_path, query_override, ScreenshotMode::Window)
}

pub fn capture_mode(
    output_path: &str,
    query_override: Option<&str>,
    mode: ScreenshotMode,
) -> Result<String> {
    match mode {
        ScreenshotMode::Window => {
            let window = crate::firefox::window::find_firefox_window(query_override)?;
            capture_window(output_path, &window)
        }
        ScreenshotMode::Active => crate::chrome::screenshot::capture_mode(
            output_path,
            query_override,
            ScreenshotMode::Active,
        ),
    }
}

fn capture_window(output_path: &str, window: &crate::window::WindowMatch) -> Result<String> {
    use std::{fs, path::Path, process::Stdio};

    let output = Path::new(output_path);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let xwd = std::process::Command::new("xwd")
        .args(["-silent", "-id", &window.id])
        .output()?;

    if !xwd.status.success() {
        anyhow::bail!(
            "xwd failed for window {}: {}",
            window.id,
            String::from_utf8_lossy(&xwd.stderr).trim()
        );
    }

    let mut convert = std::process::Command::new("convert")
        .args(["xwd:-", output_path])
        .stdin(Stdio::piped())
        .spawn()?;

    use std::io::Write;
    if let Some(stdin) = convert.stdin.as_mut() {
        stdin.write_all(&xwd.stdout)?;
    }

    let convert_output = convert.wait_with_output()?;
    if !convert_output.status.success() {
        anyhow::bail!(
            "convert failed: {}",
            String::from_utf8_lossy(&convert_output.stderr).trim()
        );
    }

    Ok(format!(
        "saved {} from window {} ({}, {}x{} at {},{})",
        output.display(),
        window.id,
        window.name,
        window.width,
        window.height,
        window.x,
        window.y
    ))
}
