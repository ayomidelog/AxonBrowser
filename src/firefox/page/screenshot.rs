use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, anyhow, bail};

use crate::firefox::{page::root::PageScope, screenshot};

use super::actions::target::PageActionTarget;

pub async fn capture(
    scope: &PageScope,
    output_path: &str,
    raw_selectors: &[String],
    nth: Option<usize>,
) -> Result<String> {
    if raw_selectors.is_empty() {
        return screenshot::capture(output_path, None);
    }

    let target = PageActionTarget::resolve_nth(scope, raw_selectors, nth).await?;
    let browser_window = target.browser_window().await?;
    let (x, y, width, height) = crate::inspect::component_extents(&target.node).await?;
    if width <= 0 || height <= 0 {
        bail!("page target {} has non-visible extents", target.label);
    }

    let crop_x = x - browser_window.x;
    let crop_y = y - browser_window.y;
    let max_width = i32::try_from(browser_window.width).unwrap_or(i32::MAX);
    let max_height = i32::try_from(browser_window.height).unwrap_or(i32::MAX);
    if crop_x < 0 || crop_y < 0 || crop_x >= max_width || crop_y >= max_height {
        bail!(
            "page target {} falls outside browser window bounds",
            target.label
        );
    }

    let crop_w = width.min(max_width - crop_x);
    let crop_h = height.min(max_height - crop_y);
    if crop_w <= 0 || crop_h <= 0 {
        bail!("page target {} crop area is empty", target.label);
    }

    let tmp = temp_png_path();
    let tmp_str = tmp.to_string_lossy().to_string();
    screenshot::capture(&tmp_str, Some(&browser_window.name))?;

    if let Some(parent) = Path::new(output_path)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let geometry = format!("{}x{}+{}+{}", crop_w, crop_h, crop_x, crop_y);
    let output = std::process::Command::new("convert")
        .args([&tmp_str, "-crop", &geometry, "+repage", output_path])
        .output()?;
    let _ = fs::remove_file(&tmp);

    if !output.status.success() {
        return Err(anyhow!(
            "convert crop failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(format!(
        "saved {} for {} cropped to {} from window {}",
        output_path, target.label, geometry, browser_window.id
    ))
}

fn temp_png_path() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("axonbrowser-page-shot-{}.png", stamp))
}
