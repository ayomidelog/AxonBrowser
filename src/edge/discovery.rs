use std::path::PathBuf;

use anyhow::Result;

use crate::window;

pub fn remember_profile_from_window(window_id: &str) -> Result<Option<String>> {
    let pid = match window::window_pid(window_id) {
        Ok(pid) => pid,
        Err(_) => return Ok(None),
    };
    let cmdline = match window::pid_cmdline(pid) {
        Ok(cmdline) => cmdline,
        Err(_) => return Ok(None),
    };
    let Some(profile) = parse_user_data_dir(&cmdline) else {
        return Ok(None);
    };
    crate::edge::session::remember_browser_profile(&profile)?;
    Ok(Some(profile))
}

fn parse_user_data_dir(cmdline: &str) -> Option<String> {
    for token in cmdline.split_whitespace() {
        if let Some(value) = token.strip_prefix("--user-data-dir=") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed).display().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_user_data_dir;

    #[test]
    fn parses_profile_arg_from_cmdline() {
        let cmdline = "/path/to/microsoft-edge --user-data-dir=/tmp/edge-profile --remote-debugging-port=0";
        assert_eq!(
            parse_user_data_dir(cmdline).as_deref(),
            Some("/tmp/edge-profile")
        );
    }
}
