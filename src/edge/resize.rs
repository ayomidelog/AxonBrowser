pub use crate::chrome::resize::ResizePreset;

pub fn resize(
    query_override: Option<&str>,
    preset: Option<ResizePreset>,
    width: Option<u32>,
    height: Option<u32>,
) -> anyhow::Result<String> {
    let (width, height) = match (preset, width, height) {
        (Some(preset), None, None) => preset.dimensions(),
        (None, Some(width), Some(height)) => (width, height),
        (Some(_), Some(_), Some(_)) => {
            anyhow::bail!("edge resize accepts either a preset or --width/--height, not both")
        }
        _ => anyhow::bail!("edge resize needs a preset or both --width and --height"),
    };

    if width == 0 || height == 0 {
        anyhow::bail!("edge resize dimensions must be greater than zero");
    }

    let browser_window = crate::edge::window::find_edge_window(query_override)?;
    crate::window::resize_window(&browser_window.id, width, height)?;

    Ok(format!(
        "resized window {} ({}) to {}x{}",
        browser_window.id, browser_window.name, width, height
    ))
}
