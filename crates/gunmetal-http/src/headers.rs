//! The response headers every response carries, errors included
//! (SEC-API-053, SEC-API-054, SEC-API-055, SEC-CLI-004, SEC-PRV-018,
//! SEC-PRV-020, SEC-API-038, SEC-API-040).
//!
//! The set is fixed by the kind of body, so a handler cannot add, drop or
//! weaken one: no CORS header and no `Server` header can appear, because
//! nothing here writes one.

use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY,
    STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue};

/// What a response's body is, which decides its type and its policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// No body.
    Empty,
    /// A JSON document.
    Json,
    /// An RFC 9457 problem document.
    Problem,
    /// An HTML page of the web client.
    Html,
}

/// The policy for HTML pages (SEC-API-044).
pub const HTML_CSP: &str = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; \
style-src 'self'; img-src 'self' blob:; media-src 'self' blob:; font-src 'self'; \
connect-src 'self'; worker-src 'self'; manifest-src 'self'; base-uri 'none'; \
form-action 'self'; frame-ancestors 'none'; object-src 'none'; \
require-trusted-types-for 'script'; trusted-types gunmetal-loader";

/// The policy for everything that is not HTML (SEC-API-053).
pub const OTHER_CSP: &str = "default-src 'none'; frame-ancestors 'none'; sandbox";

/// The browser features HTML pages may not use (SEC-API-053).
pub const PERMISSIONS: &str = "camera=(), microphone=(), geolocation=(), payment=(), \
usb=(), serial=(), hid=(), display-capture=()";

/// One year, the shortest `max-age` SEC-API-038 allows.
pub const HSTS: &str = "max-age=31536000";

/// The headers for a response with this kind of body. `hsts` is set for a
/// response served over HTTPS under a host name.
#[must_use]
pub fn security_headers(kind: BodyKind, hsts: bool) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let mut set = |name: HeaderName, value: &'static str| {
        headers.insert(name, HeaderValue::from_static(value));
    };
    set(X_CONTENT_TYPE_OPTIONS, "nosniff");
    set(REFERRER_POLICY, "no-referrer");
    set(
        HeaderName::from_static("cross-origin-resource-policy"),
        "same-origin",
    );
    set(CACHE_CONTROL, "no-store");
    match kind {
        BodyKind::Empty => {}
        BodyKind::Json => set(CONTENT_TYPE, "application/json"),
        BodyKind::Problem => set(CONTENT_TYPE, "application/problem+json"),
        BodyKind::Html => set(CONTENT_TYPE, "text/html; charset=utf-8"),
    }
    if kind == BodyKind::Html {
        set(CONTENT_SECURITY_POLICY, HTML_CSP);
        set(
            HeaderName::from_static("cross-origin-opener-policy"),
            "same-origin",
        );
        set(HeaderName::from_static("permissions-policy"), PERMISSIONS);
    } else {
        set(CONTENT_SECURITY_POLICY, OTHER_CSP);
    }
    if hsts {
        set(STRICT_TRANSPORT_SECURITY, HSTS);
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(headers: &HeaderMap) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_owned()))
            .collect();
        pairs.sort();
        pairs
    }

    fn golden(extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = [
            ("cache-control", "no-store"),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ]
        .iter()
        .chain(extra)
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
        pairs.sort();
        pairs
    }

    const NON_HTML_CSP: (&str, &str) = (
        "content-security-policy",
        "default-src 'none'; frame-ancestors 'none'; sandbox",
    );

    /// Verifies: SEC-API-053, SEC-API-054, SEC-CLI-004, SEC-STD-013
    #[test]
    fn json_and_problem_responses_carry_the_golden_set() {
        assert_eq!(
            pairs(&security_headers(BodyKind::Json, false)),
            golden(&[NON_HTML_CSP, ("content-type", "application/json")])
        );
        assert_eq!(
            pairs(&security_headers(BodyKind::Problem, false)),
            golden(&[NON_HTML_CSP, ("content-type", "application/problem+json")])
        );
        assert_eq!(
            pairs(&security_headers(BodyKind::Empty, false)),
            golden(&[NON_HTML_CSP])
        );
    }

    /// Verifies: SEC-API-053, SEC-PRV-018, SEC-CLI-004
    #[test]
    fn html_responses_carry_the_page_policy() {
        assert_eq!(
            pairs(&security_headers(BodyKind::Html, false)),
            golden(&[
                (
                    "content-security-policy",
                    "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; \
                     img-src 'self' blob:; media-src 'self' blob:; font-src 'self'; \
                     connect-src 'self'; worker-src 'self'; manifest-src 'self'; \
                     base-uri 'none'; form-action 'self'; frame-ancestors 'none'; \
                     object-src 'none'; require-trusted-types-for 'script'; \
                     trusted-types gunmetal-loader"
                ),
                ("content-type", "text/html; charset=utf-8"),
                ("cross-origin-opener-policy", "same-origin"),
                (
                    "permissions-policy",
                    "camera=(), microphone=(), geolocation=(), payment=(), usb=(), \
                     serial=(), hid=(), display-capture=()"
                ),
            ])
        );
    }

    /// Verifies: SEC-API-038
    #[test]
    fn adds_hsts_of_a_year_when_asked() {
        assert_eq!(
            pairs(&security_headers(BodyKind::Empty, true)),
            golden(&[
                NON_HTML_CSP,
                ("strict-transport-security", "max-age=31536000")
            ])
        );
    }
}
