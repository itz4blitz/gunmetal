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
    /// A request body was larger than its route allows, and was refused before it was decoded (SEC-API-060).
    BodyTooLarge = ("body_too_large", Some(413), "That request is too large."),
    /// A request carried a credential somewhere other than the session cookie or the Authorization header, or carried two (SEC-API-004, SEC-EXT-006).
    CredentialMisplaced = ("credential_misplaced", Some(400), "Send credentials only in the Authorization header or the session cookie, and only once."),
    /// A cookie-authenticated request did not come from the server's own pages (SEC-API-033, SEC-API-034).
    CrossSiteRequest = ("cross_site_request", Some(403), "This request was refused because it didn't come from this server's own pages."),
    /// An `ID3v1` or APE tag at the end of a file could not be read and was skipped; the rest of the file was kept (SEC-MED-017).
    EndTagSkipped = ("end_tag_skipped", None, "We couldn't read a tag at the end of this file, so we skipped it."),
    /// A user event's clock is further ahead of the server's time than the skew bound allows, so the event was refused (ADR 3, section 6).
    EventClockAhead = ("event_clock_ahead", Some(400), "This device's clock is set ahead of the server's, so the change wasn't recorded. Check the device's date and time."),
    /// A stored user event could not be read: its bytes are damaged or are not an event. It is reported, never guessed at (ADR 3, section 4).
    EventMalformed = ("event_malformed", None, "A saved listening or library change is damaged and couldn't be read."),
    /// One optional part of a media file, such as a tag, a picture or its lyrics, could not be read and was skipped; the file's other facts were kept and it stays playable (SEC-MED-017).
    FilePartSkipped = ("file_part_skipped", None, "We skipped a part of this file that we couldn't read. The rest of it was kept."),
    /// A media file could not be probed: its format is not one we read, or the part of it that playback needs is damaged; the problem's argument gives the reason.
    FileUnreadable = ("file_unreadable", None, "We couldn't read this file. It may be damaged, or in a format we don't support."),
    /// The caller is signed in but may not do this (SEC-IAM-068).
    Forbidden = ("forbidden", Some(403), "You don't have permission to do that."),
    /// Request handling failed unexpectedly. Nothing about the failure reaches the client (SEC-API-073, SEC-TM-040).
    InternalError = ("internal_error", Some(500), "Something went wrong on the server. Try again later."),
    /// A typed one-time code is missing, the wrong kind, mistyped or has a bad checksum.
    InvalidCode = ("invalid_code", Some(400), "That code is not valid. Check it and try again."),
    /// A request's parameters or body were not in the form its route expects (SEC-API-067).
    InvalidRequest = ("invalid_request", Some(400), "The request wasn't in the expected form."),
    /// A folder offered as a library root, or as an approved link target, is the filesystem root (SEC-MED-037).
    LibraryRootFilesystemRoot = ("library_root_filesystem_root", Some(400), "This is the top of the file system, which holds everything on this computer. Choose the folder that holds your media."),
    /// A folder offered as a library root, or as an approved link target, equals, contains or lies inside one of Gunmetal's own data, cache, configuration or log directories (SEC-MED-037).
    LibraryRootOwnData = ("library_root_own_data", Some(400), "This folder overlaps the folders where Gunmetal keeps its own data, so other people could download that data. Choose another folder."),
    /// A folder offered as a library root, or as an approved link target, is a system directory or lies inside one (SEC-MED-037).
    LibraryRootSystemFolder = ("library_root_system_folder", Some(400), "This is a system folder, not a media folder. Choose the folder that holds your media."),
    /// A capability URL's expiry has passed, so the client should refresh it and retry (SEC-API-027).
    MediaUrlExpired = ("media_url_expired", Some(401), "This media link has expired. Refresh it and try again."),
    /// The route does not take this method (SEC-API-008).
    MethodNotAllowed = ("method_not_allowed", Some(405), "That action isn't available here."),
    /// No object with this identifier is visible to the caller: it does not exist, the caller may not see it, or the identifier is malformed or of another kind. All of these get this one answer (SEC-API-011, SEC-API-024).
    NotFound = ("not_found", Some(404), "We couldn't find that. It may have been removed, or you may not have access to it."),
    /// An Ogg Opus stream's identification or comment header could not be read; the scan records it against the file (SEC-MED-017).
    OpusHeaderUnreadable = ("opus_header_unreadable", None, "This file's Opus stream headers are damaged or use a version we can't read."),
    /// A queue operation does not fit the queue as it is: it names an entry that is not there, adds an entry whose ID is taken, or has nothing to act on. It changed nothing.
    QueueRefused = ("queue_refused", Some(409), "That change doesn't fit the queue as it is now, so it wasn't made."),
    /// A queue operation was built on an older version of the queue. It changed nothing; the client rebuilds it on the current version.
    QueueStale = ("queue_stale", Some(409), "The queue changed somewhere else first, so this change wasn't made."),
    /// An MP4 file's sample tables, which map play times to positions in the file, could not be joined into a seek index (WP-018).
    SampleTableDamaged = ("sample_table_damaged", None, "The part of this file that maps play times to positions is damaged, so seeking in it won't work."),
    /// The action needs an administrator session or a fresh check with a passkey first (SEC-IAM-041, SEC-TM-017).
    StepUpRequired = ("step_up_required", Some(403), "Confirm it's you with your passkey, then try again."),
    /// The request named a host the server does not answer to (SEC-API-007, SEC-NET-014).
    UnknownHost = ("unknown_host", Some(421), "This server doesn't answer to that name."),
    /// A request body was not JSON, or was compressed (SEC-API-035, SEC-API-065).
    UnsupportedBody = ("unsupported_body", Some(415), "The request body must be JSON, sent without compression."),
    /// An Ogg Vorbis stream's identification or comment header could not be read; the scan records it against the file (SEC-MED-017).
    VorbisHeaderUnreadable = ("vorbis_header_unreadable", None, "This file's Vorbis stream headers are damaged or use a version we can't read."),
    /// A WAV file is damaged, or is not one, so it cannot be played; the problem's arguments give the reason and where in the file it was found.
    WavUnreadable = ("wav_unreadable", None, "We couldn't read this WAV file. It may be damaged, or written in a way we don't support."),
    /// Passkey authenticator data, a COSE key or the CBOR that carries them could not be read (SEC-MED-001).
    WebauthnDataUnreadable = ("webauthn_data_unreadable", None, "That passkey data could not be read."),
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
    const CATALOGUE: [(&str, Option<u16>, &str); 32] = [
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
        ("body_too_large", Some(413), "That request is too large."),
        (
            "credential_misplaced",
            Some(400),
            "Send credentials only in the Authorization header or the session cookie, and only once.",
        ),
        (
            "cross_site_request",
            Some(403),
            "This request was refused because it didn't come from this server's own pages.",
        ),
        (
            "end_tag_skipped",
            None,
            "We couldn't read a tag at the end of this file, so we skipped it.",
        ),
        (
            "event_clock_ahead",
            Some(400),
            "This device's clock is set ahead of the server's, so the change wasn't recorded. Check the device's date and time.",
        ),
        (
            "event_malformed",
            None,
            "A saved listening or library change is damaged and couldn't be read.",
        ),
        (
            "file_part_skipped",
            None,
            "We skipped a part of this file that we couldn't read. The rest of it was kept.",
        ),
        (
            "file_unreadable",
            None,
            "We couldn't read this file. It may be damaged, or in a format we don't support.",
        ),
        (
            "forbidden",
            Some(403),
            "You don't have permission to do that.",
        ),
        (
            "internal_error",
            Some(500),
            "Something went wrong on the server. Try again later.",
        ),
        (
            "invalid_code",
            Some(400),
            "That code is not valid. Check it and try again.",
        ),
        (
            "invalid_request",
            Some(400),
            "The request wasn't in the expected form.",
        ),
        (
            "library_root_filesystem_root",
            Some(400),
            "This is the top of the file system, which holds everything on this computer. Choose the folder that holds your media.",
        ),
        (
            "library_root_own_data",
            Some(400),
            "This folder overlaps the folders where Gunmetal keeps its own data, so other people could download that data. Choose another folder.",
        ),
        (
            "library_root_system_folder",
            Some(400),
            "This is a system folder, not a media folder. Choose the folder that holds your media.",
        ),
        (
            "media_url_expired",
            Some(401),
            "This media link has expired. Refresh it and try again.",
        ),
        (
            "method_not_allowed",
            Some(405),
            "That action isn't available here.",
        ),
        (
            "not_found",
            Some(404),
            "We couldn't find that. It may have been removed, or you may not have access to it.",
        ),
        (
            "opus_header_unreadable",
            None,
            "This file's Opus stream headers are damaged or use a version we can't read.",
        ),
        (
            "queue_refused",
            Some(409),
            "That change doesn't fit the queue as it is now, so it wasn't made.",
        ),
        (
            "queue_stale",
            Some(409),
            "The queue changed somewhere else first, so this change wasn't made.",
        ),
        (
            "sample_table_damaged",
            None,
            "The part of this file that maps play times to positions is damaged, so seeking in it won't work.",
        ),
        (
            "step_up_required",
            Some(403),
            "Confirm it's you with your passkey, then try again.",
        ),
        (
            "unknown_host",
            Some(421),
            "This server doesn't answer to that name.",
        ),
        (
            "unsupported_body",
            Some(415),
            "The request body must be JSON, sent without compression.",
        ),
        (
            "vorbis_header_unreadable",
            None,
            "This file's Vorbis stream headers are damaged or use a version we can't read.",
        ),
        (
            "wav_unreadable",
            None,
            "We couldn't read this WAV file. It may be damaged, or written in a way we don't support.",
        ),
        (
            "webauthn_data_unreadable",
            None,
            "That passkey data could not be read.",
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

    /// Words that name what a client must never be told about: the
    /// language, the database, a query, a stack trace or a panic. Each is
    /// matched as a whole word, in any case, so each inflected form is
    /// listed too ("selected" is left out: a client may select a folder).
    const FORBIDDEN_WORDS: [&str; 22] = [
        "backtrace",
        "panic",
        "panicked",
        "panicking",
        "panics",
        "rust",
        "rustc",
        "select",
        "selects",
        "sql",
        "sqlite",
        "sqlstate",
        "stack",
        "stacks",
        "stacktrace",
        "trace",
        "traceback",
        "traces",
        "unwrap",
        "unwrapped",
        "unwrapping",
        "unwraps",
    ];

    /// The forbidden words `text` holds, in the order it holds them. A
    /// word is a run of ASCII letters, so "trust" does not hold "rust".
    fn forbidden_words(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_ascii_alphabetic())
            .map(str::to_ascii_lowercase)
            .filter(|word| FORBIDDEN_WORDS.contains(&word.as_str()))
            .collect()
    }

    #[test]
    fn finds_forbidden_words_only_as_whole_words() {
        assert_eq!(
            forbidden_words("You can trust the selected folder. It's frustrating, we know."),
            [""; 0]
        );
        assert_eq!(
            forbidden_words("Retrace your steps, then restack the haystack."),
            [""; 0]
        );
        assert_eq!(
            forbidden_words("A Rust panic: SELECT failed, see the stack trace."),
            ["rust", "panic", "select", "stack", "trace"]
        );
        assert_eq!(
            forbidden_words("SQLite panicked. Backtrace-stacktrace,traceback!Unwrap?sql"),
            [
                "sqlite",
                "panicked",
                "backtrace",
                "stacktrace",
                "traceback",
                "unwrap",
                "sql"
            ]
        );
        assert_eq!(
            forbidden_words(
                "It panics, panicking; traces, stacks and selects. Unwrapped, unwrapping, unwraps: rustc sqlstate."
            ),
            [
                "panics",
                "panicking",
                "traces",
                "stacks",
                "selects",
                "unwrapped",
                "unwrapping",
                "unwraps",
                "rustc",
                "sqlstate"
            ]
        );
    }

    /// Verifies: SEC-API-072
    #[test]
    fn texts_hold_no_version_path_sql_or_trace() {
        // Letters, spaces and sentence punctuation only: no digit can spell
        // a version, no slash a path, and no bracket, colon or semicolon a
        // stack trace or a statement.
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
        }
        let mentions: Vec<(ProblemCode, Vec<String>)> = ProblemCode::ALL
            .iter()
            .map(|code| (*code, forbidden_words(code.text())))
            .filter(|(_, words)| !words.is_empty())
            .collect();
        assert_eq!(mentions, []);
    }
}
