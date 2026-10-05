//! Credential extraction: from the session cookie or the `Authorization`
//! header only, and never more than one (SEC-API-004, SEC-EXT-006,
//! SEC-HIS-041).
//!
//! A request that carries a credential anywhere else is refused, not
//! ignored: a query parameter or path parameter from the reserved list of
//! credential names, a header that other media servers read tokens from, or
//! two credentials at once. The refusal carries nothing of what was sent,
//! so the rejected value cannot reach a log.
//!
//! The credential that is found is a secret (SEC-OPS-013). A [`Token`] has
//! no `==`, no `Display` and no serialised form, and its `Debug` form is a
//! fixed word. It is not the secrets crate's wrapper, which also wipes its
//! value when dropped: this crate does not depend on that crate, so a
//! token's bytes are not wiped here.

use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, COOKIE};

use crate::query::percent_decode;

/// The web client's session cookie (SEC-API-032).
pub const SESSION_COOKIE: &str = "__Host-gm_session";

/// Parameter names that carry a credential, compared without case
/// (SEC-EXT-006).
const RESERVED: [&str; 6] = [
    "access_token",
    "api_key",
    "apikey",
    "password",
    "refresh_token",
    "token",
];

/// Headers that carry a credential in other media servers and tools. A
/// request with any of them is refused rather than authenticated by it.
const FOREIGN_HEADERS: [&str; 7] = [
    "x-access-token",
    "x-api-key",
    "x-auth-token",
    "x-emby-authorization",
    "x-emby-token",
    "x-mediabrowser-token",
    "x-plex-token",
];

/// The credential a request carries, if any. Like the [`Token`] it holds,
/// it has no `==`: match on it.
#[derive(Debug, Clone)]
pub enum Credential {
    /// No credential.
    None,
    /// The value of the session cookie.
    Cookie(Token),
    /// The value of the `Authorization` header, whose scheme the session
    /// layer parses.
    Header(Token),
}

/// A credential's bytes (SEC-OPS-013). Its `Debug` form never shows them,
/// so a token cannot reach a log through formatting, and it has no `==`,
/// which would compare in variable time and tempt code into checking a
/// guess against it.
#[derive(Clone)]
pub struct Token(Vec<u8>);

impl Token {
    /// The credential's bytes, for the session layer to verify, which is
    /// the access hook and lives outside this crate. Nothing else should
    /// call this: it is the one way to the bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl core::fmt::Debug for Token {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Token(..)")
    }
}

/// A request carried a credential somewhere it may not, or carried two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Misplaced;

/// Whether the request's URL carries a credential: a query parameter, or a
/// `name=value` piece of a path segment, whose name is on the reserved list.
/// The pipeline asks before it matches a route, so the answer is 400 on
/// every path, and nothing of the URL is kept.
#[must_use]
pub fn in_url(path: &str, query: &[(String, String)]) -> bool {
    let in_query = query.iter().any(|(name, _)| reserved(name));
    let in_path = path.split('/').flat_map(|s| s.split(';')).any(|piece| {
        piece
            .split_once('=')
            .and_then(|(name, _)| percent_decode(name, false))
            .is_some_and(|name| reserved(&name))
    });
    in_query || in_path
}

/// Finds the request's one credential in its headers.
///
/// # Errors
///
/// [`Misplaced`] for a foreign credential header, two `Authorization`
/// headers, two session cookies, or a cookie and a header together.
pub fn extract(headers: &HeaderMap) -> Result<Credential, Misplaced> {
    if FOREIGN_HEADERS
        .iter()
        .any(|name| headers.contains_key(*name))
    {
        return Err(Misplaced);
    }
    let mut header = headers.get_all(AUTHORIZATION).iter();
    let mut cookies = headers
        .get_all(COOKIE)
        .iter()
        .flat_map(|value| value.as_bytes().split(|byte| *byte == b';'))
        .filter_map(|pair| {
            let (name, value) = split_pair(pair.trim_ascii())?;
            (name == SESSION_COOKIE.as_bytes()).then_some(value)
        });
    match (header.next(), header.next(), cookies.next(), cookies.next()) {
        (None, None, None, None) => Ok(Credential::None),
        (Some(value), None, None, None) => Ok(Credential::Header(Token(value.as_bytes().to_vec()))),
        (None, None, Some(value), None) => Ok(Credential::Cookie(Token(value.to_vec()))),
        _ => Err(Misplaced),
    }
}

/// Whether a parameter name is a reserved credential name.
fn reserved(name: &str) -> bool {
    RESERVED
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
}

/// Splits a cookie pair at its first `=`.
fn split_pair(pair: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut parts = pair.splitn(2, |byte| *byte == b'=');
    match (parts.next(), parts.next()) {
        (Some(name), Some(value)) => Some((name, value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use core::marker::PhantomData;

    fn headers(pairs: &[(&'static str, &[u8])]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(*name, HeaderValue::from_bytes(value).unwrap());
        }
        map
    }

    fn token(bytes: &[u8]) -> Token {
        Token(bytes.to_vec())
    }

    /// What [`extract`] found, with the token's bytes copied out. A token
    /// has no `==` (SEC-OPS-013), so these tests compare its bytes.
    #[derive(Debug, PartialEq, Eq)]
    enum Found {
        None,
        Cookie(Vec<u8>),
        Header(Vec<u8>),
    }

    fn found(pairs: &[(&'static str, &[u8])]) -> Result<Found, Misplaced> {
        extract(&headers(pairs)).map(|credential| match credential {
            Credential::None => Found::None,
            Credential::Cookie(sent) => Found::Cookie(sent.as_bytes().to_vec()),
            Credential::Header(sent) => Found::Header(sent.as_bytes().to_vec()),
        })
    }

    #[test]
    fn finds_no_credential_in_a_plain_request() {
        assert_eq!(found(&[("cookie", b"theme=dark")]), Ok(Found::None));
    }

    #[test]
    fn takes_the_authorization_header() {
        assert_eq!(
            found(&[("authorization", b"Bearer abc")]),
            Ok(Found::Header(b"Bearer abc".to_vec()))
        );
    }

    #[test]
    fn takes_the_session_cookie_among_others() {
        assert_eq!(
            found(&[
                ("cookie", b"theme=dark;  __Host-gm_session=s3cr3t ; lang"),
                ("cookie", b"other=1"),
            ]),
            Ok(Found::Cookie(b"s3cr3t".to_vec()))
        );
    }

    #[test]
    fn ignores_cookies_with_similar_names() {
        assert_eq!(
            found(&[(
                "cookie",
                b"gm_session=a; __host-gm_session=b; __Host-gm_session2=c",
            )]),
            Ok(Found::None)
        );
    }

    /// Verifies: SEC-API-004
    #[test]
    fn refuses_two_credentials() {
        let cases: [&[(&'static str, &[u8])]; 4] = [
            &[
                ("authorization", b"Bearer a"),
                ("authorization", b"Bearer b"),
            ],
            &[("cookie", b"__Host-gm_session=a; __Host-gm_session=b")],
            &[
                ("cookie", b"__Host-gm_session=a"),
                ("cookie", b"__Host-gm_session=b"),
            ],
            &[
                ("authorization", b"Bearer a"),
                ("cookie", b"__Host-gm_session=b"),
            ],
        ];
        for case in cases {
            assert_eq!(found(case), Err(Misplaced));
        }
    }

    /// Verifies: SEC-API-004, SEC-EXT-006, SEC-HIS-041
    #[test]
    fn refuses_reserved_names_in_the_query_whatever_their_case() {
        for name in [
            "token",
            "access_token",
            "api_key",
            "apiKey",
            "APIKEY",
            "password",
            "refresh_token",
        ] {
            let query = [
                ("limit".to_owned(), "5".to_owned()),
                (name.to_owned(), "secret".to_owned()),
            ];
            assert!(in_url("/api/v1/me", &query));
        }
        let plain = [
            ("limit".to_owned(), "token".to_owned()),
            ("tokens".to_owned(), "5".to_owned()),
            ("to".to_owned(), "ken".to_owned()),
        ];
        assert!(!in_url("/api/v1/me", &plain));
    }

    /// Verifies: SEC-API-004
    #[test]
    fn refuses_reserved_names_as_path_parameters() {
        for path in [
            "/api/v1/me;token=abc",
            "/api/v1/token=abc/me",
            "/api/v1/me;a=b;Api%5FKey=abc",
        ] {
            assert!(in_url(path, &[]));
        }
        for path in ["/api/v1/me;limit=5", "/api/v1/token/me", "/api/v1/me;%zz=1"] {
            assert!(!in_url(path, &[]));
        }
    }

    /// Verifies: SEC-API-004
    #[test]
    fn refuses_credentials_in_foreign_headers() {
        for name in [
            "x-access-token",
            "x-api-key",
            "x-auth-token",
            "x-emby-authorization",
            "x-emby-token",
            "x-mediabrowser-token",
            "x-plex-token",
        ] {
            assert_eq!(found(&[(name, b"abc")]), Err(Misplaced));
        }
    }

    #[test]
    fn hides_tokens_from_debug_output() {
        assert_eq!(
            format!("{:?}", Credential::Cookie(token(b"s3cr3t"))),
            "Cookie(Token(..))"
        );
        assert_eq!(token(b"s3cr3t").as_bytes(), b"s3cr3t");
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

    /// A token, and the credential that holds one, has no `==`, no
    /// `Display` and no serialised form. The first two rows show that the
    /// probe tells each of the three apart on types that have them.
    ///
    /// Verifies: SEC-OPS-013
    #[test]
    fn a_token_cannot_be_compared_displayed_or_serialised() {
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
                Probe::<Token>::COMPARES,
                Probe::<Token>::DISPLAYS,
                Probe::<Token>::SERIALISES,
            ),
            (
                Probe::<Credential>::COMPARES,
                Probe::<Credential>::DISPLAYS,
                Probe::<Credential>::SERIALISES,
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
}
