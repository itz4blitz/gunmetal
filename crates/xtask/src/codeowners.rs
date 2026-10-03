//! Who `.github/CODEOWNERS` names as the owners of a path.
//!
//! GitHub reads the file top to bottom and the last line whose pattern
//! matches the path decides; a line with a pattern and no owners leaves the
//! path unowned. Patterns follow `.gitignore`: a pattern with a `/` at its
//! start or in its middle is anchored to the repository root, any other
//! pattern matches at any depth, and a trailing `/` matches only a
//! directory's contents. Only literal patterns are read. A pattern with a
//! wildcard, a character class, a negation or an escape makes the whole
//! file unreadable, because a check that guessed its meaning could hand
//! ownership to the wrong person.

/// The owners the last matching line of `codeowners` gives `path`, as
/// written (`@login`, `@org/team` or an email address), or the number of
/// the first line, counted from 1, whose pattern this module cannot read.
pub fn owners<'a>(codeowners: &'a str, path: &str) -> Result<Vec<&'a str>, usize> {
    let mut found = Vec::new();
    for (index, line) in codeowners.lines().enumerate() {
        let mut words = line
            .split_whitespace()
            .take_while(|word| !word.starts_with('#'));
        let Some(pattern) = words.next() else {
            continue;
        };
        if pattern.contains(['*', '?', '[', '!', '\\']) {
            return Err(index + 1);
        }
        if matches(pattern, path) {
            found = words.collect();
        }
    }
    Ok(found)
}

/// Whether literal `pattern` matches the file `path`.
fn matches(pattern: &str, path: &str) -> bool {
    let anchored = pattern.trim_end_matches('/').contains('/');
    let directory = pattern.ends_with('/');
    let pattern = pattern.trim_matches('/');
    let mut starts = vec![path];
    if !anchored {
        starts.extend(path.match_indices('/').map(|(at, _)| &path[at + 1..]));
    }
    starts
        .into_iter()
        .any(|rest| match rest.strip_prefix(pattern) {
            Some("") => !directory,
            Some(after) => after.starts_with('/'),
            None => false,
        })
}

#[cfg(test)]
mod tests {
    use super::owners;

    /// Whether the one-line file `pattern @owner` gives `path` an owner.
    fn matches(pattern: &str, path: &str) -> bool {
        owners(&format!("{pattern} @owner\n"), path) == Ok(vec!["@owner"])
    }

    #[test]
    fn the_last_matching_line_decides() {
        let codeowners = "\
# Code owners.

/.github/ @ci
Cargo.lock @lock-owner @PremierStudio/maintainers # every lock file
/fuzz/ @fuzz-owner
/docs/
";
        assert_eq!(
            owners(codeowners, "Cargo.lock"),
            Ok(vec!["@lock-owner", "@PremierStudio/maintainers"])
        );
        assert_eq!(
            owners(codeowners, "fuzz/Cargo.lock"),
            Ok(vec!["@fuzz-owner"])
        );
        assert_eq!(owners(codeowners, ".github/CODEOWNERS"), Ok(vec!["@ci"]));
        assert_eq!(owners(codeowners, "docs/decisions.md"), Ok(vec![]));
        assert_eq!(owners(codeowners, "README.md"), Ok(vec![]));
    }

    #[test]
    fn a_later_line_without_owners_leaves_the_path_unowned() {
        assert_eq!(
            owners("Cargo.lock @a\n/Cargo.lock\n", "Cargo.lock"),
            Ok(vec![])
        );
        assert_eq!(
            owners("Cargo.lock @a\n/Cargo.lock # no owner\n", "Cargo.lock"),
            Ok(vec![])
        );
    }

    #[test]
    fn matches_patterns_as_gitignore_does() {
        for (pattern, path, expected) in [
            ("/Cargo.lock", "Cargo.lock", true),
            ("/Cargo.lock", "fuzz/Cargo.lock", false),
            ("Cargo.lock", "Cargo.lock", true),
            ("Cargo.lock", "fuzz/Cargo.lock", true),
            ("Cargo.lock", "fuzz/Cargo.lock.orig", false),
            ("lock", "fuzz/Cargo.lock", false),
            ("Cargo", "fuzz/Cargo.lock", false),
            ("uzz", "fuzz/Cargo.lock", false),
            ("fuzz", "fuzz/Cargo.lock", true),
            ("fuzz/", "fuzz/Cargo.lock", true),
            ("fuzz/", "fuzz", false),
            ("fuzz/", "a/fuzz/Cargo.lock", true),
            ("/fuzz/", "a/fuzz/Cargo.lock", false),
            ("a/fuzz", "a/fuzz/Cargo.lock", true),
            ("a/fuzz", "x/a/fuzz/Cargo.lock", false),
            ("/", "Cargo.lock", false),
        ] {
            assert_eq!(matches(pattern, path), expected, "{pattern} for {path}");
        }
    }

    #[test]
    fn a_pattern_it_cannot_read_makes_the_file_unreadable() {
        for pattern in [
            "*.lock",
            "Cargo.lo?k",
            "Cargo.loc[k]",
            "!Cargo.lock",
            "\\#lock",
        ] {
            let codeowners = format!("# Owners.\nCargo.lock @a\n{pattern} @b\nfuzz/ @c\n");
            assert_eq!(owners(&codeowners, "Cargo.lock"), Err(3), "{pattern}");
            assert_eq!(owners(&codeowners, "docs/x.md"), Err(3), "{pattern}");
        }
        assert_eq!(owners("#*.lock @a\n", "Cargo.lock"), Ok(vec![]));
    }
}
