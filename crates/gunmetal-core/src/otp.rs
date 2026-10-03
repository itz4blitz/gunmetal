//! Human-handled one-time codes: claim, recovery and pairing.
//!
//! Each generator takes random bytes the caller supplies from the one CSPRNG
//! function (WP-047). This module only formats and parses; it does not draw
//! randomness, store codes or compare them in constant time.
//!
//! Claim codes carry 128 bits as 26 Crockford base32 symbols plus a checksum,
//! grouped by fives (SEC-IAM-007). Recovery codes carry 80 bits as 16 symbols
//! plus a checksum, grouped by fours (SEC-IAM-089). Pairing user codes are 8
//! characters from the RFC 8628 section 6.1 base-20 alphabet, grouped as
//! `XXXX-XXXX` (SEC-IAM-056).

use crate::problem::{Describe, Problem, ProblemCode};

/// Crockford's base32 symbols, upper case, without `I`, `L`, `O` and `U`.
const CROCKFORD: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Crockford's check alphabet: the 32 payload symbols, then `*~$=U`.
const CHECKSUM: [u8; 37] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ*~$=U";
/// RFC 8628 section 6.1 base-20 alphabet, upper case.
const PAIRING: [u8; 20] = *b"BCDFGHJKLMNPQRSTVWXZ";

/// Compact length of a claim code: 26 payload symbols and one checksum.
const CLAIM_LEN: usize = 27;
/// Compact length of a recovery code: 16 payload symbols and one checksum.
const RECOVERY_LEN: usize = 17;
/// Compact length of a pairing user code.
const PAIRING_LEN: usize = 8;

/// Which kind of one-time code a caller is generating or parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKind {
    /// A setup claim code (SEC-IAM-007).
    Claim,
    /// An account recovery code (SEC-IAM-089).
    Recovery,
    /// A pairing user code (SEC-IAM-056).
    Pairing,
}

/// A parsed one-time code of one of the three kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A setup claim code.
    Claim(ClaimCode),
    /// An account recovery code.
    Recovery(RecoveryCode),
    /// A pairing user code.
    Pairing(PairingCode),
}

/// 128 random bits formatted as a grouped claim code with a checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimCode([u8; 16]);

/// 80 random bits formatted as a grouped recovery code with a checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryCode([u8; 10]);

/// Eight pairing symbols from the RFC 8628 section 6.1 alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingCode([u8; 8]);

/// Why text is not a one-time code of the requested kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeError {
    /// The text is not a code of the requested kind: wrong length, a symbol
    /// outside that kind's alphabet, or more than 128 bits in a claim code.
    Malformed {
        /// The kind the caller asked for.
        kind: CodeKind,
    },
    /// The symbols are in the alphabet and the length is right, but the
    /// checksum does not match, so a typo can be refused without spending an
    /// attempt (SEC-IAM-007).
    Checksum {
        /// The kind the caller asked for.
        kind: CodeKind,
    },
    /// The text has the compact length of a different kind of code.
    WrongKind {
        /// The kind the caller asked for.
        expected: CodeKind,
        /// The kind whose compact length the text has.
        actual: CodeKind,
    },
}

impl ClaimCode {
    /// The 16 random bytes this code was generated from.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }

    /// The canonical grouped text, upper case, with hyphens every five
    /// symbols.
    #[must_use]
    pub fn text(self) -> String {
        group(&claim_symbols(self.0), 5)
    }
}

impl RecoveryCode {
    /// The 10 random bytes this code was generated from.
    #[must_use]
    pub const fn bytes(self) -> [u8; 10] {
        self.0
    }

    /// The canonical grouped text, upper case, with hyphens every four
    /// symbols.
    #[must_use]
    pub fn text(self) -> String {
        group(&recovery_symbols(self.0), 4)
    }
}

impl PairingCode {
    /// The canonical grouped text, upper case, as `XXXX-XXXX`.
    #[must_use]
    pub fn text(self) -> String {
        group(&self.0, 4)
    }
}

impl Code {
    /// The canonical grouped text of this code.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Claim(code) => code.text(),
            Self::Recovery(code) => code.text(),
            Self::Pairing(code) => code.text(),
        }
    }
}

/// Formats 16 random bytes as a claim code (SEC-IAM-007).
#[must_use]
pub const fn claim_code(random: [u8; 16]) -> ClaimCode {
    ClaimCode(random)
}

/// Formats 10 random bytes as a recovery code (SEC-IAM-089).
#[must_use]
pub const fn recovery_code(random: [u8; 10]) -> RecoveryCode {
    RecoveryCode(random)
}

/// Formats 8 random bytes as a pairing user code (SEC-IAM-056).
///
/// Each byte selects one symbol as `alphabet[byte % 20]`, so every input
/// produces 8 symbols from the RFC 8628 section 6.1 alphabet.
#[must_use]
pub fn pairing_code(random: [u8; 8]) -> PairingCode {
    let mut symbols = [0; 8];
    for (slot, byte) in symbols.iter_mut().zip(random) {
        *slot = pairing_symbol(byte);
    }
    PairingCode(symbols)
}

/// Reads a typed code of kind `kind`.
///
/// Hyphens and ASCII whitespace are ignored. Claim and recovery codes are
/// case-insensitive and fold `O` to `0` and `I` and `L` to `1`. Pairing
/// codes are case-insensitive and do not fold: `L` is a pairing symbol.
///
/// # Errors
///
/// Returns a [`CodeError`] when the text is not a well-formed code of
/// `kind`. A wrong checksum is a distinct error so the setup page can catch
/// a typo before an attempt is spent.
pub fn parse_code(text: &str, kind: CodeKind) -> Result<Code, CodeError> {
    let compact = compact(text);
    match kind_for_len(compact.len()) {
        None => Err(CodeError::Malformed { kind }),
        Some(actual) if actual != kind => Err(CodeError::WrongKind {
            expected: kind,
            actual,
        }),
        Some(CodeKind::Claim) => parse_claim(copy_fixed(&compact)).map(Code::Claim),
        Some(CodeKind::Recovery) => parse_recovery(copy_fixed(&compact)).map(Code::Recovery),
        Some(CodeKind::Pairing) => parse_pairing(copy_fixed(&compact)).map(Code::Pairing),
    }
}

impl Describe for CodeError {
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::InvalidCode,
            args: Vec::new(),
        }
    }
}

/// Drops hyphens and ASCII whitespace, keeping every other octet.
fn compact(text: &str) -> Vec<u8> {
    text.bytes()
        .filter(|byte| *byte != b'-' && !byte.is_ascii_whitespace())
        .collect()
}

/// The kind whose compact length is `len`.
const fn kind_for_len(len: usize) -> Option<CodeKind> {
    match len {
        PAIRING_LEN => Some(CodeKind::Pairing),
        RECOVERY_LEN => Some(CodeKind::Recovery),
        CLAIM_LEN => Some(CodeKind::Claim),
        _ => None,
    }
}

/// Copies `src` into an `N`-octet array, ignoring extra octets and leaving
/// unread slots as zero.
fn copy_fixed<const N: usize>(src: &[u8]) -> [u8; N] {
    let mut bytes = [0; N];
    for (slot, &byte) in bytes.iter_mut().zip(src) {
        *slot = byte;
    }
    bytes
}

/// 26 Crockford symbols of `bytes` and the Crockford check symbol.
fn claim_symbols(bytes: [u8; 16]) -> [u8; CLAIM_LEN] {
    let payload = encode_bits::<26>(u128::from_be_bytes(bytes));
    let mut symbols = [0; CLAIM_LEN];
    for (slot, symbol) in symbols.iter_mut().zip(payload) {
        *slot = symbol;
    }
    symbols[26] = checksum_symbol(u128::from_be_bytes(bytes));
    symbols
}

/// 16 Crockford symbols of `bytes` and the Crockford check symbol.
fn recovery_symbols(bytes: [u8; 10]) -> [u8; RECOVERY_LEN] {
    let value = wide_80(bytes);
    let payload = encode_bits::<16>(value);
    let mut symbols = [0; RECOVERY_LEN];
    for (slot, symbol) in symbols.iter_mut().zip(payload) {
        *slot = symbol;
    }
    symbols[16] = checksum_symbol(value);
    symbols
}

/// The 80-bit integer `bytes` write, most significant first.
fn wide_80(bytes: [u8; 10]) -> u128 {
    let mut value = 0_u128;
    for byte in bytes {
        value = value.wrapping_shl(8).wrapping_add(u128::from(byte));
    }
    value
}

/// The low 10 octets of an 80-bit value.
fn low_80(value: u128) -> [u8; 10] {
    let full = value.to_be_bytes();
    let mut bytes = [0; 10];
    for (slot, &byte) in bytes.iter_mut().zip(full.iter().skip(6)) {
        *slot = byte;
    }
    bytes
}

/// Writes `value` as `N` Crockford symbols, most significant first.
fn encode_bits<const N: usize>(mut value: u128) -> [u8; N] {
    let mut symbols = [0; N];
    for symbol in symbols.iter_mut().rev() {
        let [low, ..] = value.to_le_bytes();
        *symbol = symbol_for(low);
        value = value.wrapping_shr(5);
    }
    symbols
}

/// The Crockford payload symbol for the low five bits of `bits`.
#[expect(
    clippy::indexing_slicing,
    reason = "five bits index the 32-symbol alphabet"
)]
fn symbol_for(bits: u8) -> u8 {
    CROCKFORD[usize::from(bits & 0x1F)]
}

/// The Crockford check symbol for `value` modulo 37.
#[expect(
    clippy::indexing_slicing,
    reason = "a residue modulo 37 indexes the 37-symbol check alphabet"
)]
fn checksum_symbol(value: u128) -> u8 {
    let [low, ..] = value.wrapping_rem(37).to_le_bytes();
    CHECKSUM[usize::from(low)]
}

/// The pairing symbol `byte` selects: `alphabet[byte % 20]`.
#[expect(
    clippy::indexing_slicing,
    reason = "a byte modulo 20 indexes the 20-symbol alphabet"
)]
fn pairing_symbol(byte: u8) -> u8 {
    PAIRING[usize::from(byte.wrapping_rem(20))]
}

/// Inserts a hyphen every `width` symbols.
fn group(symbols: &[u8], width: usize) -> String {
    symbols
        .chunks(width)
        .map(|chunk| chunk.iter().copied().map(char::from).collect::<String>())
        .collect::<Vec<String>>()
        .join("-")
}

/// Reads 27 compact claim symbols, after folding, into 16 bytes.
fn parse_claim(compact: [u8; CLAIM_LEN]) -> Result<ClaimCode, CodeError> {
    let folded = copy_fixed::<CLAIM_LEN>(&fold_crockford(&compact));
    let mut payload = [0; 26];
    for (slot, &byte) in payload.iter_mut().zip(&folded) {
        *slot = byte;
    }
    let value = decode_crockford(&payload, CodeKind::Claim)?;
    check_match(value, folded[26], CodeKind::Claim)?;
    Ok(ClaimCode(value.to_be_bytes()))
}

/// Reads 17 compact recovery symbols, after folding, into 10 bytes.
fn parse_recovery(compact: [u8; RECOVERY_LEN]) -> Result<RecoveryCode, CodeError> {
    let folded = copy_fixed::<RECOVERY_LEN>(&fold_crockford(&compact));
    let mut payload = [0; 16];
    for (slot, &byte) in payload.iter_mut().zip(&folded) {
        *slot = byte;
    }
    let value = decode_crockford(&payload, CodeKind::Recovery)?;
    check_match(value, folded[16], CodeKind::Recovery)?;
    Ok(RecoveryCode(low_80(value)))
}

/// Reads 8 compact pairing symbols into a pairing code.
fn parse_pairing(compact: [u8; PAIRING_LEN]) -> Result<PairingCode, CodeError> {
    let mut symbols = [0; PAIRING_LEN];
    for (slot, byte) in symbols.iter_mut().zip(compact) {
        let symbol = byte.to_ascii_uppercase();
        if !PAIRING.contains(&symbol) {
            return Err(CodeError::Malformed {
                kind: CodeKind::Pairing,
            });
        }
        *slot = symbol;
    }
    Ok(PairingCode(symbols))
}

/// Accepts `check` only when it is the Crockford check symbol for `value`.
fn check_match(value: u128, check: u8, kind: CodeKind) -> Result<(), CodeError> {
    if !CHECKSUM.contains(&check) {
        Err(CodeError::Malformed { kind })
    } else if checksum_symbol(value) == check {
        Ok(())
    } else {
        Err(CodeError::Checksum { kind })
    }
}

/// Upper-cases and folds Crockford ambiguous letters in `compact`.
fn fold_crockford(compact: &[u8]) -> Vec<u8> {
    compact
        .iter()
        .map(|byte| match byte {
            b'o' | b'O' => b'0',
            b'i' | b'I' | b'l' | b'L' => b'1',
            _ => byte.to_ascii_uppercase(),
        })
        .collect()
}

/// Reads Crockford payload symbols as one integer.
fn decode_crockford(symbols: &[u8], kind: CodeKind) -> Result<u128, CodeError> {
    let mut value: u128 = 0;
    for &symbol in symbols {
        let digit = (0_u8..)
            .zip(CROCKFORD)
            .find_map(|(digit, candidate)| (candidate == symbol).then_some(digit))
            .ok_or(CodeError::Malformed { kind })?;
        value = value
            .checked_mul(32)
            .ok_or(CodeError::Malformed { kind })?
            .wrapping_add(u128::from(digit));
    }
    Ok(value)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Crockford payload symbols, written out independently of the code
    /// under test.
    const REFERENCE_CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    /// Crockford check symbols, written out independently of the code under
    /// test.
    const REFERENCE_CHECK: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ*~$=U";
    /// RFC 8628 section 6.1 alphabet, written out independently of the code
    /// under test.
    const REFERENCE_PAIRING: &str = "BCDFGHJKLMNPQRSTVWXZ";

    /// Reference encoder: `value` as `count` Crockford symbols, most
    /// significant first, then the check symbol, then hyphens every
    /// `width`.
    fn reference_grouped(value: u128, count: usize, width: usize) -> String {
        let mut rest = value;
        let mut payload = Vec::new();
        for _ in 0..count {
            let digit = usize::try_from(rest % 32).expect("a residue modulo 32 fits");
            payload.push(
                REFERENCE_CROCKFORD
                    .as_bytes()
                    .get(digit)
                    .copied()
                    .expect("32 symbols"),
            );
            rest /= 32;
        }
        payload.reverse();
        let check = REFERENCE_CHECK
            .as_bytes()
            .get(usize::try_from(value % 37).expect("a residue modulo 37 fits"))
            .copied()
            .expect("37 check symbols");
        payload.push(check);
        reference_hyphens(&payload, width)
    }

    /// Hyphens every `width` symbols, written independently of [`group`].
    fn reference_hyphens(symbols: &[u8], width: usize) -> String {
        symbols
            .chunks(width)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|symbol| char::from(*symbol))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("-")
    }

    /// The 128-bit integer `bytes` write.
    fn wide_16(bytes: [u8; 16]) -> u128 {
        u128::from_be_bytes(bytes)
    }

    /// The 80-bit integer `bytes` write.
    fn wide_10(bytes: [u8; 10]) -> u128 {
        bytes
            .iter()
            .fold(0_u128, |value, byte| value * 256 + u128::from(*byte))
    }

    fn malformed(kind: CodeKind) -> CodeError {
        CodeError::Malformed { kind }
    }

    fn checksum(kind: CodeKind) -> CodeError {
        CodeError::Checksum { kind }
    }

    fn wrong(expected: CodeKind, actual: CodeKind) -> CodeError {
        CodeError::WrongKind { expected, actual }
    }

    /// Verifies: SEC-IAM-007
    #[test]
    fn claim_codes_match_literal_encodings() {
        let cases: [([u8; 16], &str); 6] = [
            ([0; 16], "00000-00000-00000-00000-00000-00"),
            ([0xFF; 16], "7ZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-Z*"),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                "00000-00000-00000-00000-00000-11",
            ),
            (
                [0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "40000-00000-00000-00000-00000-0=",
            ),
            (
                [
                    0x01, 0x10, 0xC8, 0x53, 0x1D, 0x09, 0x52, 0xD8, 0xD7, 0x3E, 0x11, 0x94, 0xE9,
                    0x5B, 0x5F, 0x19,
                ],
                "01234-56789-ABCDE-FGHJK-MNPQR-S9",
            ),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34],
                "00000-00000-00000-00000-00001-2$",
            ),
        ];
        for (bytes, text) in cases {
            let code = claim_code(bytes);
            assert_eq!(code.bytes(), bytes);
            assert_eq!(code.text(), text, "{bytes:?}");
            assert_eq!(parse_code(text, CodeKind::Claim), Ok(Code::Claim(code)));
            assert_eq!(
                parse_code(text, CodeKind::Claim).map(Code::text),
                Ok(text.to_owned())
            );
        }
    }

    /// Verifies: SEC-IAM-089
    #[test]
    fn recovery_codes_match_literal_encodings() {
        let cases: [([u8; 10], &str); 5] = [
            ([0; 10], "0000-0000-0000-0000-0"),
            ([0xFF; 10], "ZZZZ-ZZZZ-ZZZZ-ZZZZ-~"),
            ([0, 0, 0, 0, 0, 0, 0, 0, 0, 1], "0000-0000-0000-0001-1"),
            (
                [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x00, 0x11],
                "04HM-ASW9-NF6Y-Y00H-~",
            ),
            ([0, 0, 0, 0, 0, 0, 0, 0, 0, 36], "0000-0000-0000-0014-U"),
        ];
        for (bytes, text) in cases {
            let code = recovery_code(bytes);
            assert_eq!(code.bytes(), bytes);
            assert_eq!(code.text(), text, "{bytes:?}");
            assert_eq!(
                parse_code(text, CodeKind::Recovery),
                Ok(Code::Recovery(code))
            );
        }
    }

    /// Verifies: SEC-IAM-056
    #[test]
    fn pairing_codes_match_literal_encodings() {
        let cases: [([u8; 8], &str); 6] = [
            ([0; 8], "BBBB-BBBB"),
            ([1; 8], "CCCC-CCCC"),
            ([0, 1, 2, 3, 4, 5, 6, 7], "BCDF-GHJK"),
            ([19; 8], "ZZZZ-ZZZZ"),
            ([20; 8], "BBBB-BBBB"),
            ([255; 8], "TTTT-TTTT"),
        ];
        for (bytes, text) in cases {
            let code = pairing_code(bytes);
            assert_eq!(code.text(), text, "{bytes:?}");
            assert_eq!(parse_code(text, CodeKind::Pairing), Ok(Code::Pairing(code)));
        }
        assert_eq!(pairing_code([17, 2, 6, 0, 9, 6, 5, 15]).text(), "WDJB-MJHT");
    }

    /// Verifies: SEC-IAM-056
    #[test]
    fn pairing_codes_use_only_the_base20_alphabet_for_every_input_byte() {
        for byte in 0_u8..=255 {
            let text = pairing_code([byte; 8]).text();
            let compact: String = text.chars().filter(|c| *c != '-').collect();
            assert_eq!(compact.len(), 8, "{byte}");
            assert!(
                compact.chars().all(|c| REFERENCE_PAIRING.contains(c)),
                "{byte}: {text}"
            );
        }
    }

    /// Verifies: SEC-IAM-007
    #[test]
    fn claim_and_recovery_codes_parse_folded_spaces_and_lower_case() {
        let claim = claim_code([0; 16]);
        assert_eq!(
            parse_code("oooo o-oooo o-oooo o-oooo o-oooo o-oo", CodeKind::Claim),
            Ok(Code::Claim(claim))
        );
        let ones = claim_code([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(
            parse_code("00000-00000-00000-00000-00000-ii", CodeKind::Claim),
            Ok(Code::Claim(ones))
        );
        assert_eq!(
            parse_code("00000\t00000\n00000 00000 00000 LL", CodeKind::Claim),
            Ok(Code::Claim(ones))
        );
        let recovery = recovery_code([0; 10]);
        assert_eq!(
            parse_code("oooo-oooo-oooo-oooo-o", CodeKind::Recovery),
            Ok(Code::Recovery(recovery))
        );
        assert_eq!(
            parse_code("00000000000000000", CodeKind::Recovery),
            Ok(Code::Recovery(recovery))
        );
        let pairing = pairing_code([0; 8]);
        assert_eq!(
            parse_code("bbbb bbbb", CodeKind::Pairing),
            Ok(Code::Pairing(pairing))
        );
        assert_eq!(
            parse_code("BBBBBBBB", CodeKind::Pairing),
            Ok(Code::Pairing(pairing))
        );
        assert_eq!(
            parse_code("bbbb-bbbl", CodeKind::Pairing),
            Ok(Code::Pairing(pairing_code([0, 0, 0, 0, 0, 0, 0, 8])))
        );
    }

    /// Verifies: SEC-IAM-007
    #[test]
    fn a_wrong_checksum_is_refused() {
        assert_eq!(
            parse_code("00000-00000-00000-00000-00000-01", CodeKind::Claim),
            Err(checksum(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("00000-00000-00000-00000-00000-0*", CodeKind::Claim),
            Err(checksum(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("0000-0000-0000-0000-*", CodeKind::Recovery),
            Err(checksum(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("7ZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-Z~", CodeKind::Claim),
            Err(checksum(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("0000-0000-0000-0000-1", CodeKind::Recovery),
            Err(checksum(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("ZZZZ-ZZZZ-ZZZZ-ZZZZ-*", CodeKind::Recovery),
            Err(checksum(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("00000-00000-00000-00000-00001-4u", CodeKind::Claim),
            Ok(Code::Claim(claim_code([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36
            ])))
        );
    }

    /// Verifies: SEC-IAM-007, SEC-IAM-056, SEC-IAM-089
    #[test]
    fn a_code_of_the_wrong_kind_is_refused() {
        let claim = claim_code([0; 16]).text();
        let recovery = recovery_code([0; 10]).text();
        let pairing = pairing_code([0; 8]).text();
        assert_eq!(
            parse_code(&claim, CodeKind::Recovery),
            Err(wrong(CodeKind::Recovery, CodeKind::Claim))
        );
        assert_eq!(
            parse_code(&claim, CodeKind::Pairing),
            Err(wrong(CodeKind::Pairing, CodeKind::Claim))
        );
        assert_eq!(
            parse_code(&recovery, CodeKind::Claim),
            Err(wrong(CodeKind::Claim, CodeKind::Recovery))
        );
        assert_eq!(
            parse_code(&recovery, CodeKind::Pairing),
            Err(wrong(CodeKind::Pairing, CodeKind::Recovery))
        );
        assert_eq!(
            parse_code(&pairing, CodeKind::Claim),
            Err(wrong(CodeKind::Claim, CodeKind::Pairing))
        );
        assert_eq!(
            parse_code(&pairing, CodeKind::Recovery),
            Err(wrong(CodeKind::Recovery, CodeKind::Pairing))
        );
    }

    /// Verifies: SEC-IAM-007, SEC-IAM-056, SEC-IAM-089
    #[test]
    fn malformed_text_is_refused() {
        assert_eq!(
            parse_code("", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("", CodeKind::Recovery),
            Err(malformed(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("!!!!!!!!", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("BBBB-BBB0", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("BBBB-BBBO", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("U0000-00000-00000-00000-00000-00", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("*0000-00000-00000-00000-00000-00", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("80000-00000-00000-00000-00000-00", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("00000-00000-00000-00000-00000-0/", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("0000-0000-0000-0000-/", CodeKind::Recovery),
            Err(malformed(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("0", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
        assert_eq!(
            parse_code("0000-0000-0000-0000", CodeKind::Recovery),
            Err(malformed(CodeKind::Recovery))
        );
        assert_eq!(
            parse_code("BBBB-BBB", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("BBBB-BBBBB", CodeKind::Pairing),
            Err(malformed(CodeKind::Pairing))
        );
        assert_eq!(
            parse_code("00000-00000-00000-00000-00000-000", CodeKind::Claim),
            Err(malformed(CodeKind::Claim))
        );
    }

    /// Verifies: SEC-API-072
    #[test]
    fn describes_every_error_as_the_same_invalid_code() {
        let errors = [
            malformed(CodeKind::Claim),
            checksum(CodeKind::Recovery),
            wrong(CodeKind::Pairing, CodeKind::Claim),
        ];
        for error in errors {
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::InvalidCode,
                    args: vec![],
                }
            );
        }
    }

    proptest! {
        /// Verifies: SEC-IAM-007
        #[test]
        fn claim_text_matches_the_reference_encoder(bytes in any::<[u8; 16]>()) {
            prop_assert_eq!(
                claim_code(bytes).text(),
                reference_grouped(wide_16(bytes), 26, 5)
            );
        }

        /// Verifies: SEC-IAM-089
        #[test]
        fn recovery_text_matches_the_reference_encoder(bytes in any::<[u8; 10]>()) {
            prop_assert_eq!(
                recovery_code(bytes).text(),
                reference_grouped(wide_10(bytes), 16, 4)
            );
        }

        /// Verifies: SEC-IAM-056
        #[test]
        fn pairing_text_uses_only_the_written_base20_alphabet(bytes in any::<[u8; 8]>()) {
            let text = pairing_code(bytes).text();
            let compact: String = text.chars().filter(|c| *c != '-').collect();
            prop_assert_eq!(compact.len(), 8);
            prop_assert!(compact.chars().all(|c| REFERENCE_PAIRING.contains(c)));
            prop_assert_eq!(text.chars().filter(|c| *c == '-').count(), 1);
        }

        /// Verifies: SEC-IAM-007, SEC-IAM-056, SEC-IAM-089
        #[test]
        fn every_generated_code_parses_back(
            claim in any::<[u8; 16]>(),
            recovery in any::<[u8; 10]>(),
            pairing in any::<[u8; 8]>(),
        ) {
            let claim = claim_code(claim);
            let recovery = recovery_code(recovery);
            let pairing = pairing_code(pairing);
            prop_assert_eq!(parse_code(&claim.text(), CodeKind::Claim), Ok(Code::Claim(claim)));
            prop_assert_eq!(
                parse_code(&recovery.text(), CodeKind::Recovery),
                Ok(Code::Recovery(recovery))
            );
            prop_assert_eq!(
                parse_code(&pairing.text(), CodeKind::Pairing),
                Ok(Code::Pairing(pairing))
            );
        }

        /// Verifies: SEC-IAM-007, SEC-MED-001
        #[test]
        fn parse_returns_on_any_text(text in "(?s).{0,64}", kind in kind_strategy()) {
            let _ = parse_code(&text, kind);
        }
    }

    fn kind_strategy() -> impl Strategy<Value = CodeKind> {
        prop_oneof![
            Just(CodeKind::Claim),
            Just(CodeKind::Recovery),
            Just(CodeKind::Pairing),
        ]
    }
}
