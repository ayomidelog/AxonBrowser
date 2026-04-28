pub fn is_plausible_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return false;
    }

    trimmed.contains("://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("edge:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("view-source:")
        || (trimmed.contains('.') && !trimmed.starts_with("Push Button:"))
}

#[cfg(test)]
mod tests {
    use super::is_plausible_url;

    #[test]
    fn detects_url_like_values() {
        assert!(is_plausible_url("https://example.com"));
        assert!(is_plausible_url("about:blank"));
        assert!(is_plausible_url("example.com"));
        assert!(!is_plausible_url("Search or enter web address"));
        assert!(!is_plausible_url("Push Button:Reload"));
    }
}
