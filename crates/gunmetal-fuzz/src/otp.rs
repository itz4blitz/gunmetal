//! The harness for the one-time code parsers in `gunmetal_core::otp`
//! (SEC-IAM-007, SEC-IAM-056, SEC-IAM-089).

use gunmetal_core::otp::{
    Code, CodeError, CodeKind, claim_code, pairing_code, parse_code, recovery_code,
};

/// What [`parse_code`] reported for one input, for each kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`parse_code`] for a claim code, as its canonical grouped text.
    pub claim: Result<String, CodeError>,
    /// [`parse_code`] for a recovery code, as its canonical grouped text.
    pub recovery: Result<String, CodeError>,
    /// [`parse_code`] for a pairing code, as its canonical grouped text.
    pub pairing: Result<String, CodeError>,
}

/// Feeds `data` to every one-time-code entry point.
///
/// # Panics
///
/// Panics when an accepted code does not read back from its canonical text,
/// a generated pairing code uses a symbol outside the RFC 8628 section 6.1
/// alphabet, or a generated claim or recovery code fails to parse.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let mut claim_bytes = [0; 16];
    let mut recovery_bytes = [0; 10];
    let mut pairing_bytes = [0; 8];
    for (slot, &byte) in claim_bytes.iter_mut().zip(data) {
        *slot = byte;
    }
    for (slot, &byte) in recovery_bytes.iter_mut().zip(data) {
        *slot = byte;
    }
    for (slot, &byte) in pairing_bytes.iter_mut().zip(data) {
        *slot = byte;
    }
    let generated_claim = claim_code(claim_bytes);
    let generated_recovery = recovery_code(recovery_bytes);
    let generated_pairing = pairing_code(pairing_bytes);
    assert!(
        parse_code(&generated_claim.text(), CodeKind::Claim) == Ok(Code::Claim(generated_claim))
            && parse_code(&generated_recovery.text(), CodeKind::Recovery)
                == Ok(Code::Recovery(generated_recovery))
            && parse_code(&generated_pairing.text(), CodeKind::Pairing)
                == Ok(Code::Pairing(generated_pairing))
            && generated_pairing
                .text()
                .bytes()
                .filter(|byte| *byte != b'-')
                .all(|byte| b"BCDFGHJKLMNPQRSTVWXZ".contains(&byte)),
        "generated codes must parse and pairing must stay in the base-20 alphabet"
    );

    let text = String::from_utf8_lossy(data);
    let claim = parse_code(&text, CodeKind::Claim).map(Code::text);
    let recovery = parse_code(&text, CodeKind::Recovery).map(Code::text);
    let pairing = parse_code(&text, CodeKind::Pairing).map(Code::text);
    assert!(
        claim.as_ref().ok().is_none_or(|written| {
            parse_code(written, CodeKind::Claim).map(Code::text) == Ok(written.clone())
        }) && recovery.as_ref().ok().is_none_or(|written| {
            parse_code(written, CodeKind::Recovery).map(Code::text) == Ok(written.clone())
        }) && pairing.as_ref().ok().is_none_or(|written| {
            parse_code(written, CodeKind::Pairing).map(Code::text) == Ok(written.clone())
        }),
        "{text:?} gave {claim:?}, {recovery:?}, {pairing:?}"
    );
    Outcome {
        claim,
        recovery,
        pairing,
    }
}
