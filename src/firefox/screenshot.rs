use anyhow::Result;

pub use crate::chrome::screenshot::ScreenshotMode;

pub async fn capture(output_path: &str, query_override: Option<&str>) -> Result<String> {
    capture_mode(output_path, query_override, ScreenshotMode::Window).await
}

pub async fn capture_mode(
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
        )
        .await,
    }
}

fn capture_window(output_path: &str, window: &crate::window::WindowMatch) -> Result<String> {
    use std::{fs, path::Path};

    let output = Path::new(output_path);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let capture = std::process::Command::new("import")
        .args(["-window", &window.id, output_path])
        .output()?;

    if !capture.status.success() {
        anyhow::bail!(
            "import failed for window {}: {}",
            window.id,
            String::from_utf8_lossy(&capture.stderr).trim()
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
