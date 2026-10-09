//! Callback state, per-user calls and returned text.
//!
//! A link callback is host-owned: bound to one plugin and one session,
//! single-use, and expired after ten minutes (SEC-EXT-031). A per-user call
//! is refused for a private session, for a person who has not turned the
//! plugin on, and for a scope the plugin or the person does not hold
//! (SEC-EXT-029, SEC-EXT-030). File names are passed without directories.
//! Plugin text is plain text, with a byte limit and a count limit
//! (SEC-EXT-028).

use crate::scope::Scope;

/// How long a callback state stays valid, in milliseconds. Ten minutes
/// (SEC-EXT-031).
pub const STATE_TTL_MS: i64 = 600_000;

/// The longest plugin text, in bytes (SEC-EXT-028).
pub const MAX_TEXT: usize = 1_024;

/// The most items a plugin list may return (SEC-EXT-028).
pub const MAX_ITEMS: usize = 20;

/// A host-owned link callback, bound to one plugin and one session.
///
/// It is single-use and expires ten minutes after it was begun
/// (SEC-EXT-031).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackState {
    /// The plugin the callback was begun for.
    pub plugin: String,
    /// The session the callback was begun for.
    pub session: String,
    /// The first instant, in milliseconds, at which the callback is expired.
    pub expires_at_ms: i64,
    /// Whether this state has already been taken.
    pub used: bool,
}

/// Why a callback state cannot be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackError {
    /// The caller has no state. [`take_callback`] does not report this;
    /// [`missing`] does.
    Missing,
    /// The state was begun for a different session.
    OtherSession,
    /// The state was already taken.
    Replayed,
    /// `now_ms` is at or after the state's expiry.
    Expired,
    /// The state was begun for a different plugin.
    OtherPlugin,
}

/// The error for a callback the caller does not have.
///
/// [`take_callback`] does not look a state up. A caller that has no state
/// returns this (SEC-EXT-031).
#[must_use]
pub const fn missing() -> CallbackError {
    CallbackError::Missing
}

/// Starts a host-owned callback bound to `plugin` and `session`.
///
/// The state expires at `now_ms` plus [`STATE_TTL_MS`] and has not been
/// used. Addition saturates at `i64::MAX`, so a clock at the end of the
/// range does not panic.
#[must_use]
pub fn begin_callback(plugin: &str, session: &str, now_ms: i64) -> CallbackState {
    CallbackState {
        plugin: plugin.to_owned(),
        session: session.to_owned(),
        expires_at_ms: now_ms.saturating_add(STATE_TTL_MS),
        used: false,
    }
}

/// Consumes a callback if it is this plugin's, this session's, unused, and
/// not yet expired.
///
/// The checks run in that order, so the earliest fault is the error. The
/// input is not changed. The accepted state is a copy with `used` set. A
/// state the caller does not have is [`missing`], not a lookup here.
///
/// # Errors
///
/// [`CallbackError::OtherPlugin`] when `plugin` differs, then
/// [`CallbackError::OtherSession`] when `session` differs, then
/// [`CallbackError::Replayed`] when the state was already taken, then
/// [`CallbackError::Expired`] when `now_ms` is at or after `expires_at_ms`.
#[must_use = "a refused callback must not be dropped"]
pub fn take_callback(
    state: &CallbackState,
    plugin: &str,
    session: &str,
    now_ms: i64,
) -> Result<CallbackState, CallbackError> {
    if state.plugin != plugin {
        return Err(CallbackError::OtherPlugin);
    }
    if state.session != session {
        return Err(CallbackError::OtherSession);
    }
    if state.used {
        return Err(CallbackError::Replayed);
    }
    if now_ms >= state.expires_at_ms {
        return Err(CallbackError::Expired);
    }
    Ok(CallbackState {
        plugin: state.plugin.clone(),
        session: state.session.clone(),
        expires_at_ms: state.expires_at_ms,
        used: true,
    })
}

/// Why a per-user host call is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallRefuse {
    /// The plugin was not granted the scope, or the person does not hold it.
    Scope,
    /// The session is private. Nothing is passed to the plugin.
    Private,
    /// The person has not turned the plugin on.
    NotOptedIn,
}

/// Whether a per-user host call may run.
///
/// A private session is refused even when every other check would pass.
/// Then the person must have turned the plugin on. Then `needed` must be in
/// `granted`, and the person must currently hold that scope. The checks run
/// in that order (SEC-EXT-029, SEC-EXT-030).
///
/// # Errors
///
/// [`CallRefuse::Private`] when `private_session` is set, then
/// [`CallRefuse::NotOptedIn`] when `opted_in` is not set, then
/// [`CallRefuse::Scope`] when `needed` is not in `granted` or
/// `user_has_scope` is not set.
#[must_use = "a refused per-user call must not be dropped"]
pub fn per_user_call(
    granted: &[Scope],
    needed: Scope,
    user_has_scope: bool,
    private_session: bool,
    opted_in: bool,
) -> Result<(), CallRefuse> {
    if private_session {
        return Err(CallRefuse::Private);
    }
    if !opted_in {
        return Err(CallRefuse::NotOptedIn);
    }
    if !granted.contains(&needed) {
        return Err(CallRefuse::Scope);
    }
    if !user_has_scope {
        return Err(CallRefuse::Scope);
    }
    Ok(())
}

/// Accepts `name` only when it is one file name.
///
/// [`file_name`] drops every separator before calling this. A result that
/// still contains `/` or `\` is refused here.
///
/// # Errors
///
/// `Err(())` when `name` is empty, `.`, `..`, or contains `/` or `\`.
fn checked_name(name: &str) -> Result<String, ()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        Err(())
    } else {
        Ok(name.to_owned())
    }
}

/// The file name at the end of `path`, with no directory.
///
/// The name is the text after the last `/` or `\`. `.` and `..` are
/// directories, not names (SEC-EXT-030).
///
/// # Errors
///
/// `Err(())` when the name is empty, `.`, `..`, or still contains `/` or
/// `\`. A trailing separator leaves an empty name.
#[must_use = "a refused file name must not be passed on"]
#[expect(
    clippy::result_unit_err,
    reason = "a path either names one file or it does not"
)]
pub fn file_name(path: &str) -> Result<String, ()> {
    let mut name = String::new();
    for ch in path.chars() {
        if ch == '/' || ch == '\\' {
            name.clear();
        } else {
            name.push(ch);
        }
    }
    checked_name(&name)
}

/// Why plugin text or a plugin list is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnRefuse {
    /// The text is longer than the byte limit.
    TooLong {
        /// The limit, in bytes.
        limit: usize,
        /// The text's length, in bytes.
        got: usize,
    },
    /// The list has more items than the count limit.
    TooMany {
        /// The limit.
        limit: usize,
        /// How many items were given.
        got: usize,
    },
    /// The text contains `<` or `>`.
    Markup,
}

/// Plugin text a client may render as plain text.
///
/// Longer than [`MAX_TEXT`] bytes is refused. A `<` or `>` is refused, not
/// stripped (SEC-EXT-028). Length is checked before markup.
///
/// # Errors
///
/// [`ReturnRefuse::TooLong`] when `text` is longer than [`MAX_TEXT`] bytes.
/// [`ReturnRefuse::Markup`] when `text` contains `<` or `>`. A long text
/// that also contains markup is too long.
#[must_use = "refused plugin text must not be stored"]
pub fn plain_text(text: &str) -> Result<&str, ReturnRefuse> {
    if text.len() > MAX_TEXT {
        return Err(ReturnRefuse::TooLong {
            limit: MAX_TEXT,
            got: text.len(),
        });
    }
    if text.contains('<') || text.contains('>') {
        return Err(ReturnRefuse::Markup);
    }
    Ok(text)
}

/// A list of plain-text slices, in the order given.
///
/// More than [`MAX_ITEMS`] items is refused before any item is read. An item
/// that fails [`plain_text`] fails the list with that error (SEC-EXT-028).
///
/// # Errors
///
/// [`ReturnRefuse::TooMany`] when there are more than [`MAX_ITEMS`] items.
/// Otherwise the [`ReturnRefuse`] from [`plain_text`] for the first item
/// that fails.
#[must_use = "a refused plugin list must not be stored"]
pub fn plain_list<'a>(items: &[&'a str]) -> Result<Vec<&'a str>, ReturnRefuse> {
    if items.len() > MAX_ITEMS {
        return Err(ReturnRefuse::TooMany {
            limit: MAX_ITEMS,
            got: items.len(),
        });
    }
    let mut kept = Vec::new();
    for item in items {
        kept.push(plain_text(item)?);
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::{
        CallRefuse, CallbackError, CallbackState, MAX_ITEMS, MAX_TEXT, ReturnRefuse, STATE_TTL_MS,
        begin_callback, file_name, missing, per_user_call, plain_list, plain_text, take_callback,
    };
    use crate::scope::Scope;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve `prop_oneof!` through `prelude::*`.
    use proptest::prop_assert_eq;
    use proptest::prop_oneof;
    use proptest::test_runner::{Config, TestRunner};

    /// Verifies: SEC-EXT-031
    #[test]
    fn a_callback_expires_ten_minutes_after_it_begins() {
        assert_eq!(STATE_TTL_MS, 600_000);
        assert_eq!(
            begin_callback("org.example.lyrics", "session-7", 1_000),
            CallbackState {
                plugin: "org.example.lyrics".to_owned(),
                session: "session-7".to_owned(),
                expires_at_ms: 601_000,
                used: false,
            }
        );
        assert_eq!(
            begin_callback("p", "s", 0),
            CallbackState {
                plugin: "p".to_owned(),
                session: "s".to_owned(),
                expires_at_ms: 600_000,
                used: false,
            }
        );
        assert_eq!(
            begin_callback("", "", -1_000),
            CallbackState {
                plugin: String::new(),
                session: String::new(),
                expires_at_ms: 599_000,
                used: false,
            }
        );
        assert_eq!(
            begin_callback("プラグイン", "セッション", i64::MIN),
            CallbackState {
                plugin: "プラグイン".to_owned(),
                session: "セッション".to_owned(),
                expires_at_ms: i64::MIN + 600_000,
                used: false,
            }
        );
        assert_eq!(
            begin_callback("p", "s", i64::MAX - 600_000),
            CallbackState {
                plugin: "p".to_owned(),
                session: "s".to_owned(),
                expires_at_ms: i64::MAX,
                used: false,
            }
        );
        assert_eq!(
            begin_callback("near", "end", i64::MAX - 599_999),
            CallbackState {
                plugin: "near".to_owned(),
                session: "end".to_owned(),
                expires_at_ms: i64::MAX,
                used: false,
            }
        );
    }

    /// Verifies: SEC-EXT-031
    #[test]
    fn taking_a_callback_returns_a_used_copy_and_leaves_the_input() {
        let state = CallbackState {
            plugin: "org.example.scrobbler".to_owned(),
            session: "sess-9".to_owned(),
            expires_at_ms: 5_000,
            used: false,
        };
        let before = state.clone();
        assert_eq!(
            take_callback(&state, "org.example.scrobbler", "sess-9", 4_999),
            Ok(CallbackState {
                plugin: "org.example.scrobbler".to_owned(),
                session: "sess-9".to_owned(),
                expires_at_ms: 5_000,
                used: true,
            })
        );
        assert_eq!(state, before);

        let begun = begin_callback("org.example.lyrics", "session-7", 1_000);
        let begun_before = begun.clone();
        assert_eq!(
            take_callback(&begun, "org.example.lyrics", "session-7", 600_999),
            Ok(CallbackState {
                plugin: "org.example.lyrics".to_owned(),
                session: "session-7".to_owned(),
                expires_at_ms: 601_000,
                used: true,
            })
        );
        assert_eq!(begun, begun_before);
        assert_eq!(
            take_callback(&begun, "org.example.lyrics", "session-7", 601_000),
            Err(CallbackError::Expired)
        );
        assert_eq!(begun, begun_before);
    }

    fn assert_unchanged(
        state: &CallbackState,
        plugin: &str,
        session: &str,
        now_ms: i64,
        expected: &Result<CallbackState, CallbackError>,
    ) {
        let before = state.clone();
        assert_eq!(
            &take_callback(state, plugin, session, now_ms),
            expected,
            "{plugin:?} {session:?} {now_ms}"
        );
        assert_eq!(state, &before);
    }

    /// Verifies: SEC-EXT-031
    #[test]
    fn a_callback_is_refused_for_the_earliest_fault() {
        let live = CallbackState {
            plugin: "plugin-a".to_owned(),
            session: "session-a".to_owned(),
            expires_at_ms: 5_000,
            used: false,
        };
        let used = CallbackState {
            used: true,
            ..live.clone()
        };
        assert_unchanged(
            &live,
            "plugin-b",
            "session-b",
            9_000,
            &Err(CallbackError::OtherPlugin),
        );
        assert_unchanged(
            &live,
            "plugin-b",
            "session-a",
            0,
            &Err(CallbackError::OtherPlugin),
        );
        assert_unchanged(
            &used,
            "plugin-b",
            "session-b",
            9_000,
            &Err(CallbackError::OtherPlugin),
        );
        assert_unchanged(
            &live,
            "Plugin-a",
            "session-a",
            0,
            &Err(CallbackError::OtherPlugin),
        );
        assert_unchanged(
            &live,
            "plugin-a",
            "session-b",
            9_000,
            &Err(CallbackError::OtherSession),
        );
        assert_unchanged(
            &used,
            "plugin-a",
            "session-b",
            0,
            &Err(CallbackError::OtherSession),
        );
        assert_unchanged(
            &live,
            "plugin-a",
            "session-a ",
            0,
            &Err(CallbackError::OtherSession),
        );
    }

    /// Verifies: SEC-EXT-031
    #[test]
    fn a_used_callback_is_replayed_and_an_old_one_is_expired() {
        let live = CallbackState {
            plugin: "plugin-a".to_owned(),
            session: "session-a".to_owned(),
            expires_at_ms: 5_000,
            used: false,
        };
        let used = CallbackState {
            used: true,
            ..live.clone()
        };
        assert_unchanged(
            &used,
            "plugin-a",
            "session-a",
            9_000,
            &Err(CallbackError::Replayed),
        );
        assert_unchanged(
            &used,
            "plugin-a",
            "session-a",
            4_999,
            &Err(CallbackError::Replayed),
        );
        assert_unchanged(
            &live,
            "plugin-a",
            "session-a",
            5_000,
            &Err(CallbackError::Expired),
        );
        assert_unchanged(
            &live,
            "plugin-a",
            "session-a",
            5_001,
            &Err(CallbackError::Expired),
        );
        assert_unchanged(
            &live,
            "plugin-a",
            "session-a",
            i64::MAX,
            &Err(CallbackError::Expired),
        );
    }

    /// Verifies: SEC-EXT-031
    #[test]
    fn a_missing_callback_is_missing() {
        assert_eq!(missing(), CallbackError::Missing);
        assert_eq!(
            [
                missing(),
                CallbackError::OtherSession,
                CallbackError::Replayed,
                CallbackError::Expired,
                CallbackError::OtherPlugin,
            ],
            [
                CallbackError::Missing,
                CallbackError::OtherSession,
                CallbackError::Replayed,
                CallbackError::Expired,
                CallbackError::OtherPlugin,
            ]
        );
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn a_private_session_is_refused_even_when_everything_else_passes() {
        assert_eq!(
            per_user_call(
                &[Scope::HistoryWrite, Scope::LibraryRead],
                Scope::HistoryWrite,
                true,
                true,
                true,
            ),
            Err(CallRefuse::Private)
        );
        assert_eq!(
            per_user_call(&[], Scope::HistoryWrite, false, true, false),
            Err(CallRefuse::Private)
        );
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn a_per_user_call_is_refused_in_check_order() {
        assert_eq!(
            per_user_call(&[], Scope::LibraryRead, false, false, false),
            Err(CallRefuse::NotOptedIn)
        );
        assert_eq!(
            per_user_call(
                &[Scope::LibraryRead],
                Scope::HistoryWrite,
                false,
                false,
                true,
            ),
            Err(CallRefuse::Scope)
        );
        assert_eq!(
            per_user_call(&[], Scope::EventsSelf, true, false, true),
            Err(CallRefuse::Scope)
        );
        assert_eq!(
            per_user_call(
                &[Scope::LibraryRead, Scope::HistoryWrite, Scope::EventsSelf],
                Scope::HistoryWrite,
                false,
                false,
                true,
            ),
            Err(CallRefuse::Scope)
        );
        assert_eq!(
            per_user_call(
                &[Scope::LibraryRead, Scope::HistoryWrite],
                Scope::HistoryWrite,
                true,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            per_user_call(
                &[Scope::RatingsWrite],
                Scope::RatingsWrite,
                true,
                false,
                true
            ),
            Ok(())
        );
    }

    fn per_user_oracle(
        granted: &[Scope],
        needed: Scope,
        user_has_scope: bool,
        private_session: bool,
        opted_in: bool,
    ) -> Result<(), CallRefuse> {
        if private_session {
            return Err(CallRefuse::Private);
        }
        if !opted_in {
            return Err(CallRefuse::NotOptedIn);
        }
        if !granted.contains(&needed) {
            return Err(CallRefuse::Scope);
        }
        if !user_has_scope {
            return Err(CallRefuse::Scope);
        }
        Ok(())
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn every_per_user_combination_follows_the_check_order() {
        let width = u32::try_from(Scope::ALL.len()).expect("scope count");
        let combinations = 1_usize.checked_shl(width).expect("scope subsets");
        for private_session in [false, true] {
            for opted_in in [false, true] {
                for user_has_scope in [false, true] {
                    for needed in Scope::ALL {
                        for mask in 0..combinations {
                            let mut granted = Vec::new();
                            let mut bits = mask;
                            for scope in Scope::ALL {
                                if bits % 2 == 1 {
                                    granted.push(scope);
                                }
                                bits /= 2;
                            }
                            assert_eq!(
                                per_user_call(
                                    &granted,
                                    needed,
                                    user_has_scope,
                                    private_session,
                                    opted_in,
                                ),
                                per_user_oracle(
                                    &granted,
                                    needed,
                                    user_has_scope,
                                    private_session,
                                    opted_in,
                                )
                            );
                        }
                    }
                }
            }
        }
    }

    fn file_name_oracle(path: &str) -> Result<String, ()> {
        let slash = path.rfind('/');
        let backslash = path.rfind('\\');
        let start = match (slash, backslash) {
            (Some(left), Some(right)) => left.max(right).saturating_add(1),
            (Some(index), None) | (None, Some(index)) => index.saturating_add(1),
            (None, None) => 0,
        };
        let name = &path[start..];
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
        {
            Err(())
        } else {
            Ok(name.to_owned())
        }
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn a_file_name_is_the_text_after_the_last_separator() {
        let cases = [
            ("a/b/c.flac", Ok("c.flac".to_owned())),
            ("c.flac", Ok("c.flac".to_owned())),
            ("Album/Track.flac", Ok("Track.flac".to_owned())),
            ("Album\\Track.flac", Ok("Track.flac".to_owned())),
            ("a\\b\\c.flac", Ok("c.flac".to_owned())),
            ("a/b\\c.flac", Ok("c.flac".to_owned())),
            ("a\\b/c.flac", Ok("c.flac".to_owned())),
            ("/c.flac", Ok("c.flac".to_owned())),
            ("\\c.flac", Ok("c.flac".to_owned())),
            ("./c.flac", Ok("c.flac".to_owned())),
            ("../c.flac", Ok("c.flac".to_owned())),
            ("Album/../Track.flac", Ok("Track.flac".to_owned())),
            (".hidden", Ok(".hidden".to_owned())),
            ("..flac", Ok("..flac".to_owned())),
            ("...", Ok("...".to_owned())),
            ("Album/Track Name.flac", Ok("Track Name.flac".to_owned())),
            ("Album/tràck.flac", Ok("tràck.flac".to_owned())),
            ("a//c.flac", Ok("c.flac".to_owned())),
            ("", Err(())),
            (".", Err(())),
            ("..", Err(())),
            ("Album/", Err(())),
            ("a/b/", Err(())),
            ("/", Err(())),
            ("\\", Err(())),
            ("Album\\", Err(())),
            ("a/.", Err(())),
            ("a/..", Err(())),
            ("a\\.", Err(())),
            ("a\\..", Err(())),
            ("./", Err(())),
            ("../", Err(())),
            ("a//", Err(())),
            ("a/b\\", Err(())),
        ];
        for (path, expected) in cases {
            assert_eq!(file_name(path), expected, "{path:?}");
            assert_eq!(file_name(path), file_name_oracle(path), "{path:?}");
        }
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn a_name_that_still_contains_a_separator_is_refused() {
        assert_eq!(super::checked_name("a/b"), Err(()));
        assert_eq!(super::checked_name("a\\b"), Err(()));
        assert_eq!(super::checked_name("a/b\\c"), Err(()));
        assert_eq!(super::checked_name("/"), Err(()));
        assert_eq!(super::checked_name("\\"), Err(()));
        assert_eq!(super::checked_name(""), Err(()));
        assert_eq!(super::checked_name("."), Err(()));
        assert_eq!(super::checked_name(".."), Err(()));
        assert_eq!(super::checked_name("c.flac"), Ok("c.flac".to_owned()));
        assert_eq!(
            super::checked_name("Track.flac"),
            Ok("Track.flac".to_owned())
        );
    }

    /// Verifies: SEC-EXT-029, SEC-EXT-030
    #[test]
    fn file_names_match_an_independent_split() {
        let paths = prop_oneof![
            Just("a/b/c.flac".to_owned()),
            Just("c.flac".to_owned()),
            Just("Album/Track.flac".to_owned()),
            Just(String::new()),
            Just(".".to_owned()),
            Just("..".to_owned()),
            Just("Album/".to_owned()),
            Just("a\\b\\c.flac".to_owned()),
            Just("a/b\\c.flac".to_owned()),
            "(?s).{0,24}",
        ];
        TestRunner::new(Config::default())
            .run(&paths, |path| {
                prop_assert_eq!(file_name(&path), file_name_oracle(&path));
                Ok(())
            })
            .expect("file name property");
    }

    fn plain_text_oracle(text: &str) -> Result<&str, ReturnRefuse> {
        if text.len() > 1_024 {
            Err(ReturnRefuse::TooLong {
                limit: 1_024,
                got: text.len(),
            })
        } else if text.contains('<') || text.contains('>') {
            Err(ReturnRefuse::Markup)
        } else {
            Ok(text)
        }
    }

    /// Verifies: SEC-EXT-028
    #[test]
    fn plain_text_refuses_markup_and_text_over_1024_bytes() {
        assert_eq!(MAX_TEXT, 1_024);
        let text = "biography";
        assert_eq!(plain_text(text), Ok(text));
        assert_eq!(
            plain_text(text).map(|kept| (kept.as_ptr(), kept.len())),
            Ok((text.as_ptr(), text.len()))
        );
        assert_eq!(plain_text(""), Ok(""));
        assert_eq!(
            plain_text("rock & roll \"quoted\""),
            Ok("rock & roll \"quoted\"")
        );
        assert_eq!(plain_text("a line\nstill plain"), Ok("a line\nstill plain"));
        assert_eq!(plain_text("<script>"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text("<img onerror>"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text("a<b"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text("a>b"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text("<"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text(">"), Err(ReturnRefuse::Markup));
        assert_eq!(plain_text("<>"), Err(ReturnRefuse::Markup));

        let exact = "x".repeat(1_024);
        assert_eq!(plain_text(&exact), Ok(exact.as_str()));
        let under = "x".repeat(1_023);
        assert_eq!(plain_text(&under), Ok(under.as_str()));
        let over = "x".repeat(1_025);
        assert_eq!(
            plain_text(&over),
            Err(ReturnRefuse::TooLong {
                limit: 1_024,
                got: 1_025,
            })
        );

        let exact_bytes = "é".repeat(512);
        assert_eq!(exact_bytes.len(), 1_024);
        assert_eq!(exact_bytes.chars().count(), 512);
        assert_eq!(plain_text(&exact_bytes), Ok(exact_bytes.as_str()));
        let over_bytes = "é".repeat(513);
        assert_eq!(over_bytes.len(), 1_026);
        assert_eq!(over_bytes.chars().count(), 513);
        assert_eq!(
            plain_text(&over_bytes),
            Err(ReturnRefuse::TooLong {
                limit: 1_024,
                got: 1_026,
            })
        );

        let mut less_at_end = "a".repeat(1_023);
        less_at_end.push('<');
        assert_eq!(less_at_end.len(), 1_024);
        assert_eq!(plain_text(&less_at_end), Err(ReturnRefuse::Markup));
        let mut greater_at_end = "a".repeat(1_023);
        greater_at_end.push('>');
        assert_eq!(plain_text(&greater_at_end), Err(ReturnRefuse::Markup));
        let mut less_at_start = String::from("<");
        less_at_start.push_str(&"a".repeat(1_023));
        assert_eq!(plain_text(&less_at_start), Err(ReturnRefuse::Markup));

        let both = format!("<{}>", "x".repeat(1_024));
        assert_eq!(both.len(), 1_026);
        assert!(both.contains('<'));
        assert_eq!(
            plain_text(&both),
            Err(ReturnRefuse::TooLong {
                limit: 1_024,
                got: 1_026,
            })
        );
    }

    /// Verifies: SEC-EXT-028
    #[test]
    fn plain_text_matches_the_byte_and_markup_rules() {
        let texts = prop_oneof![
            Just(String::new()),
            Just("a".repeat(1_023)),
            Just("a".repeat(1_024)),
            Just("a".repeat(1_025)),
            Just("é".repeat(512)),
            Just("é".repeat(513)),
            Just("<script>".to_owned()),
            Just("<img onerror>".to_owned()),
            Just("a<b".to_owned()),
            Just("a>b".to_owned()),
            Just(format!("<{}>", "x".repeat(1_024))),
            ".{0,80}",
            "<.{0,40}",
            ".{0,40}>",
        ];
        TestRunner::new(Config::default())
            .run(&texts, |text| {
                prop_assert_eq!(plain_text(&text), plain_text_oracle(&text));
                Ok(())
            })
            .expect("plain text property");
    }

    fn plain_list_oracle<'a>(items: &[&'a str]) -> Result<Vec<&'a str>, ReturnRefuse> {
        if items.len() > 20 {
            return Err(ReturnRefuse::TooMany {
                limit: 20,
                got: items.len(),
            });
        }
        let mut kept = Vec::new();
        for item in items {
            kept.push(plain_text_oracle(item)?);
        }
        Ok(kept)
    }

    /// Verifies: SEC-EXT-028
    #[test]
    fn a_plain_list_keeps_the_same_slices_up_to_twenty() {
        assert_eq!(MAX_ITEMS, 20);
        assert_eq!(plain_list(&[]), Ok(vec![]));
        assert_eq!(plain_list(&[""]), Ok(vec![""]));
        let items = ["alpha", "beta", "gamma"];
        assert_eq!(plain_list(&items), Ok(vec!["alpha", "beta", "gamma"]));
        let kept = plain_list(&items).expect("three plain slices");
        assert_eq!(
            kept.iter()
                .map(|item| (item.as_ptr(), item.len()))
                .collect::<Vec<_>>(),
            items
                .iter()
                .map(|item| (item.as_ptr(), item.len()))
                .collect::<Vec<_>>()
        );

        let twenty = [
            "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14",
            "15", "16", "17", "18", "19", "20",
        ];
        assert_eq!(plain_list(&twenty), Ok(twenty.to_vec()));
        let twenty_one = [
            "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14",
            "15", "16", "17", "18", "19", "20", "21",
        ];
        assert_eq!(
            plain_list(&twenty_one),
            Err(ReturnRefuse::TooMany { limit: 20, got: 21 })
        );
        assert_eq!(
            plain_list(&["ok", "<script>", "later"]),
            Err(ReturnRefuse::Markup)
        );
        assert_eq!(plain_list(&["<script>", "ok"]), Err(ReturnRefuse::Markup));
        let long = "x".repeat(1_025);
        assert_eq!(
            plain_list(&["ok", long.as_str()]),
            Err(ReturnRefuse::TooLong {
                limit: 1_024,
                got: 1_025,
            })
        );
        assert_eq!(
            plain_list(&["<bad>", long.as_str()]),
            Err(ReturnRefuse::Markup)
        );
        let mut too_many = vec!["<script>"];
        too_many.resize(21, "track");
        assert_eq!(
            plain_list(&too_many),
            Err(ReturnRefuse::TooMany { limit: 20, got: 21 })
        );
        let exact = "y".repeat(1_024);
        assert_eq!(plain_list(&[exact.as_str()]), Ok(vec![exact.as_str()]));
    }

    /// Verifies: SEC-EXT-028
    #[test]
    fn plain_lists_match_an_independent_limit() {
        let lists = proptest::collection::vec(
            prop_oneof![
                Just(String::new()),
                Just("track".to_owned()),
                Just("<script>".to_owned()),
                Just("a>b".to_owned()),
                Just("x".repeat(1_025)),
                ".{0,12}",
            ],
            0..24,
        );
        TestRunner::new(Config::default())
            .run(&lists, |items| {
                let refs: Vec<&str> = items.iter().map(String::as_str).collect();
                prop_assert_eq!(plain_list(&refs), plain_list_oracle(&refs));
                Ok(())
            })
            .expect("plain list property");
    }
}
