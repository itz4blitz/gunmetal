//! Request-forgery checks for cookie-authenticated requests (SEC-API-033,
//! SEC-API-034, SEC-IAM-040, SEC-CLI-007).
//!
//! A browser attaches the session cookie to requests that other sites
//! start, so a request authenticated by the cookie must prove that the
//! server's own pages sent it:
//!
//! 1. It carries `Gunmetal-Request: 1`, a header no other site can add
//!    without a CORS preflight, which the server never answers. This works
//!    on plain HTTP and on engines that send no fetch metadata.
//! 2. Its `Sec-Fetch-Site`, when present, is neither `cross-site` nor
//!    `same-site`: a sibling subdomain is not the server's origin.
//! 3. If its method is not `GET`, its `Origin`, when present, is one of the
//!    configured origins; when there is no `Origin`, `Sec-Fetch-Site` must
//!    say `same-origin`.
//!
//! A header given twice fails the check.

use axum::http::{HeaderMap, HeaderValue};

use crate::route::Method;

/// The header the web client adds to every request.
pub const REQUEST_HEADER: &str = "gunmetal-request";

/// Whether a cookie-authenticated request passes the checks above.
#[must_use]
pub fn same_origin(headers: &HeaderMap, method: Method, origins: &[String]) -> bool {
    let (Ok(marker), Ok(site), Ok(origin)) = (
        single(headers, REQUEST_HEADER),
        single(headers, "sec-fetch-site"),
        single(headers, "origin"),
    ) else {
        return false;
    };
    if marker != Some(b"1".as_slice()) {
        return false;
    }
    match (method, site, origin) {
        (_, Some(b"cross-site" | b"same-site"), _) => false,
        (Method::Get, _, _) => true,
        (_, _, Some(origin)) => {
            site.is_none_or(|site| site == b"same-origin")
                && origins.iter().any(|allowed| allowed.as_bytes() == origin)
        }
        (_, site, None) => site == Some(b"same-origin".as_slice()),
    }
}

/// A header was given more than once.
struct Repeated;

/// The value of a header given at most once, or `None` when it is absent.
fn single<'h>(headers: &'h HeaderMap, name: &str) -> Result<Option<&'h [u8]>, Repeated> {
    let mut values = headers.get_all(name).iter();
    let first = values.next().map(HeaderValue::as_bytes);
    match values.next() {
        None => Ok(first),
        Some(_) => Err(Repeated),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://music.example.com";

    fn check(method: Method, pairs: &[(&'static str, &'static str)]) -> bool {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, HeaderValue::from_static(value));
        }
        same_origin(&headers, method, &[ORIGIN.to_owned()])
    }

    /// Verifies: SEC-API-033
    #[test]
    fn needs_the_request_header_on_every_method() {
        for method in [
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Patch,
            Method::Delete,
        ] {
            let site = ("sec-fetch-site", "same-origin");
            assert!(check(method, &[site, ("gunmetal-request", "1")]));
            assert!(!check(method, &[site]));
            assert!(!check(method, &[site, ("gunmetal-request", "0")]));
            assert!(!check(method, &[site, ("gunmetal-request", "1 ")]));
            assert!(!check(
                method,
                &[site, ("gunmetal-request", "1"), ("gunmetal-request", "1")]
            ));
        }
    }

    /// Verifies: SEC-API-034, SEC-CLI-007, SEC-IAM-040
    #[test]
    fn refuses_cross_site_and_same_site_fetches() {
        for method in [Method::Get, Method::Post] {
            for site in ["cross-site", "same-site"] {
                let headers = [
                    ("gunmetal-request", "1"),
                    ("sec-fetch-site", site),
                    ("origin", ORIGIN),
                ];
                assert!(!check(method, &headers));
            }
        }
    }

    /// Verifies: SEC-API-034, SEC-CLI-007, SEC-IAM-040
    #[test]
    fn checks_the_origin_of_unsafe_requests() {
        let marker = ("gunmetal-request", "1");
        let cases: [(&[(&'static str, &'static str)], bool); 9] = [
            (&[marker, ("origin", ORIGIN)], true),
            (
                &[
                    marker,
                    ("origin", ORIGIN),
                    ("sec-fetch-site", "same-origin"),
                ],
                true,
            ),
            (&[marker, ("sec-fetch-site", "same-origin")], true),
            (&[marker, ("origin", "https://evil.example.com")], false),
            (
                &[marker, ("origin", "https://sub.music.example.com")],
                false,
            ),
            (&[marker, ("origin", "http://music.example.com")], false),
            (&[marker, ("origin", "null")], false),
            (
                &[marker, ("origin", ORIGIN), ("sec-fetch-site", "none")],
                false,
            ),
            (&[marker], false),
        ];
        for (headers, expected) in cases {
            assert_eq!(check(Method::Delete, headers), expected);
        }
    }

    #[test]
    fn lets_a_get_through_without_fetch_metadata() {
        assert!(check(Method::Get, &[("gunmetal-request", "1")]));
        assert!(check(
            Method::Get,
            &[
                ("gunmetal-request", "1"),
                ("sec-fetch-site", "none"),
                ("origin", "https://evil.example.com")
            ]
        ));
    }

    #[test]
    fn refuses_repeated_fetch_metadata() {
        let marker = ("gunmetal-request", "1");
        let same = ("sec-fetch-site", "same-origin");
        assert!(!check(Method::Get, &[marker, same, same]));
        assert!(!check(
            Method::Post,
            &[marker, same, ("origin", ORIGIN), ("origin", ORIGIN)]
        ));
    }
}
