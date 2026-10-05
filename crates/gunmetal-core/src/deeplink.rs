//! Inbound links: what a link, a QR payload or an address with a fragment
//! asks a client to do.
//!
//! [`parse_link`] is the one function that reads them, on every platform,
//! the web client included through WASM (SEC-CLI-025). It takes the whole
//! text as it arrived and returns a [`Route`] from a closed set, so no
//! client takes an address apart itself. Anything else is
//! [`Route::NotRecognised`], with no reason given: no caller can treat a
//! near miss differently, and nothing from the text, which may hold a
//! secret, is echoed back.
//!
//! In R1 the set holds the four links R1 issues. Each is the server's
//! origin, a fixed path, and in the fragment a secret, which therefore
//! never reaches a server log (SEC-NET-036):
//!
//! ```text
//! <origin>/claim#<claim code>                 claim a new server
//! <origin>/invite#<secret>                    join by invitation
//! <origin>/pair#<pairing code>.<server key>   approve a browser
//! <origin>/recover#<secret>                   enrol a passkey after recovery
//! ```
//!
//! The origin is `https://` and a host, with a port unless it is 443. It is
//! read by [`crate::link::Link::parse`], the one URL reader, so a link names
//! the server a browser would open, and a host with a letter from another
//! script, a user name before the host and everything else that reader
//! refuses is not a link here either. The one origin without TLS is this
//! machine itself, `http://localhost` with a port unless it is 80, where a
//! server is claimed before it has a name (SEC-NET-001). The claim code and
//! the pairing code are the grouped text of [`crate::otp`]. A secret is 16
//! octets and the server key 32, in URL-safe base64 without padding.
//!
//! Exactly one spelling of each link is read: the one its server writes,
//! which is also what a browser's address bar holds after opening it. Upper
//! case in the scheme or the host, a default port written out, a backslash,
//! a space, a tab, an escape, a lower-case code and a padded secret are all
//! not recognised, so two readers cannot disagree about a link that was
//! read. A code typed by hand is not a link: it is read where it is typed,
//! by [`crate::otp::parse_code`].
//!
//! Reading a link does nothing and grants nothing. Every route that would
//! change something says so through [`Route::needs_confirmation`], and a
//! client acts on it only after a screen that names the server and says
//! what will happen (SEC-CLI-013). Resolving a route on the server and
//! redeeming a code or a secret are other modules' work.

use crate::base64::{self, Alphabet};
use crate::link::Link;
use crate::otp::{self, ClaimCode, Code, CodeKind, PairingCode};
use crate::untrusted::Untrusted;

/// The longest text read, in octets. No link a server writes is longer than
/// 326: `https://`, a name of at most 253 octets, a port, and the longest
/// path and fragment, the pairing link's 59. Anything over this limit is
/// not recognised before any of it is copied or decoded, which bounds the
/// work on every input (SEC-TM-032) and the length of the server's address
/// a confirmation screen has to show.
const MAX_LINK_LEN: usize = 512;

/// What an inbound link asks a client to do.
///
/// The set is closed. A route other than [`Route::NotRecognised`] comes
/// only from [`parse_link`], because the server, the secret and the server
/// key inside it have no other constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Claim a new server with its one-time setup code (ACC-001).
    Claim {
        /// The server to claim.
        server: Server,
        /// The setup code the server printed.
        code: ClaimCode,
    },
    /// Join a server through an invitation (ACC-080).
    Invitation {
        /// The server that issued the invitation.
        server: Server,
        /// The invitation's secret.
        secret: LinkSecret,
    },
    /// Approve a browser that asked to be signed in, from a device that is
    /// signed in already (ACC-062).
    Pairing {
        /// The server the asking browser is connected to.
        server: Server,
        /// The code the asking browser shows.
        code: PairingCode,
        /// That server's identity key, which a client that pinned one
        /// compares with its own (SEC-IAM-057).
        key: ServerKey,
    },
    /// Enrol a new passkey through a recovery link (ACC-004, ACC-064).
    Recovery {
        /// The server that issued the link.
        server: Server,
        /// The link's secret.
        secret: LinkSecret,
    },
    /// Anything that is not exactly a link a server issues.
    NotRecognised,
}

impl Route {
    /// Whether a client must show a confirmation screen, and have the
    /// person agree, before it acts on this route.
    ///
    /// Claiming a server, joining one, approving a browser and enrolling a
    /// passkey each change something, so each needs one; text that was not
    /// recognised asks for nothing. Every route is named here, so one added
    /// later does not compile until it answers too (SEC-CLI-025).
    #[must_use]
    pub const fn needs_confirmation(&self) -> bool {
        match self {
            Self::Claim { .. }
            | Self::Invitation { .. }
            | Self::Pairing { .. }
            | Self::Recovery { .. } => true,
            Self::NotRecognised => false,
        }
    }
}

/// The server a link belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server(String);

impl Server {
    /// The server's origin as a browser writes it: `https://`, the host and
    /// the port unless it is 443, or `http://localhost` and the port unless
    /// it is 80. It is the address a confirmation screen names, and two
    /// links to one origin give the same text.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.0
    }
}

/// The 128-bit secret of an invitation or of a recovery link, which its
/// holder presents once to redeem it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkSecret([u8; 16]);

impl LinkSecret {
    /// The 16 octets.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }

    /// The secret as its link carries it: 22 characters of URL-safe base64,
    /// without padding.
    #[must_use]
    pub fn text(self) -> String {
        base64::encode(&self.0, Alphabet::UrlSafe)
    }
}

/// A server's identity key as a pairing link carries it: 32 octets, which
/// this module does not check to be a key. A client that pinned the
/// server's key compares the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerKey([u8; 32]);

impl ServerKey {
    /// The 32 octets.
    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Reads a link, a QR payload or an address with its fragment, whole and
/// exactly as it arrived.
///
/// The text arrives [`Untrusted`] (SEC-TM-031). The answer is one of the
/// routes R1 issues, or [`Route::NotRecognised`] for anything else; there
/// is no error and no partial reading.
#[must_use]
pub fn parse_link(input: Untrusted<&str>) -> Route {
    recognise(input.into_inner()).unwrap_or(Route::NotRecognised)
}

/// The route whose one spelling `raw` is, if it is the spelling of any.
fn recognise(raw: &str) -> Option<Route> {
    if raw.len() > MAX_LINK_LEN {
        return None;
    }
    let (server, tail) = server_and_tail(raw)?;
    let (path, fragment) = tail.split_once('#')?;
    match path {
        "claim" => claim(fragment).map(|code| Route::Claim { server, code }),
        "invite" => link_secret(fragment).map(|secret| Route::Invitation { server, secret }),
        "pair" => pairing(fragment).map(|(code, key)| Route::Pairing { server, code, key }),
        "recover" => link_secret(fragment).map(|secret| Route::Recovery { server, secret }),
        _ => None,
    }
}

/// Splits `raw` into the server it names and what follows the `/` after
/// the server's origin.
///
/// An `https` origin is one only when `raw` is, to the octet, the URL the
/// one URL reader writes for it, which leaves a single spelling of the
/// scheme, the host and the port, and nothing between them and the path
/// that another reader could take for part of either.
fn server_and_tail(raw: &str) -> Option<(Server, &str)> {
    let (scheme, rest) = raw.split_once("://")?;
    let (authority, tail) = rest.split_once('/')?;
    let named = match scheme {
        "https" => Link::parse(Untrusted::new(raw)).is_ok_and(|link| link.href() == raw),
        "http" => this_machine(authority),
        _ => false,
    };
    named.then(|| (Server(format!("{scheme}://{authority}")), tail))
}

/// Whether `authority` is this machine as a browser writes it: `localhost`,
/// alone or with a port in plain decimal that is not 80, the port a browser
/// leaves out.
fn this_machine(authority: &str) -> bool {
    authority == "localhost"
        || authority
            .strip_prefix("localhost:")
            .and_then(|port| port.parse::<u16>().ok())
            .is_some_and(|port| port != 80 && authority == format!("localhost:{port}"))
}

/// A claim code written as its server writes it. The spellings a person may
/// type, in lower case or without hyphens, are not what a link carries.
fn claim(text: &str) -> Option<ClaimCode> {
    match otp::parse_code(Untrusted::new(text), CodeKind::Claim) {
        Ok(Code::Claim(code)) if code.text() == text => Some(code),
        _ => None,
    }
}

/// A pairing code written as its server writes it.
fn user_code(text: &str) -> Option<PairingCode> {
    match otp::parse_code(Untrusted::new(text), CodeKind::Pairing) {
        Ok(Code::Pairing(code)) if code.text() == text => Some(code),
        _ => None,
    }
}

/// The fragment of a pairing link: the pairing code, a dot, and the
/// server's identity key.
fn pairing(fragment: &str) -> Option<(PairingCode, ServerKey)> {
    let (code, key) = fragment.split_once('.')?;
    Some((user_code(code)?, ServerKey(octets(key)?)))
}

/// The fragment of an invitation or of a recovery link.
fn link_secret(text: &str) -> Option<LinkSecret> {
    octets(text).map(LinkSecret)
}

/// Exactly `N` octets written in URL-safe base64 without padding. The
/// decoder also reads padded text, so only text that encodes back to
/// itself is taken, which leaves one spelling.
fn octets<const N: usize>(text: &str) -> Option<[u8; N]> {
    let decoded = base64::decode(Untrusted::new(text.as_bytes()), Alphabet::UrlSafe, N).ok()?;
    let exact = <[u8; N]>::try_from(decoded).ok()?;
    (base64::encode(&exact, Alphabet::UrlSafe) == text).then_some(exact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The octets of the claim code that is written as [`CODE`].
    const CLAIM: [u8; 16] = [
        0x01, 0x10, 0xC8, 0x53, 0x1D, 0x09, 0x52, 0xD8, 0xD7, 0x3E, 0x11, 0x94, 0xE9, 0x5B, 0x5F,
        0x19,
    ];
    /// A claim code as its server writes it.
    const CODE: &str = "01234-56789-ABCDE-FGHJK-MNPQR-S9";
    /// The octets 0 to 15, which URL-safe base64 without padding writes as
    /// `AAECAwQFBgcICQoLDA0ODw`.
    const SECRET: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    /// The octets 0 to 31, which it writes as
    /// `AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8`.
    const KEY: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    /// The random octets behind the pairing code `WDJB-MJHT`.
    const USER: [u8; 8] = [17, 2, 6, 0, 9, 6, 5, 15];
    /// The paths of the four routes.
    const PATHS: [&str; 4] = ["claim", "invite", "pair", "recover"];

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a 256 KiB stack, so a reader that recursed would fail
    /// its test instead of passing on the runner's larger stack
    /// (SEC-MED-001).
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn parse(raw: &str) -> Route {
        parse_link(Untrusted::new(raw))
    }

    fn server(origin: &str) -> Server {
        Server(origin.to_owned())
    }

    /// The route of a claim link on `origin` that carries [`CODE`].
    fn claim_on(origin: &str) -> Route {
        Route::Claim {
            server: server(origin),
            code: crate::otp::claim_code(CLAIM),
        }
    }

    /// The route of an invitation on `origin` whose secret is [`SECRET`].
    fn invitation_on(origin: &str) -> Route {
        Route::Invitation {
            server: server(origin),
            secret: LinkSecret(SECRET),
        }
    }

    /// The route of a pairing link on `origin` for [`USER`] and [`KEY`].
    fn pairing_on(origin: &str) -> Route {
        Route::Pairing {
            server: server(origin),
            code: crate::otp::pairing_code(USER),
            key: ServerKey(KEY),
        }
    }

    /// The route of a recovery link on `origin` whose secret is [`SECRET`].
    fn recovery_on(origin: &str) -> Route {
        Route::Recovery {
            server: server(origin),
            secret: LinkSecret(SECRET),
        }
    }

    /// URL-safe base64 without padding, from the codec the core shares. The
    /// reader under test takes no part in it.
    fn unpadded(octets: &[u8]) -> String {
        base64::encode(octets, Alphabet::UrlSafe)
    }

    /// The link a server writes for `route`: its origin, its path and, in
    /// the fragment, its code or secret as text. It is written here on its
    /// own, so the reader is checked against it and not against itself.
    fn written(route: &Route) -> String {
        match route {
            Route::Claim { server, code } => format!("{}/claim#{}", server.origin(), code.text()),
            Route::Invitation { server, secret } => {
                format!("{}/invite#{}", server.origin(), unpadded(&secret.bytes()))
            }
            Route::Pairing { server, code, key } => format!(
                "{}/pair#{}.{}",
                server.origin(),
                code.text(),
                unpadded(&key.bytes())
            ),
            Route::Recovery { server, secret } => {
                format!("{}/recover#{}", server.origin(), unpadded(&secret.bytes()))
            }
            Route::NotRecognised => String::new(),
        }
    }

    /// Whether acting on `route` would change anything, from what each
    /// route is for: claiming a server, joining one, approving a browser
    /// and enrolling a passkey all do, and text that was not recognised
    /// does not.
    fn changes_state(route: &Route) -> bool {
        match route {
            Route::Claim { .. }
            | Route::Invitation { .. }
            | Route::Pairing { .. }
            | Route::Recovery { .. } => true,
            Route::NotRecognised => false,
        }
    }

    /// Asserts that none of `texts` is recognised.
    fn assert_not_recognised(texts: &[&str]) {
        for raw in texts {
            assert_eq!(parse(raw), Route::NotRecognised, "{raw:?}");
        }
    }

    /// `count` copies of `symbol`.
    fn run_of(symbol: char, count: usize) -> String {
        (0..count).map(|_| symbol).collect()
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_claim_link() {
        assert_eq!(
            parse("https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9"),
            Route::Claim {
                server: server("https://music.example"),
                code: crate::otp::claim_code(CLAIM),
            }
        );
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_an_invitation_link() {
        assert_eq!(
            parse("https://music.example/invite#AAECAwQFBgcICQoLDA0ODw"),
            Route::Invitation {
                server: server("https://music.example"),
                secret: LinkSecret(SECRET),
            }
        );
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_pairing_link() {
        assert_eq!(
            parse(
                "https://music.example:8443/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"
            ),
            Route::Pairing {
                server: server("https://music.example:8443"),
                code: crate::otp::pairing_code(USER),
                key: ServerKey(KEY),
            }
        );
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_recovery_link() {
        assert_eq!(
            parse("http://localhost:8096/recover#AAECAwQFBgcICQoLDA0ODw"),
            Route::Recovery {
                server: server("http://localhost:8096"),
                secret: LinkSecret(SECRET),
            }
        );
    }

    /// A claim code ends in one of 37 check symbols, and five of them are
    /// neither letters nor digits. A fragment holds each of them as it is,
    /// so a link with any of them is read.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_claim_code_that_ends_in_any_kind_of_check_symbol() {
        let cases: [([u8; 16], &str); 6] = [
            ([0; 16], "00000-00000-00000-00000-00000-00"),
            ([0xFF; 16], "7ZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-Z*"),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 33],
                "00000-00000-00000-00000-00001-1~",
            ),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34],
                "00000-00000-00000-00000-00001-2$",
            ),
            (
                [0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "40000-00000-00000-00000-00000-0=",
            ),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36],
                "00000-00000-00000-00000-00001-4U",
            ),
        ];
        for (octets, text) in cases {
            assert_eq!(
                parse(&format!("https://music.example/claim#{text}")),
                Route::Claim {
                    server: server("https://music.example"),
                    code: crate::otp::claim_code(octets),
                },
                "{text}"
            );
        }
    }

    /// The two symbols URL-safe base64 has besides letters and digits.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_secret_written_with_hyphens_or_underscores() {
        assert_eq!(
            parse("https://music.example/invite#---------------------w"),
            Route::Invitation {
                server: server("https://music.example"),
                secret: LinkSecret([
                    0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB,
                    0xEF, 0xBE, 0xFB,
                ]),
            }
        );
        assert_eq!(
            parse("https://music.example/recover#_____________________w"),
            Route::Recovery {
                server: server("https://music.example"),
                secret: LinkSecret([0xFF; 16]),
            }
        );
    }

    /// Each origin here is written as a browser's address bar shows it, and
    /// is handed back unchanged as the server's name.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn names_the_server_by_its_origin_as_a_browser_writes_it() {
        for origin in [
            "https://music.example",
            "https://music.example:8443",
            "https://music.example:80",
            "https://music.example:65535",
            "https://music.example.",
            "https://a_b.music.example",
            "https://xn--bcher-kva.example",
            "https://192.0.2.1",
            "https://192.0.2.1:8443",
            "https://[2001:db8::1]",
            "https://[2001:db8::1]:8443",
            "https://localhost",
            "https://localhost:8096",
            "http://localhost",
            "http://localhost:8096",
            "http://localhost:443",
            "http://localhost:1",
            "http://localhost:65535",
        ] {
            assert_eq!(
                parse(&format!("{origin}/claim#{CODE}")),
                claim_on(origin),
                "{origin}"
            );
        }
    }

    /// A browser sends nothing but a help page over plain HTTP to another
    /// machine (SEC-NET-001), so the one plain origin is `localhost`, and
    /// only as a browser writes it: lower case, and the port in plain
    /// decimal unless it is 80.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_plain_http_only_for_this_machine_written_one_way() {
        for start in [
            "http://music.example",
            "http://music.example/localhost",
            "http://192.0.2.1",
            "http://127.0.0.1",
            "http://127.0.0.1:8096",
            "http://[::1]",
            "http://localhost.",
            "http://localhost.music.example",
            "http://localhost@music.example",
            "http://localhost:8096@music.example",
            "http://LOCALHOST",
            "http://Localhost:8096",
            "HTTP://localhost",
            "Http://localhost:8096",
            " http://localhost",
            "http://localhost:80",
            "http://localhost:",
            "http://localhost:08096",
            "http://localhost:+8096",
            "http://localhost:65536",
            "http://localhost:8096x",
            "http://localhost:-1",
            "http://localhost: 8096",
            "http://localhost:8096:8096",
            "http:/localhost",
            "http:localhost",
            "http:///localhost",
            "http://localhost#",
            "http://localhost?",
            "http://localhost\\",
            "ftp://localhost",
            "ws://localhost:8096",
            "gunmetal://localhost",
            "://localhost",
            "//localhost",
            "localhost",
            "localhost:8096",
        ] {
            assert_eq!(
                parse(&format!("{start}/claim#{CODE}")),
                Route::NotRecognised,
                "{start:?}"
            );
        }
    }

    /// Every one of these opens `music.example`, or looks as if it did, in
    /// some URL reader. None is what a server writes or an address bar
    /// shows, so none is read, and a link that is read means the same to
    /// every reader.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_an_https_origin_only_as_a_browser_writes_it() {
        for start in [
            "HTTPS://music.example",
            "Https://music.example",
            "https://Music.Example",
            "https://MUSIC.EXAMPLE",
            "https://music.example:443",
            "https://music.example:08443",
            "https://music.example:",
            " https://music.example",
            "\u{FEFF}https://music.example",
            "ht\ttps://music.example",
            "https:\\\\music.example",
            "https://music.example\\",
            "https:music.example",
            "https:/music.example",
            "https:///music.example",
            "https://0xC0.0.2.1",
            "https://3221225985",
            "https://192.0.2.1.",
            "https://[2001:DB8::1]",
            "https://[2001:db8:0:0:0:0:0:1]",
            "https://%6Dusic.example",
            "https://music.example@evil.example",
            "https://user:secret@music.example",
            "https://evil.example\\@music.example",
            "https://mus\u{456}c.example",
            "https://m\u{FC}sic.example",
            "https://music example",
            "https://music.example:65536",
            "https://music.example:x",
            "https://",
            "https://:8443",
            "https://music.example?",
            "https://music.example#",
        ] {
            assert_eq!(
                parse(&format!("{start}/claim#{CODE}")),
                Route::NotRecognised,
                "{start:?}"
            );
        }
    }

    /// Text with no server in it, and schemes that name none here. A code
    /// or a secret on its own is not a link, and no custom scheme can carry
    /// one (SEC-CLI-039).
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_text_that_names_no_server() {
        assert_not_recognised(&[
            "",
            " ",
            "claim",
            "/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "WDJB-MJHT",
            "AAECAwQFBgcICQoLDA0ODw",
            "music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "//music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "javascript:alert(1)",
            "javascript://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "data:text/html,https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "file:///claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "gunmetal://claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "gunmetal://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "intent://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "wss://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https+x://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
        ]);
    }

    /// The path is one of four words, written exactly, and the code or the
    /// secret is in the fragment and nowhere else: a query string reaches
    /// the server's log (SEC-NET-036).
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_another_path_or_a_secret_outside_the_fragment() {
        assert_not_recognised(&[
            "https://music.example",
            "https://music.example/",
            "https://music.example/claim",
            "https://music.example/claim#",
            "https://music.example/#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim?01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim?code=01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim?x=1#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim/01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim/#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example//claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/x/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/x/../claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/./claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/Claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/CLAIM#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claims#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/clai#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/cl%61im#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim%2301234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim;x#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/#claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/#/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/library",
            "https://music.example/albums/alb_0123456789abcdefghjkmnpqrs",
            "https://music.example/albums#alb_0123456789abcdefghjkmnpqrs",
        ]);
    }

    /// A link carries its claim code exactly as the server printed it. The
    /// spellings a person may type by hand belong to the code field, and a
    /// code of another kind or with a wrong check symbol is no claim code.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_a_claim_code_written_any_other_way() {
        assert_not_recognised(&[
            "https://music.example/claim#01234-56789-abcde-fghjk-mnpqr-s9",
            "https://music.example/claim#0123456789ABCDEFGHJKMNPQRS9",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQRS9",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S-9",
            "https://music.example/claim#-01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim#O1234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim#0I234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S8",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S99",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9#",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9#x",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9 ",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9\n",
            "https://music.example/claim#01234%2D56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9%20",
            "https://music.example/claim#80000-00000-00000-00000-00000-00",
            "https://music.example/claim#00000-00000-00000-00000-00001-4u",
            "https://music.example/claim#0000-0000-0000-0000-0",
            "https://music.example/claim#WDJB-MJHT",
            "https://music.example/claim#AAECAwQFBgcICQoLDA0ODw",
            "http://localhost:8096/claim#01234-56789-abcde-fghjk-mnpqr-s9",
            "http://localhost:8096/claim#01234 56789 ABCDE FGHJK MNPQR S9",
            "http://localhost:8096/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9\n",
        ]);
    }

    /// An invitation and a recovery link each carry exactly 16 octets, in
    /// the one spelling URL-safe base64 has without padding.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_a_secret_that_is_not_sixteen_octets_written_one_way() {
        for fragment in [
            "",
            "AAECAwQFBgcICQoLDA0O",
            "AAECAwQFBgcICQoLDA0OD",
            "AAECAwQFBgcICQoLDA0ODxA",
            "AAECAwQFBgcICQoLDA0ODw=",
            "AAECAwQFBgcICQoLDA0ODw==",
            "AAECAwQFBgcICQoLDA0ODw%3D%3D",
            "AAECAwQFBgcICQoLDA0ODx",
            "AAECAwQFBgcICQoLDA0OD.",
            "/////////////////////w",
            "+++++++++++++++++++++w",
            "000102030405060708090a0b0c0d0e0f",
            "inv_0123456789abcdefghjkmnpqrs",
            "01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "AAECAwQFBgcICQoLDA0ODw.AAECAwQFBgcICQoLDA0ODw",
            "AAECAwQFBgcICQoLDA0ODw#AAECAwQFBgcICQoLDA0ODw",
        ] {
            for path in ["invite", "recover"] {
                for origin in ["https://music.example", "http://localhost:8096"] {
                    assert_eq!(
                        parse(&format!("{origin}/{path}#{fragment}")),
                        Route::NotRecognised,
                        "{origin}/{path}#{fragment}"
                    );
                }
            }
        }
    }

    /// A pairing link carries the code and the server's key, each written
    /// one way, with one dot between them.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_a_pairing_fragment_without_its_code_and_its_key() {
        for fragment in [
            "",
            ".",
            "WDJB-MJHT",
            "WDJB-MJHT.",
            ".AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8.WDJB-MJHT",
            "WDJB-MJHT:AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJHT,AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJHT..AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "wdjb-mjht.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJBMJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJ-BMJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJHA.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJH.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJHTT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "01234-56789-ABCDE-FGHJK-MNPQR-S9.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8A",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh9",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODw",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8.x",
            "WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8#x",
        ] {
            for origin in ["https://music.example", "http://localhost:8096"] {
                assert_eq!(
                    parse(&format!("{origin}/pair#{fragment}")),
                    Route::NotRecognised,
                    "{origin}/pair#{fragment}"
                );
            }
        }
    }

    /// A fragment is read as its own path's kind and no other: a claim code
    /// is no secret, a secret is no claim code, and neither is a pairing
    /// code with a key.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn does_not_recognise_one_routes_fragment_on_another_routes_path() {
        assert_not_recognised(&[
            "https://music.example/invite#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/pair#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/recover#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/claim#AAECAwQFBgcICQoLDA0ODw",
            "https://music.example/pair#AAECAwQFBgcICQoLDA0ODw",
            "https://music.example/claim#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "https://music.example/invite#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "https://music.example/recover#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
        ]);
    }

    /// Nothing longer than 512 octets is read, however it is spelled, and a
    /// link of exactly 512 octets is.
    ///
    /// Verifies: SEC-TM-032
    #[test]
    fn reads_nothing_longer_than_512_octets() {
        let name = run_of('a', 465);
        let longest = format!("https://{name}/claim#{CODE}");
        assert_eq!(longest.len(), 512);
        assert_eq!(parse(&longest), claim_on(&format!("https://{name}")));
        let longer = format!("https://a{name}/claim#{CODE}");
        assert_eq!(longer.len(), 513);
        assert_eq!(parse(&longer), Route::NotRecognised);
        let far_longer = format!("https://music.example/claim#{}", run_of('0', 65_536));
        assert_eq!(parse(&far_longer), Route::NotRecognised);
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn asks_for_a_confirmation_screen_on_every_route_that_changes_something() {
        let cases = [
            (claim_on("https://music.example"), true),
            (invitation_on("https://music.example"), true),
            (pairing_on("https://music.example"), true),
            (recovery_on("https://music.example"), true),
            (Route::NotRecognised, false),
        ];
        for (route, asks) in cases {
            assert_eq!(route.needs_confirmation(), asks, "{route:?}");
            assert_eq!(changes_state(&route), asks, "{route:?}");
        }
        for raw in [
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
            "https://music.example/invite#AAECAwQFBgcICQoLDA0ODw",
            "https://music.example/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            "https://music.example/recover#AAECAwQFBgcICQoLDA0ODw",
        ] {
            assert!(parse(raw).needs_confirmation(), "{raw}");
        }
        assert!(!parse("https://music.example/library").needs_confirmation());
    }

    #[test]
    fn hands_back_the_parts_of_a_route_as_they_were_written() {
        assert_eq!(
            server("https://music.example:8443").origin(),
            "https://music.example:8443"
        );
        assert_eq!(LinkSecret(SECRET).bytes(), SECRET);
        assert_eq!(LinkSecret(SECRET).text(), "AAECAwQFBgcICQoLDA0ODw");
        assert_eq!(LinkSecret([0xFF; 16]).text(), "_____________________w");
        assert_eq!(ServerKey(KEY).bytes(), KEY);
    }

    /// The reference writer the property tests rely on, against literal
    /// links.
    #[test]
    fn the_reference_writer_writes_literal_links() {
        assert_eq!(
            written(&claim_on("https://music.example")),
            "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9"
        );
        assert_eq!(
            written(&invitation_on("https://music.example")),
            "https://music.example/invite#AAECAwQFBgcICQoLDA0ODw"
        );
        assert_eq!(
            written(&pairing_on("https://music.example:8443")),
            "https://music.example:8443/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"
        );
        assert_eq!(
            written(&recovery_on("http://localhost:8096")),
            "http://localhost:8096/recover#AAECAwQFBgcICQoLDA0ODw"
        );
        assert_eq!(written(&Route::NotRecognised), "");
    }

    /// Origins as a browser writes them: a name, an address or this
    /// machine, alone or with a port that is not the scheme's own.
    fn origins() -> impl Strategy<Value = String> {
        let host = prop_oneof![
            "[a-z][a-z0-9-]{0,8}(\\.[a-z][a-z0-9-]{0,8}){0,2}"
                .prop_map(|name| format!("https://{name}")),
            Just("https://192.0.2.1".to_owned()),
            Just("https://[2001:db8::1]".to_owned()),
            Just("http://localhost".to_owned()),
        ];
        (host, proptest::option::of(1024_u16..=65_535)).prop_map(|(host, port)| match port {
            Some(port) => format!("{host}:{port}"),
            None => host,
        })
    }

    /// A link a server writes, in its three parts: the origin, the path
    /// and the fragment.
    fn parts() -> impl Strategy<Value = (String, &'static str, String)> {
        let path_and_fragment = prop_oneof![
            any::<[u8; 16]>().prop_map(|octets| ("claim", crate::otp::claim_code(octets).text())),
            any::<[u8; 16]>().prop_map(|octets| ("invite", unpadded(&octets))),
            (any::<[u8; 8]>(), any::<[u8; 32]>()).prop_map(|(user, key)| {
                let code = crate::otp::pairing_code(user).text();
                ("pair", format!("{code}.{}", unpadded(&key)))
            }),
            any::<[u8; 16]>().prop_map(|octets| ("recover", unpadded(&octets))),
        ];
        (origins(), path_and_fragment)
            .prop_map(|(origin, (path, fragment))| (origin, path, fragment))
    }

    proptest! {
        /// Verifies: SEC-CLI-025
        #[test]
        fn reads_every_link_a_server_writes(
            origin in origins(),
            claim_octets in any::<[u8; 16]>(),
            secret in any::<[u8; 16]>(),
            user in any::<[u8; 8]>(),
            key in any::<[u8; 32]>(),
        ) {
            let claim_code = crate::otp::claim_code(claim_octets);
            prop_assert_eq!(
                parse(&format!("{origin}/claim#{}", claim_code.text())),
                Route::Claim { server: server(&origin), code: claim_code }
            );
            prop_assert_eq!(
                parse(&format!("{origin}/invite#{}", unpadded(&secret))),
                Route::Invitation { server: server(&origin), secret: LinkSecret(secret) }
            );
            let pairing_code = crate::otp::pairing_code(user);
            prop_assert_eq!(
                parse(&format!("{origin}/pair#{}.{}", pairing_code.text(), unpadded(&key))),
                Route::Pairing { server: server(&origin), code: pairing_code, key: ServerKey(key) }
            );
            prop_assert_eq!(
                parse(&format!("{origin}/recover#{}", unpadded(&secret))),
                Route::Recovery { server: server(&origin), secret: LinkSecret(secret) }
            );
        }

        /// One change to a link a server writes, of the kinds a careless
        /// or hostile writer makes, and it is no longer a link.
        ///
        /// Verifies: SEC-CLI-025
        #[test]
        fn does_not_recognise_a_written_link_after_any_one_change(
            (origin, path, fragment) in parts(),
            word in "[a-z]{1,8}".prop_filter("not a route's path", |word| !PATHS.contains(&word.as_str())),
        ) {
            let mut shorter = fragment.clone();
            shorter.truncate(shorter.len().saturating_sub(1));
            let changed = [
                format!("{origin}/{path}?{fragment}"),
                format!("{origin}/{path}"),
                format!("{origin}/{path}#{fragment}A"),
                format!("{origin}/{path}#{shorter}"),
                format!("{origin}/{path}#{fragment} "),
                format!(" {origin}/{path}#{fragment}"),
                format!("{origin}/{word}#{fragment}"),
                format!("{origin}/x/{path}#{fragment}"),
                format!("{origin}/{path}/#{fragment}"),
                format!("{origin}//{path}#{fragment}"),
                format!("{}/{path}#{fragment}", origin.to_uppercase()),
            ];
            for raw in changed {
                prop_assert_eq!(parse(&raw), Route::NotRecognised, "{:?}", raw);
            }
        }

        /// Whatever the text, the reader returns, on a small stack. What it
        /// recognises is, to the octet, the link written for the route it
        /// answered, and only a route that changes something asks for a
        /// confirmation screen.
        ///
        /// Verifies: SEC-CLI-025, SEC-MED-001
        #[test]
        fn reads_only_what_a_server_writes_and_returns_for_any_text(
            raw in prop_oneof![
                vec(any::<u8>(), 0..256)
                    .prop_map(|octets| String::from_utf8_lossy(&octets).into_owned()),
                "(?s).{0,64}",
                parts().prop_map(|(origin, path, fragment)| format!("{origin}/{path}#{fragment}")),
                "(https?://)?(localhost|music\\.example)(:[0-9]{1,5})?/(claim|invite|pair|recover)[#?][0-9A-Za-z*~$=._-]{0,48}",
            ],
        ) {
            let text = raw.clone();
            let route = on_small_stack(move || parse(&text));
            prop_assert!(
                written(&route) == raw || route == Route::NotRecognised,
                "{raw:?} gave {route:?}"
            );
            prop_assert_eq!(route.needs_confirmation(), changes_state(&route), "{:?}", raw);
        }
    }
}
