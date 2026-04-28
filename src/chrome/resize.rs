use anyhow::{Result, anyhow, bail};

use super::window;

#[derive(Debug, Clone, Copy)]
pub enum ResizePreset {
    Desktop,
    Tablet,
    Mobile,
}

impl ResizePreset {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Desktop => (1440, 900),
            Self::Tablet => (1024, 768),
            Self::Mobile => (430, 932),
        }
    }
}

pub fn resize(
    query_override: Option<&str>,
    preset: Option<ResizePreset>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<String> {
    let (width, height) = match (preset, width, height) {
        (Some(preset), None, None) => preset.dimensions(),
        (None, Some(width), Some(height)) => (width, height),
        (Some(_), Some(_), Some(_)) => {
            bail!("chrome resize accepts either a preset or --width/--height, not both")
        }
        _ => bail!("chrome resize needs a preset or both --width and --height"),
    };

    if width == 0 || height == 0 {
        return Err(anyhow!(
            "chrome resize dimensions must be greater than zero"
        ));
    }

    let browser_window = window::find_browser_window(query_override)?;
    crate::window::resize_window(&browser_window.id, width, height)?;

    Ok(format!(
        "resized window {} ({}) to {}x{}",
        browser_window.id, browser_window.name, width, height
    ))
}
