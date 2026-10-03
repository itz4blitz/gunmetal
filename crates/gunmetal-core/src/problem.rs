//! The problem catalogue: every error the system can report, with a stable
//! code, its HTTP status where it has one, and its plain-language English
//! text.
//!
//! Core errors do not implement `Display`. Each implements [`Describe`],
//! which maps it to a [`Problem`]: a [`ProblemCode`] from this closed
//! catalogue and any typed arguments. The server renders a problem as an
//! RFC 9457 problem-details object whose `type` comes from the code and
//! whose text comes from here, so no error reaches a client as free text
//! (SEC-API-072), and the wording people read lives in one tested place.
//!
//! This file is a registry. A package that needs a new code adds one entry
//! to the catalogue below, in sorted order of its code, and one row to the
//! literal table in the tests.

/// Declares the catalogue. Each entry is a documented variant, its stable
/// code, its HTTP status where it has one, and its text, so that one line
/// holds everything about a code.
///
/// Mutation testing does not see code a macro generates, and coverage does
/// not count its match arms one by one, so the tests pin every generated
/// value against a literal table instead.
macro_rules! problems {
    ($(
        $(#[$doc:meta])*
        $variant:ident = ($code:literal, $status:expr, $text:literal),
    )*) => {
        /// A stable code for one error the system can report.
        ///
        /// The set is closed: an error that is not in the catalogue cannot
        /// be described, so it cannot be reported (SEC-API-072).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ProblemCode {
            $( $(#[$doc])* $variant, )*
        }

        impl ProblemCode {
            /// Every code in the catalogue, in its sorted order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The stable code, in lower case letters and underscores. The
            /// server uses it as the problem type, and clients key their
            /// translations on it.
            #[must_use]
            pub const fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// The HTTP status of a response that carries this problem, for
            /// problems that can reach an HTTP response.
            #[must_use]
            pub const fn status(self) -> Option<u16> {
                match self {
                    $(Self::$variant => $status,)*
                }
            }

            /// The plain-language English text people see.
            #[must_use]
            pub const fn text(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }
        }
    };
}

problems! {
    /// An AIFF or AIFF-C file is damaged, or is not one, so it cannot be played; the problem's arguments give the reason and where in the file it was found.
    AiffUnreadable = ("aiff_unreadable", None, "We couldn't read this AIFF file. It may be damaged, or written in a way we don't support."),
    /// One item of an APE tag was skipped, or reading its items stopped early; the tag's other items and the rest of the file were kept (SEC-MED-017).
    ApeItemSkipped = ("ape_item_skipped", None, "We skipped a damaged field in this file's APE tag."),
    /// The audit log could not write the record of a security event, so the action it records did not take effect (SEC-OPS-020).
    AuditUnavailable = ("audit_unavailable", Some(503), "We couldn't record this action, so it didn't happen. Try again later."),
    /// A trusted proxy sent a forwarding chain the server could not read: malformed, over long, or in both chain headers at once (SEC-NET-018).
    BadForwardingHeader = ("bad_forwarding_header", Some(400), "A proxy in front of this server sent forwarding information we couldn't read."),
    /// An `ID3v1` or APE tag at the end of a file could not be read and was skipped; the rest of the file was kept (SEC-MED-017).
    EndTagSkipped = ("end_tag_skipped", None, "We couldn't read a tag at the end of this file, so we skipped it."),
    /// A typed one-time code is missing, the wrong kind, mistyped or has a bad checksum.
    InvalidCode = ("invalid_code", Some(400), "That code is not valid. Check it and try again."),
    /// No object with this identifier is visible to the caller: it does not exist, the caller may not see it, or the identifier is malformed or of another kind. All of these get this one answer (SEC-API-011, SEC-API-024).
    NotFound = ("not_found", Some(404), "We couldn't find that. It may have been removed, or you may not have access to it."),
    /// A WAV file is damaged, or is not one, so it cannot be played; the problem's arguments give the reason and where in the file it was found.
    WavUnreadable = ("wav_unreadable", None, "We couldn't read this WAV file. It may be damaged, or written in a way we don't support."),
}

/// A typed value that a problem carries alongside its code.
///
/// There is no variant for free text, so an argument can never echo text a
/// client submitted or a file contained (SEC-API-072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// A number, such as an offset or a limit.
    Number(u64),
    /// A name fixed in the program, such as a field or a limit.
    Name(&'static str),
}

/// What an error reports: its code from the catalogue and any arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Which catalogue entry the error is.
    pub code: ProblemCode,
    /// Named values that say more about this occurrence.
    pub args: Vec<(&'static str, Arg)>,
}

/// Implemented by every error type in the core, in place of `Display`.
pub trait Describe {
    /// The problem this error reports.
    fn problem(&self) -> Problem;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole catalogue, written out independently of the declaration
    /// above: code, status and text of every entry, in order.
    const CATALOGUE: [(&str, Option<u16>, &str); 8] = [
        (
            "aiff_unreadable",
            None,
            "We couldn't read this AIFF file. It may be damaged, or written in a way we don't support.",
        ),
        (
            "ape_item_skipped",
            None,
            "We skipped a damaged field in this file's APE tag.",
        ),
        (
            "audit_unavailable",
            Some(503),
            "We couldn't record this action, so it didn't happen. Try again later.",
        ),
        (
            "bad_forwarding_header",
            Some(400),
            "A proxy in front of this server sent forwarding information we couldn't read.",
        ),
        (
            "end_tag_skipped",
            None,
            "We couldn't read a tag at the end of this file, so we skipped it.",
        ),
        (
            "invalid_code",
            Some(400),
            "That code is not valid. Check it and try again.",
        ),
        (
            "not_found",
            Some(404),
            "We couldn't find that. It may have been removed, or you may not have access to it.",
        ),
        (
            "wav_unreadable",
            None,
            "We couldn't read this WAV file. It may be damaged, or written in a way we don't support.",
        ),
    ];

    /// Every status a catalogue entry may carry.
    const ALLOWED_STATUSES: [u16; 13] = [
        400, 401, 403, 404, 405, 409, 413, 415, 416, 421, 429, 500, 503,
    ];

    /// Verifies: SEC-API-072
    #[test]
    fn the_catalogue_holds_exactly_the_listed_entries() {
        let declared: Vec<_> = ProblemCode::ALL
            .iter()
            .map(|code| (code.code(), code.status(), code.text()))
            .collect();
        assert_eq!(declared, CATALOGUE);
    }

    /// Verifies: SEC-API-072
    #[test]
    fn codes_are_unique_sorted_lower_case_words() {
        let codes: Vec<&str> = ProblemCode::ALL.iter().map(|code| code.code()).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(codes, sorted, "codes must be unique and in sorted order");
        for code in codes {
            let shaped = code.starts_with(|c: char| c.is_ascii_lowercase())
                && code.ends_with(|c: char| c.is_ascii_lowercase())
                && code.chars().all(|c| c.is_ascii_lowercase() || c == '_');
            assert!(shaped, "code {code:?}");
        }
    }

    #[test]
    fn statuses_come_from_the_allowed_list() {
        let outside: Vec<(ProblemCode, u16)> = ProblemCode::ALL
            .iter()
            .filter_map(|code| code.status().map(|status| (*code, status)))
            .filter(|(_, status)| !ALLOWED_STATUSES.contains(status))
            .collect();
        assert_eq!(outside, []);
    }

    /// Verifies: SEC-API-072
    #[test]
    fn texts_hold_no_version_path_sql_or_trace() {
        // Letters, spaces and sentence punctuation only: no digit can spell
        // a version, no slash a path, and no bracket, colon or semicolon a
        // stack trace or a statement.
        const FORBIDDEN_WORDS: [&str; 6] = ["panic", "rust", "select", "sql", "stack", "trace"];
        for code in ProblemCode::ALL {
            let text = code.text();
            assert!(
                text.chars()
                    .all(|c| c.is_ascii_alphabetic() || " .,'?!-".contains(c)),
                "{code:?}: {text:?}"
            );
            assert!(
                text.starts_with(|c: char| c.is_ascii_uppercase()) && text.ends_with('.'),
                "{code:?}: {text:?}"
            );
            let lower = text.to_ascii_lowercase();
            for word in FORBIDDEN_WORDS {
                assert!(!lower.contains(word), "{code:?} mentions {word:?}");
            }
        }
    }
}
