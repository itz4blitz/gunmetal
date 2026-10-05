//! The problem renderer: every error is an RFC 9457 problem document whose
//! `type` and `title` come from the closed catalogue in the core, with the
//! request's identifier and nothing else (SEC-API-072, SEC-TM-040).
//!
//! A handler cannot put text of its own into an error: [`ApiError`] holds
//! only a [`ProblemCode`], so no stack trace, path, SQL, host name or
//! version can reach a client through one.

use gunmetal_core::problem::{Describe, ProblemCode};
use serde::Serialize;

/// An error a handler or hook answers with: one code from the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiError(ProblemCode);

impl ApiError {
    /// The error for a catalogue code.
    #[must_use]
    pub const fn new(code: ProblemCode) -> Self {
        Self(code)
    }

    /// The error for any core error, by the problem it describes.
    #[must_use]
    pub fn from_core(error: &impl Describe) -> Self {
        Self(error.problem().code)
    }

    /// The catalogue code.
    #[must_use]
    pub const fn code(self) -> ProblemCode {
        self.0
    }

    /// The HTTP status of the response: the code's own, or 500 for a code
    /// that has none, since such a code should never reach a response.
    #[must_use]
    pub fn status(self) -> u16 {
        self.shown().status().unwrap_or(500)
    }

    /// The code the client is told: this error's own, or `internal_error`
    /// for a code with no HTTP status, which is a failure no request should
    /// meet and so is answered as the generic error (SEC-API-073).
    fn shown(self) -> ProblemCode {
        if self.0.status().is_some() {
            self.0
        } else {
            ProblemCode::InternalError
        }
    }
}

impl From<ProblemCode> for ApiError {
    fn from(code: ProblemCode) -> Self {
        Self(code)
    }
}

/// Identifies one request in the problem body and the access log, so a
/// person reporting an error can point an admin at the server's log line.
///
/// It leaves the server, so the server draws its 128 bits from the CSPRNG
/// (SEC-API-023, WP-047) through [`crate::pipeline::Hooks::request_id`];
/// it is never a counter that would tell a client how busy the server is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u128);

impl core::fmt::Display for RequestId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// The JSON shape of a problem document.
#[derive(Serialize)]
struct Document<'a> {
    #[serde(rename = "type")]
    kind: String,
    title: &'a str,
    status: u16,
    request: String,
}

/// The problem document for an error, as bytes.
#[must_use]
pub fn render(error: ApiError, request: RequestId) -> Vec<u8> {
    let code = error.shown();
    let document = Document {
        kind: format!("urn:gunmetal:problem:{}", code.code()),
        title: code.text(),
        status: error.status(),
        request: request.to_string(),
    };
    // A struct of strings and a number always serialises.
    serde_json::to_vec(&document).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gunmetal_core::audit_event::AuditUnavailable;

    /// Verifies: SEC-API-072
    #[test]
    fn renders_a_catalogue_entry_whole() {
        assert_eq!(
            String::from_utf8(render(
                ApiError::new(ProblemCode::NotFound),
                RequestId(0x2a)
            ))
            .unwrap(),
            r#"{"type":"urn:gunmetal:problem:not_found","title":"We couldn't find that. It may have been removed, or you may not have access to it.","status":404,"request":"0000000000000000000000000000002a"}"#
        );
    }

    /// Verifies: SEC-API-073, SEC-TM-040
    #[test]
    fn renders_a_code_with_no_status_as_the_generic_error() {
        let error = ApiError::new(ProblemCode::WavUnreadable);
        assert_eq!(
            (
                error.code(),
                error.status(),
                String::from_utf8(render(error, RequestId(7))).unwrap()
            ),
            (
                ProblemCode::WavUnreadable,
                500,
                r#"{"type":"urn:gunmetal:problem:internal_error","title":"Something went wrong on the server. Try again later.","status":500,"request":"00000000000000000000000000000007"}"#
                    .to_owned()
            )
        );
    }

    #[test]
    fn converts_codes_and_core_errors() {
        assert_eq!(
            ApiError::from(ProblemCode::InternalError),
            ApiError::new(ProblemCode::InternalError)
        );
        assert_eq!(
            ApiError::from_core(&AuditUnavailable).code(),
            ProblemCode::AuditUnavailable
        );
        assert_eq!(ApiError::new(ProblemCode::UnknownHost).status(), 421);
    }

    #[test]
    fn formats_request_ids_as_thirty_two_hex_digits() {
        assert_eq!(
            RequestId(u128::MAX).to_string(),
            "ffffffffffffffffffffffffffffffff"
        );
        assert_eq!(
            RequestId(0x0123_4567_89ab_cdef_0011_2233_4455_6677).to_string(),
            "0123456789abcdef0011223344556677"
        );
    }
}
