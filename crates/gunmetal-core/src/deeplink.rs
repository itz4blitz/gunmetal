//! Inbound links: what a link, a QR payload or an address with a fragment
//! asks a client to do.
//!
//! [`parse_link`] is the one function that reads them, on every platform,
//! the web client included through WASM (SEC-CLI-025). It takes the whole
//! text as it arrived and returns a [`Route`] from a closed set, so no
//! client takes an address apart itself. Anything else is
//! [`Route::NotRecognised`], with no reason given: no caller can treat a
//! near miss differently, and nothing of a text that was not recognised is
//! kept or handed back.
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
//! refuses is not a link here either. A host is also no longer than a name
//! can be, 253 octets in labels of at most 63, so that a confirmation
//! screen can show all of it. The one origin without TLS is this machine
//! itself, `http://localhost` with a port unless it is 80, where a server
//! is claimed before it has a name (SEC-NET-001). The claim code and the
//! pairing code are the grouped text of [`crate::otp`]. A secret is 16
//! octets and the server key 32, in URL-safe base64 without padding.
//!
//! Exactly one spelling of each link is read: the one its server writes.
//! With one exception, that is also what a browser's address bar holds
//! after opening the link. The exception is a host that is an IPv6 address
//! holding an IPv4 one. The URL reader writes the last four octets of such
//! an address dotted, as in `[::ffff:192.0.2.1]`, and only that spelling is
//! read; an address bar holds `[::ffff:c000:201]`, so a link to such a host
//! that was copied from one is not recognised. Upper case in the scheme or
//! the host, a default port written out, a backslash, a space, a tab, an
//! escape, a lower-case code and a padded secret are all not recognised
//! either, so two readers cannot disagree about a link that was read. A
//! code typed by hand is not a link: it is read where it is typed, by
//! [`crate::otp::parse_code`].
//!
//! What a recognised link carried stays out of logs (SEC-IAM-095,
//! SEC-OPS-013). The secret of an invitation or of a recovery link is a
//! [`LinkSecret`], which has no `==`, no `Display` and no serialised form,
//! and whose `Debug` form is a fixed word. A [`Route`] has no `==` either,
//! and its `Debug` form holds the kind of link and the server's name and
//! nothing else, so neither a secret nor a claim code or a pairing code
//! reaches a log line through a route, whatever the code's own type
//! prints. A [`LinkSecret`] is not the secrets crate's wrapper, which also
//! wipes its value when dropped: that crate depends on this one, so a link
//! secret's octets are not wiped here.
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

/// The longest text read, in octets, which is the longest link a server
/// can write. That is a pairing link on the longest origin, and it adds up
/// to 326:
///
/// ```text
///   8  https://
/// 253  the host, at most MAX_HOST_LEN
///   6  :65535, a colon and a port of five digits
///   6  /pair#
///   9  the pairing code, XXXX-XXXX
///   1  the dot between the code and the key
///  43  the server key's 32 octets in URL-safe base64 without padding
/// ---
/// 326
/// ```
///
/// The other routes' paths and fragments are shorter: `/claim#` and a
/// claim code are 39, `/invite#` and a secret 30, `/recover#` and a secret
/// 31. Anything longer than this limit is not recognised before any of it
/// is copied or decoded, which bounds the work on every input
/// (SEC-TM-032).
const MAX_LINK_LEN: usize = 326;

/// The longest host read, in octets as it is written, a dot at its end
/// included: the longest name the DNS holds.
const MAX_HOST_LEN: usize = 253;

/// The longest label of a host, in octets, which is the DNS's bound too.
const MAX_LABEL_LEN: usize = 63;

/// What an inbound link asks a client to do.
///
/// The set is closed. A route other than [`Route::NotRecognised`] comes
/// only from [`parse_link`], because the server, the secret and the server
/// key inside it have no other constructor.
///
/// A route may hold a [`LinkSecret`], so like one it has no `==`: match on
/// it. Its `Debug` form is written by hand and holds the kind of link and
/// the server's name, never a secret, a code or a key (SEC-IAM-095,
/// SEC-OPS-013).
#[derive(Clone)]
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

impl core::fmt::Debug for Route {
    /// Writes the kind of link and the server it names, and for everything
    /// else the link carried only `..`. No field's own `Debug` form is
    /// asked for besides the server's, so what a code's type would print
    /// cannot get in.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (kind, server) = match self {
            Self::Claim { server, .. } => ("Claim", server),
            Self::Invitation { server, .. } => ("Invitation", server),
            Self::Pairing { server, .. } => ("Pairing", server),
            Self::Recovery { server, .. } => ("Recovery", server),
            Self::NotRecognised => return f.write_str("NotRecognised"),
        };
        f.debug_struct(kind)
            .field("server", server)
            .finish_non_exhaustive()
    }
}

/// The server a link belongs to. Its name is no secret: it is what a
/// confirmation screen shows.
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
/// holder presents once to redeem it (SEC-OPS-013). Its `Debug` form never
/// shows it, so it cannot reach a log through formatting, and it has no
/// `==`, which would compare in variable time and tempt code into checking
/// a guess against it.
#[derive(Clone)]
pub struct LinkSecret([u8; 16]);

impl LinkSecret {
    /// The 16 octets, for the code that presents them to the server that
    /// issued the link. Nothing else should call this: it is the one way
    /// to the octets.
    #[must_use]
    pub const fn bytes(&self) -> [u8; 16] {
        self.0
    }
}

impl core::fmt::Debug for LinkSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("LinkSecret(..)")
    }
}

/// A server's identity key as a pairing link carries it: 32 octets, which
/// this module does not check to be a key. A client that pinned the
/// server's key compares the two. The key is public, which is why, unlike a
/// [`LinkSecret`], it can be compared and printed: a server gives it to
/// every client that asks.
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
/// that another reader could take for part of either. Its host must also
/// be no longer than a name can be.
fn server_and_tail(raw: &str) -> Option<(Server, &str)> {
    let (scheme, rest) = raw.split_once("://")?;
    let (authority, tail) = rest.split_once('/')?;
    let named = match scheme {
        "https" => Link::parse(Untrusted::new(raw))
            .is_ok_and(|link| link.href() == raw && fits_a_name(link.host())),
        "http" => this_machine(authority),
        _ => false,
    };
    named.then(|| (Server(format!("{scheme}://{authority}")), tail))
}

/// Whether `host` is no longer than a name can be: at most
/// [`MAX_HOST_LEN`] octets as it is written, in labels of at most
/// [`MAX_LABEL_LEN`]. The one URL reader puts no bound on a host, and a
/// confirmation screen has to show the server's name whole: a name it had
/// to cut short could be made to show only a part that reads as another
/// server's. An address is far shorter than either bound.
fn fits_a_name(host: &str) -> bool {
    host.len() <= MAX_HOST_LEN && host.split('.').all(|label| label.len() <= MAX_LABEL_LEN)
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
    use core::marker::PhantomData;
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
    /// A pairing code as its server writes it.
    const PAIRING: &str = "WDJB-MJHT";
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

    /// A route with every part copied out in plain values. A route has no
    /// `==`, because the secret it may hold has none (SEC-OPS-013), so
    /// these tests compare all it holds, each part through its own
    /// accessor: the server's origin, a claim code's octets, a pairing
    /// code's text, a secret's octets and a key's octets.
    #[derive(Debug, PartialEq, Eq)]
    enum Seen {
        Claim {
            server: String,
            code: [u8; 16],
        },
        Invitation {
            server: String,
            secret: [u8; 16],
        },
        Pairing {
            server: String,
            code: String,
            key: [u8; 32],
        },
        Recovery {
            server: String,
            secret: [u8; 16],
        },
        NotRecognised,
    }

    /// The route `raw` is read as.
    fn read(raw: &str) -> Route {
        parse_link(Untrusted::new(raw))
    }

    /// Every part of `route`, copied out.
    fn seen_in(route: &Route) -> Seen {
        match route {
            Route::Claim { server, code } => Seen::Claim {
                server: server.origin().to_owned(),
                code: code.bytes(),
            },
            Route::Invitation { server, secret } => Seen::Invitation {
                server: server.origin().to_owned(),
                secret: secret.bytes(),
            },
            Route::Pairing { server, code, key } => Seen::Pairing {
                server: server.origin().to_owned(),
                code: code.text(),
                key: key.bytes(),
            },
            Route::Recovery { server, secret } => Seen::Recovery {
                server: server.origin().to_owned(),
                secret: secret.bytes(),
            },
            Route::NotRecognised => Seen::NotRecognised,
        }
    }

    /// Every part of the route `raw` is read as.
    fn parse(raw: &str) -> Seen {
        seen_in(&read(raw))
    }

    /// A claim link on `origin` that carries [`CODE`], as it is read.
    fn claim_on(origin: &str) -> Seen {
        Seen::Claim {
            server: origin.to_owned(),
            code: CLAIM,
        }
    }

    /// An invitation on `origin` whose secret is [`SECRET`], as it is read.
    fn invitation_on(origin: &str) -> Seen {
        Seen::Invitation {
            server: origin.to_owned(),
            secret: SECRET,
        }
    }

    /// A pairing link on `origin` for [`PAIRING`] and [`KEY`], as it is
    /// read.
    fn pairing_on(origin: &str) -> Seen {
        Seen::Pairing {
            server: origin.to_owned(),
            code: PAIRING.to_owned(),
            key: KEY,
        }
    }

    /// A recovery link on `origin` whose secret is [`SECRET`], as it is
    /// read.
    fn recovery_on(origin: &str) -> Seen {
        Seen::Recovery {
            server: origin.to_owned(),
            secret: SECRET,
        }
    }

    /// URL-safe base64 without padding, from the codec the core shares. The
    /// reader under test takes no part in it.
    fn unpadded(octets: &[u8]) -> String {
        base64::encode(octets, Alphabet::UrlSafe)
    }

    /// The link a server writes for what was `seen`: its origin, its path
    /// and, in the fragment, its code or secret as text. It is written here
    /// on its own, so the reader is checked against it and not against
    /// itself.
    fn written(seen: &Seen) -> String {
        match seen {
            Seen::Claim { server, code } => {
                format!("{server}/claim#{}", crate::otp::claim_code(*code).text())
            }
            Seen::Invitation { server, secret } => {
                format!("{server}/invite#{}", unpadded(secret))
            }
            Seen::Pairing { server, code, key } => {
                format!("{server}/pair#{code}.{}", unpadded(key))
            }
            Seen::Recovery { server, secret } => {
                format!("{server}/recover#{}", unpadded(secret))
            }
            Seen::NotRecognised => String::new(),
        }
    }

    /// Whether acting on what was `seen` would change anything, from what
    /// each route is for: claiming a server, joining one, approving a
    /// browser and enrolling a passkey all do, and text that was not
    /// recognised does not.
    fn changes_state(seen: &Seen) -> bool {
        match seen {
            Seen::Claim { .. }
            | Seen::Invitation { .. }
            | Seen::Pairing { .. }
            | Seen::Recovery { .. } => true,
            Seen::NotRecognised => false,
        }
    }

    /// Asserts that none of `texts` is recognised.
    fn assert_not_recognised(texts: &[&str]) {
        for raw in texts {
            assert_eq!(parse(raw), Seen::NotRecognised, "{raw:?}");
        }
    }

    /// `count` copies of `symbol`.
    fn run_of(symbol: char, count: usize) -> String {
        (0..count).map(|_| symbol).collect()
    }

    /// `text` with its white space taken out, so that a list is found in it
    /// whether it was written on one line or with an element on each.
    fn squeezed(text: &str) -> String {
        text.chars()
            .filter(|symbol| !symbol.is_whitespace())
            .collect()
    }

    /// The ways `octets` could be written into a `Debug` form, without
    /// white space: the list a derive writes, in decimal and with either
    /// hexadecimal flag; the octets as one hexadecimal word, in either
    /// case; and base64 in both alphabets.
    fn renderings(octets: &[u8]) -> [String; 7] {
        let each = |write: fn(&u8) -> String| -> Vec<String> { octets.iter().map(write).collect() };
        [
            each(|octet| format!("{octet}")).join(","),
            each(|octet| format!("{octet:x}")).join(","),
            each(|octet| format!("{octet:X}")).join(","),
            each(|octet| format!("{octet:02x}")).concat(),
            each(|octet| format!("{octet:02X}")).concat(),
            base64::encode(octets, Alphabet::UrlSafe),
            base64::encode(octets, Alphabet::Standard),
        ]
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_claim_link() {
        assert_eq!(
            parse("https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9"),
            Seen::Claim {
                server: "https://music.example".to_owned(),
                code: CLAIM,
            }
        );
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_an_invitation_link() {
        assert_eq!(
            parse("https://music.example/invite#AAECAwQFBgcICQoLDA0ODw"),
            Seen::Invitation {
                server: "https://music.example".to_owned(),
                secret: SECRET,
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
            Seen::Pairing {
                server: "https://music.example:8443".to_owned(),
                code: "WDJB-MJHT".to_owned(),
                key: KEY,
            }
        );
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_a_recovery_link() {
        assert_eq!(
            parse("http://localhost:8096/recover#AAECAwQFBgcICQoLDA0ODw"),
            Seen::Recovery {
                server: "http://localhost:8096".to_owned(),
                secret: SECRET,
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
                Seen::Claim {
                    server: "https://music.example".to_owned(),
                    code: octets,
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
            Seen::Invitation {
                server: "https://music.example".to_owned(),
                secret: [
                    0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB, 0xEF, 0xBE, 0xFB,
                    0xEF, 0xBE, 0xFB,
                ],
            }
        );
        assert_eq!(
            parse("https://music.example/recover#_____________________w"),
            Seen::Recovery {
                server: "https://music.example".to_owned(),
                secret: [0xFF; 16],
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

    /// The one host that is not read as an address bar holds it. An IPv6
    /// address that holds an IPv4 one is written by the one URL reader with
    /// its last four octets dotted, and that spelling is read. A browser
    /// writes the same address in hexadecimal groups, and that spelling is
    /// not, so a link to such a host that was copied from an address bar
    /// fails closed. This pins what is read today.
    ///
    /// Verifies: SEC-CLI-025
    #[test]
    fn reads_an_ipv4_mapped_address_only_with_its_last_four_octets_dotted() {
        for origin in [
            "https://[::ffff:192.0.2.1]",
            "https://[::ffff:192.0.2.1]:8443",
        ] {
            assert_eq!(
                parse(&format!("{origin}/claim#{CODE}")),
                claim_on(origin),
                "{origin}"
            );
        }
        for start in [
            "https://[::ffff:c000:201]",
            "https://[::ffff:c000:201]:8443",
        ] {
            assert_eq!(
                parse(&format!("{start}/claim#{CODE}")),
                Seen::NotRecognised,
                "{start}"
            );
        }
    }

    /// A host is at most 253 octets as it is written, the longest name the
    /// DNS holds, in labels of at most 63. The one URL reader has no such
    /// bound, and a confirmation screen shows the server's name whole: a
    /// name too long to show could be cut down to the part that looks like
    /// another server's. The last host refused here is the one a link of
    /// exactly 512 octets used to be read with.
    ///
    /// Verifies: SEC-CLI-025, SEC-TM-032
    #[test]
    fn reads_no_host_longer_than_a_name_can_be() {
        let label = run_of('a', 63);
        let full = format!("{label}.{label}.{label}.{}", run_of('a', 61));
        assert_eq!(full.len(), 253);
        let full_with_its_root = format!("{label}.{label}.{label}.{}.", run_of('a', 60));
        assert_eq!(full_with_its_root.len(), 253);
        for host in [
            full,
            full_with_its_root,
            format!("{label}.example"),
            format!("music.{label}"),
            format!("music.{label}.example"),
            label.clone(),
        ] {
            assert_eq!(
                parse(&format!("https://{host}/claim#{CODE}")),
                claim_on(&format!("https://{host}")),
                "{host}"
            );
        }
        // Every label of these two is within 63 octets: only the whole is
        // too long.
        let over = format!("{label}.{label}.{label}.{}", run_of('a', 62));
        assert_eq!(over.len(), 254);
        let over_with_its_root = format!("{label}.{label}.{label}.{}.", run_of('a', 61));
        assert_eq!(over_with_its_root.len(), 254);
        // The next four are far within 253 octets: only one label is too
        // long. The last is too long both ways.
        let wide = run_of('a', 64);
        for host in [
            over,
            over_with_its_root,
            format!("{wide}.example"),
            format!("music.{wide}"),
            format!("music.{wide}.example"),
            wide.clone(),
            run_of('a', 465),
        ] {
            assert_eq!(
                parse(&format!("https://{host}/claim#{CODE}")),
                Seen::NotRecognised,
                "{host}"
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
                Seen::NotRecognised,
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
                Seen::NotRecognised,
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
                        Seen::NotRecognised,
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
                    Seen::NotRecognised,
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

    /// Nothing longer than the longest link a server can write is read, and
    /// that link is: 326 octets, a pairing link whose host is as long as a
    /// name can be and whose port has five digits. One octet more makes it
    /// no link, and the limit is looked at before anything is copied or
    /// decoded, which bounds the work on any text.
    ///
    /// Verifies: SEC-TM-032
    #[test]
    fn reads_nothing_longer_than_the_longest_link_a_server_writes() {
        let label = run_of('a', 63);
        let origin = format!("https://{label}.{label}.{label}.{}:65535", run_of('a', 61));
        let longest =
            format!("{origin}/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8");
        assert_eq!(longest.len(), 326);
        assert_eq!(parse(&longest), pairing_on(&origin));
        let longer = format!("{longest}A");
        assert_eq!(longer.len(), 327);
        assert_eq!(parse(&longer), Seen::NotRecognised);
        let far_longer = format!("https://music.example/claim#{}", run_of('0', 65_536));
        assert_eq!(parse(&far_longer), Seen::NotRecognised);
    }

    /// Verifies: SEC-CLI-025
    #[test]
    fn asks_for_a_confirmation_screen_on_every_route_that_changes_something() {
        let cases = [
            (
                "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
                claim_on("https://music.example"),
                true,
            ),
            (
                "https://music.example/invite#AAECAwQFBgcICQoLDA0ODw",
                invitation_on("https://music.example"),
                true,
            ),
            (
                "https://music.example/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
                pairing_on("https://music.example"),
                true,
            ),
            (
                "https://music.example/recover#AAECAwQFBgcICQoLDA0ODw",
                recovery_on("https://music.example"),
                true,
            ),
            ("https://music.example/library", Seen::NotRecognised, false),
        ];
        for (raw, expected, asks) in cases {
            let route = read(raw);
            assert_eq!(seen_in(&route), expected, "{raw}");
            assert_eq!(route.needs_confirmation(), asks, "{raw}");
            assert_eq!(changes_state(&expected), asks, "{raw}");
        }
    }

    #[test]
    fn hands_back_the_parts_of_a_route_as_they_were_written() {
        assert_eq!(
            Server("https://music.example:8443".to_owned()).origin(),
            "https://music.example:8443"
        );
        assert_eq!(LinkSecret(SECRET).bytes(), SECRET);
        assert_eq!(ServerKey(KEY).bytes(), KEY);
    }

    /// The search the next test relies on, against literal texts, and
    /// against what a derive writes for a value that holds octets: on one
    /// line, with an element on each line, and with either hexadecimal
    /// flag.
    #[test]
    fn the_search_for_octets_finds_what_a_derive_writes() {
        assert_eq!(
            renderings(&[0, 10, 255]),
            [
                "0,10,255", "0,a,ff", "0,A,FF", "000aff", "000AFF", "AAr_", "AAr/"
            ]
        );
        assert_eq!(squeezed(" a\tb\n  c "), "abc");
        let held = Some(SECRET);
        let [decimal, lower, upper, ..] = renderings(&SECRET);
        assert!(squeezed(&format!("{held:?}")).contains(&decimal));
        assert!(squeezed(&format!("{held:#?}")).contains(&decimal));
        assert!(squeezed(&format!("{held:x?}")).contains(&lower));
        assert!(squeezed(&format!("{held:X?}")).contains(&upper));
    }

    /// A route's `Debug` form is what a log line, a panic message or a
    /// failed assertion would hold, so nothing a link carried is in it
    /// besides the server's name: no part of the fragment, and no octets of
    /// a code, a secret or a key, however they are written. That holds
    /// whatever the types of the codes print for themselves.
    ///
    /// Verifies: SEC-IAM-095, SEC-OPS-013
    #[test]
    fn the_debug_form_of_a_route_holds_nothing_its_link_carried() {
        let cases: [(&str, &[&[u8]]); 4] = [
            (
                "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
                &[&CLAIM],
            ),
            (
                "https://music.example/invite#AAECAwQFBgcICQoLDA0ODw",
                &[&SECRET],
            ),
            (
                "https://music.example:8443/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
                &[b"WDJBMJHT", &KEY],
            ),
            (
                "http://localhost:8096/recover#AAECAwQFBgcICQoLDA0ODw",
                &[&SECRET],
            ),
        ];
        for (raw, held) in cases {
            let route = read(raw);
            let (_, fragment) = raw.split_once('#').expect("each link has a fragment");
            let mut hidden: Vec<String> = fragment.split('.').map(str::to_owned).collect();
            for octets in held {
                hidden.extend(renderings(octets));
            }
            for shown in [
                format!("{route:?}"),
                format!("{route:#?}"),
                format!("{route:x?}"),
                format!("{route:X?}"),
            ] {
                let text = squeezed(&shown);
                for part in &hidden {
                    assert!(!text.contains(part.as_str()), "{shown:?} holds {part:?}");
                }
            }
        }
    }

    /// What a route's `Debug` form does hold: which kind of link it was and
    /// the server it names, which a confirmation screen shows as well. A
    /// secret on its own shows a fixed word.
    ///
    /// Verifies: SEC-IAM-095, SEC-OPS-013
    #[test]
    fn the_debug_form_of_a_route_is_its_kind_and_its_server() {
        let cases = [
            (
                "https://music.example/claim#01234-56789-ABCDE-FGHJK-MNPQR-S9",
                "Claim { server: Server(\"https://music.example\"), .. }",
                "Claim {\n    server: Server(\n        \"https://music.example\",\n    ),\n    ..\n}",
            ),
            (
                "https://music.example/invite#AAECAwQFBgcICQoLDA0ODw",
                "Invitation { server: Server(\"https://music.example\"), .. }",
                "Invitation {\n    server: Server(\n        \"https://music.example\",\n    ),\n    ..\n}",
            ),
            (
                "https://music.example:8443/pair#WDJB-MJHT.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
                "Pairing { server: Server(\"https://music.example:8443\"), .. }",
                "Pairing {\n    server: Server(\n        \"https://music.example:8443\",\n    ),\n    ..\n}",
            ),
            (
                "http://localhost:8096/recover#AAECAwQFBgcICQoLDA0ODw",
                "Recovery { server: Server(\"http://localhost:8096\"), .. }",
                "Recovery {\n    server: Server(\n        \"http://localhost:8096\",\n    ),\n    ..\n}",
            ),
            (
                "https://music.example/library",
                "NotRecognised",
                "NotRecognised",
            ),
        ];
        for (raw, plain, pretty) in cases {
            let route = read(raw);
            assert_eq!(format!("{route:?}"), plain, "{raw}");
            assert_eq!(format!("{route:#?}"), pretty, "{raw}");
        }
        assert_eq!(format!("{:?}", LinkSecret(SECRET)), "LinkSecret(..)");
        assert_eq!(format!("{:#?}", LinkSecret(SECRET)), "LinkSecret(..)");
    }

    /// A question about a type `T`. Each answer is `false`, from [`Lacks`],
    /// unless `T` has the trait asked about: then the inherent constant of
    /// the same name applies, is found first, and answers `true`.
    struct Probe<T>(PhantomData<T>);

    /// The answers for a type that has none of the three traits.
    trait Lacks {
        const COMPARES: bool = false;
        const DISPLAYS: bool = false;
        const SERIALISES: bool = false;
    }

    impl<T> Lacks for Probe<T> {}

    impl<T: PartialEq> Probe<T> {
        const COMPARES: bool = true;
    }

    impl<T: core::fmt::Display> Probe<T> {
        const DISPLAYS: bool = true;
    }

    impl<T: serde::Serialize> Probe<T> {
        const SERIALISES: bool = true;
    }

    /// A link's secret, and the route that holds one, has no `==`, no
    /// `Display` and no serialised form. The first two rows show that the
    /// probe tells each of the three apart on types that have them.
    ///
    /// Verifies: SEC-OPS-013
    #[test]
    fn a_link_secret_cannot_be_compared_displayed_or_serialised() {
        let answers = [
            (
                Probe::<Vec<u8>>::COMPARES,
                Probe::<Vec<u8>>::DISPLAYS,
                Probe::<Vec<u8>>::SERIALISES,
            ),
            (
                Probe::<std::io::Error>::COMPARES,
                Probe::<std::io::Error>::DISPLAYS,
                Probe::<std::io::Error>::SERIALISES,
            ),
            (
                Probe::<LinkSecret>::COMPARES,
                Probe::<LinkSecret>::DISPLAYS,
                Probe::<LinkSecret>::SERIALISES,
            ),
            (
                Probe::<Route>::COMPARES,
                Probe::<Route>::DISPLAYS,
                Probe::<Route>::SERIALISES,
            ),
        ];
        assert_eq!(
            answers,
            [
                (true, false, true),
                (false, true, false),
                (false, false, false),
                (false, false, false),
            ]
        );
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
        assert_eq!(written(&Seen::NotRecognised), "");
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
            let claim_code = crate::otp::claim_code(claim_octets).text();
            prop_assert_eq!(
                parse(&format!("{origin}/claim#{claim_code}")),
                Seen::Claim { server: origin.clone(), code: claim_octets }
            );
            prop_assert_eq!(
                parse(&format!("{origin}/invite#{}", unpadded(&secret))),
                Seen::Invitation { server: origin.clone(), secret }
            );
            let pairing_code = crate::otp::pairing_code(user).text();
            prop_assert_eq!(
                parse(&format!("{origin}/pair#{pairing_code}.{}", unpadded(&key))),
                Seen::Pairing { server: origin.clone(), code: pairing_code, key }
            );
            prop_assert_eq!(
                parse(&format!("{origin}/recover#{}", unpadded(&secret))),
                Seen::Recovery { server: origin, secret }
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
                prop_assert_eq!(parse(&raw), Seen::NotRecognised, "{:?}", raw);
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
            let (seen, asks) = on_small_stack(move || {
                let route = read(&text);
                (seen_in(&route), route.needs_confirmation())
            });
            prop_assert!(
                written(&seen) == raw || seen == Seen::NotRecognised,
                "{raw:?} gave {seen:?}"
            );
            prop_assert_eq!(asks, changes_state(&seen), "{:?}", raw);
        }
    }
}
