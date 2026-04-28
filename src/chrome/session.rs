use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

const TARGET_WINDOW_FILE: &str = "axonbrowser-target-window";

pub fn remember_browser_window_target(window_id: &str) -> Result<()> {
    let path = target_window_file();
    fs::write(&path, window_id.trim()).with_context(|| {
        format!(
            "failed to write target browser window file {}",
            path.display()
        )
    })
}

pub fn read_browser_window_target() -> Option<String> {
    let path = target_window_file();
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn target_window_file() -> PathBuf {
    std::env::temp_dir().join(TARGET_WINDOW_FILE)
}
