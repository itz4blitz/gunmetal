//! The harness for the outside-link and return-target validators in
//! `gunmetal_core::link` (SEC-API-047, SEC-API-070).

use gunmetal_core::link::{self, HOME, Link, LinkError};
use gunmetal_core::untrusted::Untrusted;

/// The client routes the return-target validator is given: the home route,
/// one with a wildcard segment and a literal one.
pub const ROUTES: [&str; 3] = ["/", "/albums/*", "/library"];

/// What the link validators reported for one input, read as UTF-8 with
/// each invalid sequence replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`Link::parse`]: the URL to open and the host to show, or why the
    /// value stays plain text.
    pub link: Result<(String, String), LinkError>,
    /// [`link::return_target`] against [`ROUTES`].
    pub return_target: String,
}

/// Feeds `data` to [`Link::parse`] and to [`link::return_target`].
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input. An
/// accepted link must open `https://`, then exactly the host it shows, then
/// at most `:` and a port other than 443 written without leading zeros,
/// then `/`; its host must be printable ASCII without a character that ends
/// a host or escapes one; and it must read back unchanged. A return target
/// must be the value itself or the home route, and must start with exactly
/// one `/` that no `/` or `\` follows.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let raw = String::from_utf8_lossy(data);
    let parsed = Link::parse(Untrusted::new(&raw));
    if let Ok(accepted) = &parsed {
        let (href, host) = (accepted.href(), accepted.host());
        assert!(
            !host.is_empty()
                && host
                    .bytes()
                    .all(|octet| octet.is_ascii_graphic() && !b"/\\?#@%".contains(&octet))
                && href
                    .strip_prefix("https://")
                    .and_then(|rest| rest.strip_prefix(host))
                    .and_then(|after_host| match after_host.strip_prefix(':') {
                        Some(port_and_path) => {
                            let (port, path) = port_and_path
                                .split_at(port_and_path.find('/').unwrap_or(port_and_path.len()));
                            port.parse::<u16>()
                                .ok()
                                .filter(|&number| number != 443 && number.to_string() == port)
                                .map(|_| path)
                        }
                        None => Some(after_host),
                    })
                    .is_some_and(|path| path.starts_with('/'))
                && Link::parse(Untrusted::new(href)).as_ref() == Ok(accepted),
            "{raw:?} gave {accepted:?}"
        );
    }
    let target = link::return_target(Untrusted::new(&raw), &ROUTES);
    let path = target.as_str();
    assert!(
        (path == raw || path == HOME)
            && path
                .strip_prefix('/')
                .is_some_and(|rest| !rest.starts_with(['/', '\\'])),
        "{raw:?} returned to {path:?}"
    );
    Outcome {
        link: parsed.map(|accepted| (accepted.href().to_owned(), accepted.host().to_owned())),
        return_target: path.to_owned(),
    }
}
