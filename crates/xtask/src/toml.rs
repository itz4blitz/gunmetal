//! The little of TOML the checks read: one `key = "value"` line at a time.
//! Cargo writes `Cargo.lock` and the manifests the checks read in this
//! shape. Anything else on a line, such as a trailing comment, makes the
//! line unreadable, and each check treats an unreadable line as absent.

/// The quoted string `line` assigns to `key`, or `None` when the line
/// assigns something else.
pub fn value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.trim()
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim_start()
        .strip_prefix('"')?
        .strip_suffix('"')
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn reads_the_quoted_value_of_its_key() {
        assert_eq!(value(r#"name = "ebml""#, "name"), Some("ebml"));
        assert_eq!(value(r#"  version = "1.0.0"  "#, "version"), Some("1.0.0"));
        assert_eq!(value(r#"name="tight""#, "name"), Some("tight"));
        assert_eq!(value(r#"name = """#, "name"), Some(""));
    }

    #[test]
    fn reads_nothing_from_any_other_line() {
        assert_eq!(value(r#"version = "1.0.0""#, "name"), None);
        assert_eq!(value(r#"names = "ebml""#, "name"), None);
        assert_eq!(value("name = 3", "name"), None);
        assert_eq!(value(r#"name = "ebml" # comment"#, "name"), None);
        assert_eq!(value("name", "name"), None);
        assert_eq!(value("[[bin]]", "name"), None);
    }
}
