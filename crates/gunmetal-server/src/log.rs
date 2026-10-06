//! The structured logger: one JSON object per line, every value escaped and
//! length-capped, every event a variant of one typed enum.
//!
//! - **One line per event.** A line is `{"ts":…,"level":…,"event":…}` and
//!   the event's fields, in a fixed order. Every string, the keys included,
//!   goes through one escaper, which writes `"` and `\` with a backslash and
//!   every control character, U+2028, U+2029 and the bidirectional controls
//!   as `\uXXXX`. So no value can end a line, start a new record, move a
//!   terminal's cursor or reorder what a reader sees (SEC-OPS-022,
//!   SEC-IAM-096, SEC-MED-062, SEC-TM-057). Nothing in a value is ever
//!   interpreted: there are no lookups or format directives.
//! - **Capped.** A text value longer than [`MAX_FIELD_BYTES`] is cut at the
//!   last character boundary within the cap and ends with `…`.
//! - **Typed events.** Every line comes from a [`LogEvent`], whose variants
//!   are listed with their fields, data classes, destination and retention
//!   in `log-inventory.toml` beside this crate's manifest; a test fails when
//!   the two differ (SEC-PRV-044). No field type can hold a request body, a
//!   cookie, an authorisation header or a secret.
//! - **Classes decide visibility.** Each field carries the data class of the
//!   privacy baseline. At the default level a field of the library or
//!   activity class (titles, file paths, search terms, plays) is written as
//!   `[withheld]`, and a field of the secret class always is (SEC-PRV-043,
//!   SEC-OPS-029, SEC-PRV-042).
//! - **URLs are redacted.** A URL is logged only as a [`LoggedUrl`], which
//!   drops the user information, the query string and the fragment and
//!   replaces every non-empty path segment, so stream signatures,
//!   invitation, share and pairing codes, OAuth codes and states, titles
//!   and search terms never reach a line (SEC-IAM-047, SEC-PRV-043).
//! - **Debug switches itself off.** The level defaults to info. Debug level
//!   is switched on for a stated time of at most 24 hours, only once the
//!   audit log has the [`SecurityEvent::GmDebugLoggingEnabled`] record, and
//!   the logger returns to its configured level by itself when the time is
//!   up (SEC-OPS-029).

use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::Write;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
use gunmetal_core::id::PublicId;
use gunmetal_core::schema::DataClass;
use gunmetal_core::time::{Clock, Timestamp, format_rfc3339};
use gunmetal_fs::dataroot::Repair;

use crate::datadir::item_name;

/// The longest text value, in bytes, a line holds before it is cut.
pub const MAX_FIELD_BYTES: usize = 1024;

/// The longest debug level stays on, in milliseconds: 24 hours.
pub const MAX_DEBUG_MS: i64 = 86_400_000;

/// What a field that may not be shown is written as.
const WITHHELD: &str = "[withheld]";

/// What a redacted part of a URL is written as.
const REDACTED: &str = "[redacted]";

/// How important an event is. Events below the logger's level are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Detail for finding a fault, which may hold identifiers and paths.
    Debug,
    /// Normal operation. The default.
    Info,
    /// Something an admin should look at.
    Warn,
    /// Something failed.
    Error,
}

impl Level {
    /// The level's name in a log line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// A URL with everything that could carry a secret or a title taken out.
///
/// Built only by [`LoggedUrl::redact`]: a recognised scheme and its host
/// stay, the user information, query and fragment go, and every non-empty
/// path segment is replaced. Until WP-118 can keep a route template's
/// literal segments, the logger fails closed (SEC-IAM-047, SEC-PRV-043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedUrl(String);

impl LoggedUrl {
    /// Redacts `url`.
    #[must_use]
    pub fn redact(url: &str) -> Self {
        let (url, _fragment) = url.split_once('#').unwrap_or((url, ""));
        let (url, query) = match url.split_once('?') {
            Some((url, _)) => (url, "?[redacted]"),
            None => (url, ""),
        };
        let (origin, path) = match url.split_once("://") {
            Some((scheme, rest)) if is_scheme(scheme) => {
                let (authority, path) = rest.find('/').map_or((rest, ""), |at| rest.split_at(at));
                let host = authority
                    .rsplit_once('@')
                    .map_or(authority, |(_, host)| host);
                (format!("{scheme}://{host}"), path)
            }
            _ => (String::new(), url),
        };
        let path: Vec<&str> = path
            .split('/')
            .map(|segment| {
                if segment.is_empty() {
                    segment
                } else {
                    REDACTED
                }
            })
            .collect();
        Self(format!("{origin}{}{query}", path.join("/")))
    }

    /// The redacted URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `text` is a URI scheme: `[a-z][a-z0-9+.-]*`. Anything else
/// before `://` is treated as a path, so a secret cannot hide in a fake
/// scheme.
fn is_scheme(text: &str) -> bool {
    let mut bytes = text.bytes();
    match bytes.next() {
        Some(b'a'..=b'z') => {
            bytes.all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'+' | b'.' | b'-'))
        }
        _ => false,
    }
}

/// The value of one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Text, escaped and capped.
    Text(String),
    /// A whole number.
    Number(u64),
    /// A moment, written in RFC 3339.
    Time(Timestamp),
    /// A redacted URL.
    Url(LoggedUrl),
}

/// One field of a log line: its key, its data class and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The key.
    pub key: &'static str,
    /// The data class, which decides when the value may be shown.
    pub class: DataClass,
    /// The value.
    pub value: Value,
}

/// Every event the server logs. Each variant is listed in
/// `log-inventory.toml` (SEC-PRV-044).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    /// The server started.
    SysStartup {
        /// The server's version.
        version: &'static str,
    },
    /// Opening the data directory tightened a mode (SEC-OPS-012).
    DataRootModeRepaired {
        /// What was tightened, named relative to the data directory.
        item: String,
        /// The permission bits before.
        from: u32,
        /// The permission bits after.
        to: u32,
    },
    /// Opening the data directory removed a temporary file that an
    /// interrupted write left in `secrets/` (SEC-OPS-012).
    DataRootLeftoverRemoved {
        /// The file, named relative to the data directory.
        item: String,
    },
    /// Debug level was switched on until the time given (SEC-OPS-029).
    DebugLoggingEnabled {
        /// When it switches itself off.
        until: Timestamp,
    },
    /// Debug level switched itself off (SEC-OPS-029).
    DebugLoggingEnded,
    /// The credential verifier refused a sign-in attempt (SEC-OPS-028,
    /// SEC-IAM-099). This is the line a fail2ban filter matches. Its name,
    /// the order of its fields and `addr` coming straight after the name
    /// are format version 1, which the line states as `v`; changing any of
    /// them is a new version. It is logged as an error so that no
    /// configured level drops it.
    AuthnLoginFail {
        /// The client address the listener resolved.
        addr: IpAddr,
        /// The name of the sign-in pathway.
        pathway: &'static str,
        /// Why the attempt was refused: a word fixed in the program.
        cause: &'static str,
    },
}

impl LogEvent {
    /// The event's name in a log line.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::SysStartup { .. } => "sys_startup",
            Self::DataRootModeRepaired { .. } => "gm_data_root_mode_repaired",
            Self::DataRootLeftoverRemoved { .. } => "gm_data_root_leftover_removed",
            Self::DebugLoggingEnabled { .. } => "gm_debug_logging_enabled",
            Self::DebugLoggingEnded => "gm_debug_logging_ended",
            Self::AuthnLoginFail { .. } => "authn_login_fail",
        }
    }

    /// The event's level.
    #[must_use]
    pub const fn level(&self) -> Level {
        match self {
            Self::SysStartup { .. } => Level::Info,
            Self::DataRootModeRepaired { .. }
            | Self::DataRootLeftoverRemoved { .. }
            | Self::DebugLoggingEnabled { .. }
            | Self::DebugLoggingEnded => Level::Warn,
            Self::AuthnLoginFail { .. } => Level::Error,
        }
    }

    /// The event's fields, in the order a line writes them.
    #[must_use]
    pub fn fields(&self) -> Vec<Field> {
        let public = |key, value| Field {
            key,
            class: DataClass::Public,
            value,
        };
        match self {
            Self::SysStartup { version } => {
                vec![public("version", Value::Text((*version).to_owned()))]
            }
            Self::DataRootModeRepaired { item, from, to } => vec![
                public("item", Value::Text(item.clone())),
                public("from", Value::Number(u64::from(*from))),
                public("to", Value::Number(u64::from(*to))),
            ],
            Self::DataRootLeftoverRemoved { item } => {
                vec![public("item", Value::Text(item.clone()))]
            }
            Self::DebugLoggingEnabled { until } => vec![public("until", Value::Time(*until))],
            Self::DebugLoggingEnded => Vec::new(),
            Self::AuthnLoginFail {
                addr,
                pathway,
                cause,
            } => vec![
                Field {
                    key: "addr",
                    class: DataClass::Identity,
                    value: Value::Text(addr.to_string()),
                },
                public("v", Value::Number(1)),
                public("pathway", Value::Text((*pathway).to_owned())),
                public("cause", Value::Text((*cause).to_owned())),
            ],
        }
    }
}

impl From<&Repair> for LogEvent {
    fn from(repair: &Repair) -> Self {
        match repair {
            Repair::Mode { item, from, to } => Self::DataRootModeRepaired {
                item: item_name(item),
                from: *from,
                to: *to,
            },
            Repair::Removed { item } => Self::DataRootLeftoverRemoved {
                item: item_name(item),
            },
        }
    }
}

/// Cuts `text` to at most [`MAX_FIELD_BYTES`] bytes at a character
/// boundary, marking a cut with `…`.
fn capped(text: &str) -> Cow<'_, str> {
    if text.len() <= MAX_FIELD_BYTES {
        return Cow::Borrowed(text);
    }
    let mut end = 0;
    for (at, _) in text.char_indices() {
        if at > MAX_FIELD_BYTES {
            break;
        }
        end = at;
    }
    Cow::Owned(format!("{}…", &text[..end]))
}

/// Whether a character is written as `\uXXXX`: the control characters, the
/// line and paragraph separators, and the bidirectional controls.
fn escaped(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Appends `text` to `line` as a JSON string.
fn push_string(line: &mut String, text: &str) {
    line.push('"');
    for c in text.chars() {
        match c {
            '"' | '\\' => {
                line.push('\\');
                line.push(c);
            }
            c if escaped(c) => {
                let _ = write!(line, "\\u{:04x}", u32::from(c));
            }
            c => line.push(c),
        }
    }
    line.push('"');
}

/// Whether a field of `class` may be shown when the logger is at `level`.
fn visible(class: DataClass, level: Level) -> bool {
    match class {
        DataClass::Public | DataClass::Identity => true,
        DataClass::Library | DataClass::Activity => level == Level::Debug,
        DataClass::Secret => false,
    }
}

/// Writes one line, ending in a newline, for `event` at `time`, with the
/// logger at `level`.
fn encode(
    time: Timestamp,
    level: Level,
    name: &str,
    event_level: Level,
    fields: &[Field],
) -> String {
    let mut line = String::from("{");
    let pairs = [
        ("ts", format_rfc3339(time)),
        ("level", event_level.name().to_owned()),
        ("event", name.to_owned()),
    ];
    for (key, value) in pairs {
        push_key(&mut line, key);
        push_string(&mut line, &value);
    }
    for field in fields {
        push_key(&mut line, field.key);
        if !visible(field.class, level) {
            push_string(&mut line, WITHHELD);
            continue;
        }
        match &field.value {
            Value::Text(text) => push_string(&mut line, &capped(text)),
            Value::Number(number) => {
                let _ = write!(line, "{number}");
            }
            Value::Time(time) => push_string(&mut line, &format_rfc3339(*time)),
            Value::Url(url) => push_string(&mut line, url.as_str()),
        }
    }
    line.push_str("}\n");
    line
}

/// Appends `,` unless `line` is just the opening brace, then `key` and `:`.
fn push_key(line: &mut String, key: &str) {
    if line.len() > 1 {
        line.push(',');
    }
    push_string(line, key);
    line.push(':');
}

/// What the logger changes as it runs.
struct State {
    out: Box<dyn Write + Send>,
    base: Level,
    /// When debug is on: when the window opened and when it must end.
    debug_window: Option<(Timestamp, Timestamp)>,
}

/// The value inside the lock, even if a previous holder panicked. Writing
/// a line cannot leave the logger half-updated, so the next event can
/// still be written.
fn recover<T>(result: Result<T, PoisonError<T>>) -> T {
    result.unwrap_or_else(PoisonError::into_inner)
}

/// The server's one logger.
pub struct Logger {
    clock: Arc<dyn Clock + Send + Sync>,
    state: Mutex<State>,
}

impl Logger {
    /// A logger at `base` level, reading the time from `clock` and writing
    /// lines to `out`. `base` is the configured level, which is never
    /// debug: debug level is only ever switched on for a time.
    #[must_use]
    pub fn new(
        clock: Arc<dyn Clock + Send + Sync>,
        base: Level,
        out: Box<dyn Write + Send>,
    ) -> Self {
        Self {
            clock,
            state: Mutex::new(State {
                out,
                base,
                debug_window: None,
            }),
        }
    }

    /// Sets the configured level, once the configuration has been read. A
    /// debug window that is open stays open until its time is up.
    pub fn set_base(&self, base: Level) {
        recover(self.state.lock()).base = base;
    }

    /// The level the logger is at now. Reading it when debug level's time
    /// is up switches debug level off and logs that it did.
    pub fn level(&self) -> Level {
        let now = self.clock.now();
        let mut state = recover(self.state.lock());
        Self::settle(&mut state, now)
    }

    /// Logs `event` if its level is at or above the logger's.
    pub fn log(&self, event: &LogEvent) {
        let now = self.clock.now();
        let mut state = recover(self.state.lock());
        let level = Self::settle(&mut state, now);
        Self::write(&mut state, now, level, event);
    }

    /// Switches debug level on for `duration`, at most 24 hours, and
    /// returns when it switches itself off. The audit log records the
    /// [`SecurityEvent::GmDebugLoggingEnabled`] event first, naming the
    /// `account` that asked, or none for the host; if it cannot, debug
    /// level stays off.
    ///
    /// # Errors
    ///
    /// [`AuditUnavailable`] when `sink` could not record the event.
    pub fn enable_debug(
        &self,
        duration: Duration,
        account: Option<PublicId>,
        sink: &dyn SecuritySink,
    ) -> Result<Timestamp, AuditUnavailable> {
        sink.record(SecurityEvent::GmDebugLoggingEnabled { account })?;
        let now = self.clock.now();
        let ms = i64::try_from(duration.as_millis())
            .unwrap_or(MAX_DEBUG_MS)
            .min(MAX_DEBUG_MS);
        let until = Timestamp::from_millis(now.millis() + ms).unwrap_or(Timestamp::MAX);
        let mut state = recover(self.state.lock());
        state.debug_window = Some((now, until));
        Self::write(
            &mut state,
            now,
            Level::Debug,
            &LogEvent::DebugLoggingEnabled { until },
        );
        Ok(until)
    }

    /// The level at `now`, switching debug level off, and logging that, when
    /// its time is up or the clock has stepped back before the window opened.
    fn settle(state: &mut State, now: Timestamp) -> Level {
        match state.debug_window {
            Some((opened, until)) if now >= opened && now < until => Level::Debug,
            Some(_) => {
                state.debug_window = None;
                let base = state.base;
                Self::write(state, now, base, &LogEvent::DebugLoggingEnded);
                base
            }
            None => state.base,
        }
    }

    /// Writes `event` at `now` with the logger at `level`, if the event's
    /// level is high enough. A line that cannot be written is lost: logging
    /// never stops the server.
    fn write(state: &mut State, now: Timestamp, level: Level, event: &LogEvent) {
        if event.level() >= level {
            let line = encode(now, level, event.name(), event.level(), &event.fields());
            let _ = state.out.write_all(line.as_bytes());
        }
    }
}

/// Compile-fail tests: a secret cannot be written into a log field. Rustdoc
/// on stable does not check which error a compile-fail test produced, so
/// the test shares everything but its last line with the control, which
/// compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: a field's text is a `String`, and all a secret offers to
    /// make one from is its debug form, the placeholder.
    ///
    /// ```
    /// use gunmetal_secrets::Secret;
    /// use gunmetal_server::log::Value;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let field = Value::Text(format!("{secret:?}"));
    /// assert_eq!(field, Value::Text("Secret([redacted])".to_owned()));
    /// ```
    struct Control;

    /// Verifies: SEC-PRV-042, SEC-TM-057
    ///
    /// A secret has no `Display`, so its value cannot be formatted into a
    /// field's text.
    ///
    /// ```compile_fail
    /// use gunmetal_secrets::Secret;
    /// use gunmetal_server::log::Value;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let field = Value::Text(format!("{secret:?}"));
    /// let field = Value::Text(format!("{secret}"));
    /// ```
    struct NoSecretText;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{self, Capture, NOON, Recording};
    use gunmetal_core::id::IdKind;
    use gunmetal_fs::dataroot::Item;
    use gunmetal_fs::path::{DataDir, DataPath};
    use gunmetal_testkit::clock::ManualClock;
    use proptest::prelude::*;

    fn logger(base: Level) -> (Logger, Arc<ManualClock>, Capture) {
        let (clock, manual) = testing::clock();
        let capture = Capture::default();
        let logger = Logger::new(clock, base, Box::new(capture.clone()));
        (logger, manual, capture)
    }

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    fn text(key: &'static str, class: DataClass, value: &str) -> Field {
        Field {
            key,
            class,
            value: Value::Text(value.to_owned()),
        }
    }

    fn line(level: Level, fields: &[Field]) -> String {
        encode(at(NOON), level, "gm_test", Level::Info, fields)
    }

    const HEAD: &str = r#"{"ts":"2026-10-03T12:00:00.000Z","level":"info","event":"gm_test""#;

    // ----- An independent reader of the lines, written only for the tests.

    /// The escapes RFC 8259 writes with one letter, and what they stand for.
    const SHORT_ESCAPES: [(char, char); 8] = [
        ('"', '"'),
        ('\\', '\\'),
        ('/', '/'),
        ('n', '\n'),
        ('r', '\r'),
        ('t', '\t'),
        ('b', '\u{8}'),
        ('f', '\u{c}'),
    ];

    /// Reads one escape after its backslash.
    fn read_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> char {
        let letter = chars.next().expect("an escape letter");
        if letter == 'u' {
            let hex: String = chars.by_ref().take(4).collect();
            let code = u32::from_str_radix(&hex, 16).expect("four hex digits");
            return char::from_u32(code).expect("not a surrogate");
        }
        SHORT_ESCAPES
            .iter()
            .find(|(written, _)| *written == letter)
            .map(|(_, meant)| *meant)
            .expect("a known escape")
    }

    /// Reads one JSON string starting at the quote `chars` is on, the way
    /// RFC 8259 says, panicking on anything else.
    fn read_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
        assert_eq!(chars.next(), Some('"'));
        let mut out = String::new();
        loop {
            match chars.next().expect("a terminated string") {
                '"' => return out,
                '\\' => out.push(read_escape(chars)),
                c => {
                    assert!(u32::from(c) >= 0x20);
                    out.push(c);
                }
            }
        }
    }

    /// Reads a whole number.
    fn read_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
        let mut digits = String::new();
        while let Some(c) = chars.next_if(char::is_ascii_digit) {
            digits.push(c);
        }
        assert!(!digits.is_empty());
        digits
    }

    /// Reads a line as a flat JSON object of strings and whole numbers.
    fn read_line(line: &str) -> Vec<(String, String)> {
        let body = line.strip_suffix('\n').expect("a line ends with a newline");
        assert!(!body.contains('\n'));
        let mut chars = body.chars().peekable();
        assert_eq!(chars.next(), Some('{'));
        let mut pairs = Vec::new();
        loop {
            let key = read_string(&mut chars);
            assert_eq!(chars.next(), Some(':'));
            let value = if chars.peek() == Some(&'"') {
                read_string(&mut chars)
            } else {
                read_number(&mut chars)
            };
            pairs.push((key, value));
            let separator = chars.next();
            if separator == Some('}') {
                break;
            }
            assert_eq!(separator, Some(','));
        }
        assert_eq!(chars.next(), None);
        pairs
    }

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    // ----- Levels.

    #[test]
    fn names_each_level() {
        let names: Vec<&str> = [Level::Debug, Level::Info, Level::Warn, Level::Error]
            .iter()
            .map(|level| level.name())
            .collect();
        assert_eq!(names, ["debug", "info", "warn", "error"]);
    }

    // ----- The encoder.

    /// Verifies: SEC-OPS-022, SEC-IAM-096, SEC-MED-062, SEC-TM-057
    #[test]
    fn escapes_line_breaks_quotes_and_terminal_controls() {
        let hostile =
            "a\r\nb\"c\\d\u{0}e\u{1b}[31mf\u{2028}g\u{2029}h\u{7f}i\u{85}j\u{202e}k\u{2066}l\tm";
        assert_eq!(
            line(Level::Info, &[text("name", DataClass::Public, hostile)]),
            format!(
                "{HEAD},{}}}\n",
                r#""name":"a\u000d\u000ab\"c\\d\u0000e\u001b[31mf\u2028g\u2029h\u007fi\u0085j\u202ek\u2066l\u0009m""#
            )
        );
    }

    #[test]
    fn leaves_the_characters_beside_the_escaped_ranges_alone() {
        let edges =
            "\u{1f}\u{20}\u{7e}\u{9f}\u{a0}\u{2027}\u{202a}\u{202f}\u{2065}\u{2069}\u{206a}";
        assert_eq!(
            line(Level::Info, &[text("k", DataClass::Public, edges)]),
            format!(
                "{HEAD},{}}}\n",
                "\"k\":\"\\u001f ~\\u009f\u{a0}\u{2027}\\u202a\u{202f}\u{2065}\\u2069\u{206a}\""
            )
        );
    }

    /// Verifies: SEC-MED-062
    #[test]
    fn caps_long_values_at_a_character_boundary() {
        let exact = "a".repeat(MAX_FIELD_BYTES);
        assert_eq!(capped(&exact), Cow::Borrowed(exact.as_str()));
        let over = "a".repeat(MAX_FIELD_BYTES + 1);
        assert_eq!(capped(&over), Cow::<str>::Owned(format!("{exact}…")));
        // A two-byte character straddles the cap, so the cut falls before it.
        let straddling = format!("{}é", "a".repeat(MAX_FIELD_BYTES - 1));
        assert_eq!(
            capped(&format!("{straddling}z")),
            Cow::<str>::Owned(format!("{}…", "a".repeat(MAX_FIELD_BYTES - 1)))
        );
    }

    #[test]
    fn the_test_reader_reads_a_number_field() {
        let written = line(
            Level::Info,
            &[Field {
                key: "n",
                class: DataClass::Public,
                value: Value::Number(7),
            }],
        );
        assert_eq!(
            read_line(&written),
            pairs(&[
                ("ts", "2026-10-03T12:00:00.000Z"),
                ("level", "info"),
                ("event", "gm_test"),
                ("n", "7"),
            ])
        );
    }

    #[test]
    fn writes_numbers_times_and_urls() {
        let fields = [
            Field {
                key: "n",
                class: DataClass::Public,
                value: Value::Number(u64::MAX),
            },
            Field {
                key: "t",
                class: DataClass::Public,
                value: Value::Time(at(0)),
            },
            Field {
                key: "u",
                class: DataClass::Public,
                value: Value::Url(LoggedUrl::redact("https://example.org/api/v1?x=1")),
            },
        ];
        assert_eq!(
            line(Level::Info, &fields),
            format!(
                "{HEAD},{}}}\n",
                r#""n":18446744073709551615,"t":"1970-01-01T00:00:00.000Z","u":"https://example.org/[redacted]/[redacted]?[redacted]""#
            )
        );
    }

    /// Verifies: SEC-PRV-043, SEC-PRV-042
    #[test]
    fn withholds_library_activity_and_secret_fields_at_the_default_level() {
        let fields = [
            text("server", DataClass::Public, "home"),
            text("user", DataClass::Identity, "ada"),
            text("title", DataClass::Library, "A Love Supreme"),
            text("played", DataClass::Activity, "track"),
            text("key", DataClass::Secret, "hunter2"),
        ];
        let expected = r#""server":"home","user":"ada","title":"[withheld]","played":"[withheld]","key":"[withheld]""#;
        for level in [Level::Info, Level::Warn, Level::Error] {
            assert_eq!(line(level, &fields), format!("{HEAD},{expected}}}\n"));
        }
        let debug = r#""server":"home","user":"ada","title":"A Love Supreme","played":"track","key":"[withheld]""#;
        assert_eq!(line(Level::Debug, &fields), format!("{HEAD},{debug}}}\n"));
    }

    fn hostile_char() -> impl Strategy<Value = char> {
        prop_oneof![
            any::<char>(),
            prop::sample::select(vec![
                '\r', '\n', '\0', '\u{1b}', '\u{2028}', '\u{2029}', '"', '\\', '\u{7f}', '\u{85}',
                '\u{202e}', '{', '}', ':', ','
            ]),
        ]
    }

    proptest! {
        /// Verifies: SEC-OPS-022, SEC-IAM-096, SEC-TM-057
        #[test]
        fn any_text_makes_exactly_one_line_that_reads_back(
            values in prop::collection::vec(prop::collection::vec(hostile_char(), 0..64), 1..4)
        ) {
            let values: Vec<String> = values.into_iter().map(String::from_iter).collect();
            let keys = ["a", "b", "c"];
            let fields: Vec<Field> = keys
                .iter()
                .zip(&values)
                .map(|(key, value)| text(key, DataClass::Public, value))
                .collect();
            let written = line(Level::Info, &fields);
            prop_assert_eq!(written.matches('\n').count(), 1);
            let raw = ['\r', '\u{2028}', '\u{2029}', '\u{1b}'];
            prop_assert!(!written.contains(raw));
            let mut expected = pairs(&[
                ("ts", "2026-10-03T12:00:00.000Z"),
                ("level", "info"),
                ("event", "gm_test"),
            ]);
            expected.extend(keys.iter().zip(&values).map(|(key, value)| ((*key).to_owned(), value.clone())));
            prop_assert_eq!(read_line(&written), expected);
        }
    }

    // ----- URLs.

    /// Verifies: SEC-IAM-047, SEC-PRV-042, SEC-TM-057
    #[test]
    fn redacts_tokens_codes_and_query_strings_from_urls() {
        const CANARY: &str = "canary7f3a";
        let cases = [
            (
                "https://music.example.org/s/AbCdEfGhIjKlMnOpQrStUv/stream?sig=1",
                "https://music.example.org/[redacted]/[redacted]/[redacted]?[redacted]",
            ),
            (
                "https://user:pass@example.org:8443/invite/4fq7k2m9x3c8v5b1n6z0p2r4t7",
                "https://example.org:8443/[redacted]/[redacted]",
            ),
            (
                "https://example.org/oauth/callback?code=abc&state=xyz#frag",
                "https://example.org/[redacted]/[redacted]?[redacted]",
            ),
            ("/pair/ABCD-EFGH", "/[redacted]/[redacted]"),
            ("/music/A%20Love%20Supreme.flac", "/[redacted]/[redacted]"),
            ("https://example.org", "https://example.org"),
            ("https://example.org/", "https://example.org/"),
            ("/a-b_c.d/0123456789abcdef", "/[redacted]/[redacted]"),
            ("/0123456789abcdefg", "/[redacted]"),
            ("relative#only", "[redacted]"),
            ("/library/canary7f3a", "/[redacted]/[redacted]"),
            ("/pair/bcdfghjk", "/[redacted]/[redacted]"),
            (
                "/s/0123456789abcdef/stream",
                "/[redacted]/[redacted]/[redacted]",
            ),
            (
                "/invite/4fq7k2m9x3c8v5b1n6z0p2r4t7/next://x",
                "/[redacted]/[redacted]/[redacted]//[redacted]",
            ),
            (
                "/invite/canary7f3a/next://x",
                "/[redacted]/[redacted]/[redacted]//[redacted]",
            ),
            ("://x", "[redacted]//[redacted]"),
            (
                "hTTP://example.org/library/canary7f3a",
                "[redacted]//[redacted]/[redacted]/[redacted]",
            ),
        ];
        for (url, expected) in cases {
            assert_eq!(LoggedUrl::redact(url).as_str(), expected);
            assert!(!LoggedUrl::redact(url).as_str().contains(CANARY));
            assert!(!expected.contains(CANARY));
        }
    }

    /// Verifies: SEC-OPS-029, SEC-PRV-043, SEC-IAM-047
    #[test]
    fn a_canary_in_a_query_string_or_a_title_never_reaches_a_default_level_line() {
        const CANARY: &str = "canary7f3a";
        let fields = [
            Field {
                key: "url",
                class: DataClass::Public,
                value: Value::Url(LoggedUrl::redact(&format!(
                    "https://example.org/search?q={CANARY}&cookie={CANARY}#{CANARY}"
                ))),
            },
            Field {
                key: "path",
                class: DataClass::Public,
                value: Value::Url(LoggedUrl::redact(&format!("/library/{CANARY}"))),
            },
            text("title", DataClass::Library, CANARY),
            text("search", DataClass::Activity, CANARY),
            text("authorization", DataClass::Secret, CANARY),
        ];
        let written = line(Level::Info, &fields);
        assert!(!written.contains(CANARY));
        assert_eq!(
            written,
            format!(
                "{HEAD},{}}}\n",
                r#""url":"https://example.org/[redacted]?[redacted]","path":"/[redacted]/[redacted]","title":"[withheld]","search":"[withheld]","authorization":"[withheld]""#
            )
        );
    }

    // ----- Events.

    fn one_of_each() -> Vec<LogEvent> {
        vec![
            LogEvent::SysStartup { version: "1.2.3" },
            LogEvent::DataRootModeRepaired {
                item: "secrets/root.key".to_owned(),
                from: 0o644,
                to: 0o600,
            },
            LogEvent::DataRootLeftoverRemoved {
                item: "secrets/keys.json".to_owned(),
            },
            LogEvent::DebugLoggingEnabled { until: at(NOON) },
            LogEvent::DebugLoggingEnded,
            LogEvent::AuthnLoginFail {
                addr: IpAddr::from([203, 0, 113, 7]),
                pathway: "claim_code",
                cause: "credential",
            },
        ]
    }

    #[test]
    fn writes_every_event_as_its_own_line() {
        let lines: Vec<String> = one_of_each()
            .iter()
            .map(|event| {
                encode(
                    at(NOON),
                    Level::Info,
                    event.name(),
                    event.level(),
                    &event.fields(),
                )
            })
            .collect();
        let ts = r#"{"ts":"2026-10-03T12:00:00.000Z","level":"#;
        assert_eq!(
            lines,
            [
                format!("{ts}\"info\",\"event\":\"sys_startup\",\"version\":\"1.2.3\"}}\n"),
                format!(
                    "{ts}\"warn\",\"event\":\"gm_data_root_mode_repaired\",\"item\":\"secrets/root.key\",\"from\":420,\"to\":384}}\n"
                ),
                format!(
                    "{ts}\"warn\",\"event\":\"gm_data_root_leftover_removed\",\"item\":\"secrets/keys.json\"}}\n"
                ),
                format!(
                    "{ts}\"warn\",\"event\":\"gm_debug_logging_enabled\",\"until\":\"2026-10-03T12:00:00.000Z\"}}\n"
                ),
                format!("{ts}\"warn\",\"event\":\"gm_debug_logging_ended\"}}\n"),
                format!(
                    "{ts}\"error\",\"event\":\"authn_login_fail\",\"addr\":\"203.0.113.7\",\"v\":1,\"pathway\":\"claim_code\",\"cause\":\"credential\"}}\n"
                ),
            ]
        );
    }

    #[test]
    fn turns_data_root_repairs_into_events() {
        let key = DataPath::constant(DataDir::Secrets, "root.key");
        assert_eq!(
            LogEvent::from(&Repair::Mode {
                item: Item::Path(key.clone()),
                from: 0o640,
                to: 0o600,
            }),
            LogEvent::DataRootModeRepaired {
                item: "secrets/root.key".to_owned(),
                from: 0o640,
                to: 0o600,
            }
        );
        assert_eq!(
            LogEvent::from(&Repair::Removed {
                item: Item::Replacement(key),
            }),
            LogEvent::DataRootLeftoverRemoved {
                item: "secrets/.root.key.tmp".to_owned(),
            }
        );
    }

    // ----- The inventory (SEC-PRV-044).

    /// An event as the inventory lists it: name, level and fields with
    /// their classes.
    type Listed = (String, String, Vec<(String, String)>);

    fn string(table: &toml::de::DeTable<'_>, key: &str) -> String {
        table
            .get(key)
            .and_then(|value| value.get_ref().as_str())
            .expect("a string")
            .to_owned()
    }

    fn tables<'a>(table: &'a toml::de::DeTable<'a>, key: &str) -> Vec<&'a toml::de::DeTable<'a>> {
        table
            .get(key)
            .and_then(|value| value.get_ref().as_array())
            .expect("an array")
            .iter()
            .map(|item| item.get_ref().as_table().expect("a table"))
            .collect()
    }

    fn inventory() -> Vec<Listed> {
        let text = include_str!("../log-inventory.toml");
        let table = toml::de::DeTable::parse(text).expect("the inventory parses");
        tables(table.get_ref(), "event")
            .into_iter()
            .map(|event| {
                for key in ["destination", "retention"] {
                    assert!(!string(event, key).trim().is_empty());
                }
                let fields = tables(event, "fields")
                    .into_iter()
                    .map(|field| (string(field, "name"), string(field, "class")))
                    .collect();
                (string(event, "name"), string(event, "level"), fields)
            })
            .collect()
    }

    /// Verifies: SEC-PRV-044
    #[test]
    fn every_event_the_code_can_emit_is_in_the_inventory_as_written() {
        let emitted: Vec<Listed> = one_of_each()
            .iter()
            .map(|event| {
                let fields = event
                    .fields()
                    .iter()
                    .map(|field| (field.key.to_owned(), field.class.name().to_owned()))
                    .collect();
                (
                    event.name().to_owned(),
                    event.level().name().to_owned(),
                    fields,
                )
            })
            .collect();
        assert_eq!(emitted, inventory());
        // `one_of_each` holds every variant: this match fails to compile
        // when a variant is added without a case there.
        for event in one_of_each() {
            match event {
                LogEvent::SysStartup { .. }
                | LogEvent::DataRootModeRepaired { .. }
                | LogEvent::DataRootLeftoverRemoved { .. }
                | LogEvent::DebugLoggingEnabled { .. }
                | LogEvent::DebugLoggingEnded
                | LogEvent::AuthnLoginFail { .. } => {}
            }
        }
    }

    /// Verifies: SEC-OPS-029, SEC-PRV-042
    #[test]
    fn no_inventoried_field_is_a_body_cookie_header_query_or_secret() {
        for (_, _, fields) in inventory() {
            for (field, class) in fields {
                assert_ne!(class, "secret");
                for word in [
                    "body",
                    "cookie",
                    "authorization",
                    "header",
                    "query",
                    "token",
                    "password",
                ] {
                    assert!(!field.contains(word));
                }
            }
        }
    }

    // ----- The logger.

    #[test]
    fn writes_events_at_or_above_its_level_and_drops_the_rest() {
        let (log, _, out) = logger(Level::Warn);
        log.log(&LogEvent::SysStartup { version: "1" });
        log.log(&LogEvent::DebugLoggingEnded);
        assert_eq!(
            out.text(),
            "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"warn\",\"event\":\"gm_debug_logging_ended\"}\n"
        );
        let (log, _, out) = logger(Level::Error);
        log.log(&LogEvent::DebugLoggingEnded);
        assert_eq!(out.text(), "");
    }

    #[test]
    fn stays_at_its_configured_level() {
        let (log, _, out) = logger(Level::Info);
        assert_eq!(log.level(), Level::Info);
        log.log(&LogEvent::SysStartup { version: "1" });
        assert_eq!(
            out.text(),
            "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"info\",\"event\":\"sys_startup\",\"version\":\"1\"}\n"
        );
    }

    fn admin() -> PublicId {
        PublicId::parse("usr_0123456789abcdefghjkmnpqrs", IdKind::User).expect("a user ID")
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn switching_debug_on_records_exactly_one_security_event_first() {
        let (log, _, out) = logger(Level::Info);
        let sink = Recording::new(true);
        assert_eq!(
            log.enable_debug(Duration::from_secs(7_200), Some(admin()), &sink),
            Ok(at(NOON + 7_200_000))
        );
        assert_eq!(
            sink.events(),
            [SecurityEvent::GmDebugLoggingEnabled {
                account: Some(admin())
            }]
        );
        assert_eq!(log.level(), Level::Debug);
        assert_eq!(
            out.text(),
            "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"warn\",\"event\":\"gm_debug_logging_enabled\",\"until\":\"2026-10-03T14:00:00.000Z\"}\n"
        );
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn debug_stays_off_when_the_audit_log_cannot_record_it() {
        let (log, _, out) = logger(Level::Info);
        let sink = Recording::new(false);
        assert_eq!(
            log.enable_debug(Duration::from_secs(60), None, &sink),
            Err(AuditUnavailable)
        );
        assert_eq!(log.level(), Level::Info);
        assert_eq!(out.text(), "");
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn a_clock_that_steps_back_ends_the_debug_window() {
        let (log, clock, _) = logger(Level::Info);
        let sink = Recording::new(true);
        log.enable_debug(Duration::from_secs(3_600), None, &sink)
            .expect("recorded");
        assert_eq!(log.level(), Level::Debug);
        clock.advance(-1);
        assert_eq!(log.level(), Level::Info);
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn debug_switches_itself_off_within_24_hours() {
        let (log, clock, out) = logger(Level::Info);
        let sink = Recording::new(true);
        assert_eq!(
            log.enable_debug(Duration::from_secs(3 * 86_400), None, &sink),
            Ok(at(NOON + MAX_DEBUG_MS))
        );
        clock.advance(MAX_DEBUG_MS - 1);
        assert_eq!(log.level(), Level::Debug);
        clock.advance(1);
        assert_eq!(log.level(), Level::Info);
        assert_eq!(log.level(), Level::Info);
        let lines: Vec<&str> = out.text().leak().lines().collect();
        assert_eq!(
            lines,
            [
                "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"warn\",\"event\":\"gm_debug_logging_enabled\",\"until\":\"2026-10-04T12:00:00.000Z\"}",
                "{\"ts\":\"2026-10-04T12:00:00.000Z\",\"level\":\"warn\",\"event\":\"gm_debug_logging_ended\"}",
            ]
        );
    }

    #[test]
    fn the_configured_level_can_be_set_once_the_configuration_is_read() {
        let (log, clock, out) = logger(Level::Info);
        log.set_base(Level::Error);
        assert_eq!(log.level(), Level::Error);
        log.log(&LogEvent::DebugLoggingEnded);
        assert_eq!(out.text(), "");
        // An open debug window outlives the change and then ends at the new level.
        let sink = Recording::new(true);
        log.enable_debug(Duration::from_millis(5), None, &sink)
            .expect("recorded");
        log.set_base(Level::Warn);
        assert_eq!(log.level(), Level::Debug);
        clock.advance(5);
        assert_eq!(log.level(), Level::Warn);
    }

    #[test]
    fn logging_after_the_window_reverts_first() {
        let (log, clock, out) = logger(Level::Info);
        let sink = Recording::new(true);
        log.enable_debug(Duration::from_millis(10), None, &sink)
            .expect("recorded");
        clock.advance(10);
        log.log(&LogEvent::SysStartup { version: "2" });
        let lines: Vec<&str> = out.text().leak().lines().collect();
        assert_eq!(
            lines[1..],
            [
                "{\"ts\":\"2026-10-03T12:00:00.010Z\",\"level\":\"warn\",\"event\":\"gm_debug_logging_ended\"}",
                "{\"ts\":\"2026-10-03T12:00:00.010Z\",\"level\":\"info\",\"event\":\"sys_startup\",\"version\":\"2\"}",
            ]
        );
    }

    #[test]
    fn a_window_past_the_last_timestamp_ends_at_it() {
        let (log, clock, _) = logger(Level::Info);
        clock.advance(Timestamp::MAX.millis() - NOON - 5);
        let sink = Recording::new(true);
        assert_eq!(
            log.enable_debug(Duration::from_secs(60), None, &sink),
            Ok(Timestamp::MAX)
        );
        assert_eq!(
            log.enable_debug(Duration::MAX, None, &sink),
            Ok(Timestamp::MAX)
        );
    }

    #[test]
    fn the_test_reader_reads_every_escape() {
        let mut chars = r#""\"\\\/\n\r\t\b\féx""#.chars().peekable();
        assert_eq!(read_string(&mut chars), "\"\\/\n\r\t\u{8}\u{c}éx");
    }

    struct Boom;
    impl Write for Boom {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            panic!("writer");
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_poisoned_logger_still_writes() {
        let lock = std::sync::Mutex::new(Level::Info);
        let _ = std::panic::catch_unwind(|| {
            let _guard = lock.lock().expect("first lock");
            panic!("poison");
        });
        assert_eq!(*recover(lock.lock()), Level::Info);
        assert_eq!(Boom.flush().ok(), Some(()));
        let (clock, _) = testing::clock();
        let log = Logger::new(clock, Level::Info, Box::new(Boom));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            log.log(&LogEvent::DebugLoggingEnded);
        }));
        assert_eq!(log.level(), Level::Info);
    }

    #[test]
    fn a_line_that_cannot_be_written_is_lost() {
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::WriteZero))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(Full.flush().ok(), Some(()));
        let (clock, _) = testing::clock();
        let log = Logger::new(clock, Level::Info, Box::new(Full));
        log.log(&LogEvent::DebugLoggingEnded);
        assert_eq!(log.level(), Level::Info);
    }
}
