//! COSE keys for the `WebAuthn` algorithms Gunmetal accepts: `ES256` and `EdDSA`.
//!
//! Owner decision 9 (ADR 0009) keeps RS256 out of R1. A `kty` of 3 or an
//! `alg` of -257 is [`WebauthnError::Algorithm`].

use super::cbor::{self, Cbor, WebauthnError};
use crate::parse::{Budget, Cursor, Depth, Limits};

/// COSE label `kty`.
const LABEL_KTY: i64 = 1;
/// COSE label `alg`.
const LABEL_ALG: i64 = 3;
/// COSE label `crv`.
const LABEL_CRV: i64 = -1;
/// COSE label `x`.
const LABEL_X: i64 = -2;
/// COSE label `y`.
const LABEL_Y: i64 = -3;

/// `kty` 1: octet key pair.
const KTY_OKP: i64 = 1;
/// `kty` 2: elliptic-curve keys with an x and y coordinate.
const KTY_EC2: i64 = 2;
/// COSE algorithm -7: ECDSA with SHA-256.
const ALG_ES256: i64 = -7;
/// COSE algorithm -8: `EdDSA`.
const ALG_EDDSA: i64 = -8;
/// P-256 (`crv` 1).
const CRV_P256: i64 = 1;
/// Ed25519 (`crv` 6).
const CRV_ED25519: i64 = 6;

/// A credential public key in the `COSE_Key` form `WebAuthn` carries inside
/// attested credential data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseKey {
    /// ECDSA over P-256 with SHA-256 (COSE algorithm -7).
    Es256 {
        /// The uncompressed x coordinate, 32 octets.
        x: [u8; 32],
        /// The uncompressed y coordinate, 32 octets.
        y: [u8; 32],
    },
    /// Ed25519 (COSE algorithm -8, curve 6).
    Eddsa {
        /// The public key, 32 octets.
        x: [u8; 32],
    },
}

/// Reads a `COSE_Key` that occupies the whole of `bytes`.
///
/// # Errors
///
/// Returns a typed error when the bytes are not a `COSE_Key` map for `ES256`
/// or `EdDSA`, when they trail after the map, or when the parse hits a
/// CBOR, budget or limit fault.
pub fn cose_key(
    bytes: &[u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<CoseKey, WebauthnError> {
    let mut cursor = Cursor::new(bytes);
    let key = cose_key_from(&mut cursor, limits, budget, depth)?;
    cbor::require_empty(&cursor)?;
    Ok(key)
}

/// Reads a `COSE_Key` from `cursor` and leaves any following octets unread,
/// so attested credential data can be followed by extension CBOR.
pub(super) fn cose_key_from(
    cursor: &mut Cursor<'_>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<CoseKey, WebauthnError> {
    let offset = cursor.offset();
    let value = cbor::decode_from(cursor, limits, budget, depth)?;
    let Cbor::Map(entries) = value else {
        return Err(WebauthnError::CoseField { offset, label: 0 });
    };
    parse_map(offset, &entries)
}

/// Interprets a decoded CBOR map as an `ES256` or `EdDSA` `COSE_Key`.
fn parse_map(offset: u64, entries: &[(Cbor<'_>, Cbor<'_>)]) -> Result<CoseKey, WebauthnError> {
    let kty = required_int(offset, entries, LABEL_KTY)?;
    let alg = required_int(offset, entries, LABEL_ALG)?;
    match (kty, alg) {
        (KTY_EC2, ALG_ES256) => {
            let crv = required_int(offset, entries, LABEL_CRV)?;
            if crv != CRV_P256 {
                return Err(algorithm(offset, kty, alg, Some(crv)));
            }
            let x = required_coord(offset, entries, LABEL_X)?;
            let y = required_coord(offset, entries, LABEL_Y)?;
            Ok(CoseKey::Es256 { x, y })
        }
        (KTY_OKP, ALG_EDDSA) => {
            let crv = required_int(offset, entries, LABEL_CRV)?;
            if crv != CRV_ED25519 {
                return Err(algorithm(offset, kty, alg, Some(crv)));
            }
            let x = required_coord(offset, entries, LABEL_X)?;
            Ok(CoseKey::Eddsa { x })
        }
        _ => Err(algorithm(offset, kty, alg, field_int(entries, LABEL_CRV))),
    }
}

fn algorithm(offset: u64, kty: i64, alg: i64, crv: Option<i64>) -> WebauthnError {
    WebauthnError::Algorithm {
        offset,
        kty: Some(kty),
        alg: Some(alg),
        crv,
    }
}

fn field<'a>(entries: &'a [(Cbor<'a>, Cbor<'a>)], label: i64) -> Option<&'a Cbor<'a>> {
    entries
        .iter()
        .find_map(|(key, value)| cbor::integer(key).filter(|n| *n == label).map(|_| value))
}

fn field_int(entries: &[(Cbor<'_>, Cbor<'_>)], label: i64) -> Option<i64> {
    field(entries, label).and_then(cbor::integer)
}

fn required_int(
    offset: u64,
    entries: &[(Cbor<'_>, Cbor<'_>)],
    label: i64,
) -> Result<i64, WebauthnError> {
    field_int(entries, label).ok_or(WebauthnError::CoseField { offset, label })
}

fn required_coord(
    offset: u64,
    entries: &[(Cbor<'_>, Cbor<'_>)],
    label: i64,
) -> Result<[u8; 32], WebauthnError> {
    match field(entries, label) {
        Some(Cbor::Bytes(bytes)) => {
            <[u8; 32]>::try_from(*bytes).map_err(|_| WebauthnError::CoseField { offset, label })
        }
        _ => Err(WebauthnError::CoseField { offset, label }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::ParseFault;
    use proptest::collection::vec;
    use proptest::prelude::*;

    const X: [u8; 32] = [0x11; 32];
    const Y: [u8; 32] = [0x22; 32];
    const P: [u8; 32] = [0x33; 32];

    fn plenty() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    fn parse(bytes: &[u8]) -> Result<CoseKey, WebauthnError> {
        cose_key(
            bytes,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    fn write_head(out: &mut Vec<u8>, major: u8, n: u64) {
        let lead = major.checked_mul(0x20).unwrap_or(0);
        if n < 24 {
            out.push(lead | u8::try_from(n).unwrap());
        } else if let Ok(byte) = u8::try_from(n) {
            out.push(lead | 0x18);
            out.push(byte);
        } else if let Ok(wide) = u16::try_from(n) {
            out.push(lead | 0x19);
            out.extend_from_slice(&wide.to_be_bytes());
        } else if let Ok(wide) = u32::try_from(n) {
            out.push(lead | 0x1A);
            out.extend_from_slice(&wide.to_be_bytes());
        } else {
            out.push(lead | 0x1B);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }

    fn unsigned(n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        write_head(&mut out, 0, n);
        out
    }

    fn negative_arg(n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        write_head(&mut out, 1, n);
        out
    }

    fn bytes(value: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_head(&mut out, 2, u64::try_from(value.len()).unwrap());
        out.extend_from_slice(value);
        out
    }

    fn map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        write_head(&mut out, 5, u64::try_from(pairs.len()).unwrap());
        for (key, value) in pairs {
            out.extend_from_slice(key);
            out.extend_from_slice(value);
        }
        out
    }

    fn es256(x: [u8; 32], y: [u8; 32]) -> Vec<u8> {
        map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&x)),
            (negative_arg(2), bytes(&y)),
        ])
    }

    fn eddsa(x: [u8; 32]) -> Vec<u8> {
        map(&[
            (unsigned(1), unsigned(1)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(6)),
            (negative_arg(1), bytes(&x)),
        ])
    }

    /// Verifies: SEC-MED-001, SEC-HIS-036
    #[test]
    fn reads_an_es256_key() {
        let bytes = es256(X, Y);
        assert_eq!(
            bytes,
            [
                0xA5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20, 0x11, 0x11, 0x11, 0x11,
                0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
                0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
                0x22, 0x58, 0x20, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
                0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
                0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
            ]
        );
        assert_eq!(parse(&bytes), Ok(CoseKey::Es256 { x: X, y: Y }));
    }

    /// Verifies: SEC-MED-001, SEC-HIS-036
    #[test]
    fn reads_an_eddsa_key() {
        let bytes = eddsa(P);
        assert_eq!(
            bytes,
            [
                0xA4, 0x01, 0x01, 0x03, 0x27, 0x20, 0x06, 0x21, 0x58, 0x20, 0x33, 0x33, 0x33, 0x33,
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
            ]
        );
        assert_eq!(parse(&bytes), Ok(CoseKey::Eddsa { x: P }));
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn accepts_labels_in_any_order_and_ignores_unknown_ones() {
        let shuffled = map(&[
            (negative_arg(2), bytes(&Y)),
            (unsigned(3), negative_arg(6)),
            (unsigned(4), unsigned(0)),
            (unsigned(1), unsigned(2)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(0), unsigned(1)),
        ]);
        assert_eq!(parse(&shuffled), Ok(CoseKey::Es256 { x: X, y: Y }));
        let with_y = map(&[
            (unsigned(1), unsigned(1)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(6)),
            (negative_arg(1), bytes(&P)),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(parse(&with_y), Ok(CoseKey::Eddsa { x: P }));
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_integer_labels_and_values_at_every_width() {
        let encoded = map(&[
            (vec![0x18, 0x01], vec![0x18, 0x02]),
            (vec![0x18, 0x03], vec![0x38, 0x06]),
            (vec![0x38, 0x00], vec![0x18, 0x01]),
            (vec![0x38, 0x01], bytes(&X)),
            (vec![0x38, 0x02], bytes(&Y)),
        ]);
        assert_eq!(parse(&encoded), Ok(CoseKey::Es256 { x: X, y: Y }));
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn refuses_rs256_and_every_other_algorithm() {
        let rs256 = map(&[(unsigned(1), unsigned(3)), (unsigned(3), negative_arg(256))]);
        assert_eq!(
            parse(&rs256),
            Err(WebauthnError::Algorithm {
                offset: 0,
                kty: Some(3),
                alg: Some(-257),
                crv: None,
            })
        );
        let es256_wrong_curve = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(2)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(
            parse(&es256_wrong_curve),
            Err(WebauthnError::Algorithm {
                offset: 0,
                kty: Some(2),
                alg: Some(-7),
                crv: Some(2),
            })
        );
        let eddsa_wrong_curve = map(&[
            (unsigned(1), unsigned(1)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&P)),
        ]);
        assert_eq!(
            parse(&eddsa_wrong_curve),
            Err(WebauthnError::Algorithm {
                offset: 0,
                kty: Some(1),
                alg: Some(-8),
                crv: Some(1),
            })
        );
        let mixed = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(
            parse(&mixed),
            Err(WebauthnError::Algorithm {
                offset: 0,
                kty: Some(2),
                alg: Some(-8),
                crv: Some(1),
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[expect(
        clippy::too_many_lines,
        reason = "each missing or malformed field is an independent oracle"
    )]
    #[test]
    fn refuses_a_missing_wrong_type_or_wrong_length_field() {
        assert_eq!(
            parse(&map(&[])),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: 1,
            })
        );
        let no_alg = map(&[(unsigned(1), unsigned(2))]);
        assert_eq!(
            parse(&no_alg),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: 3,
            })
        );
        let kty_as_text = map(&[(unsigned(1), {
            let mut out = Vec::new();
            write_head(&mut out, 3, 2);
            out.extend_from_slice(b"ec");
            out
        })]);
        assert_eq!(
            parse(&kty_as_text),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: 1,
            })
        );
        let no_crv = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(
            parse(&no_crv),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -1,
            })
        );
        let no_x = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(
            parse(&no_x),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -2,
            })
        );
        let no_y = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&X)),
        ]);
        assert_eq!(
            parse(&no_y),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -3,
            })
        );
        let short_x = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&[0x11; 31])),
            (negative_arg(2), bytes(&Y)),
        ]);
        assert_eq!(
            parse(&short_x),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -2,
            })
        );
        let long_y = map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(2), bytes(&[0x22; 33])),
        ]);
        assert_eq!(
            parse(&long_y),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -3,
            })
        );
        let x_as_text = map(&[
            (unsigned(1), unsigned(1)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(6)),
            (negative_arg(1), {
                let mut out = Vec::new();
                write_head(&mut out, 3, 32);
                out.extend_from_slice(&[b'a'; 32]);
                out
            }),
        ]);
        assert_eq!(
            parse(&x_as_text),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: -2,
            })
        );
        assert_eq!(
            parse(&[0x00]),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: 0,
            })
        );
        assert_eq!(
            parse(&[0x80]),
            Err(WebauthnError::CoseField {
                offset: 0,
                label: 0,
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-HIS-036
    #[test]
    fn refuses_octets_after_the_key() {
        let mut bytes = es256(X, Y);
        bytes.push(0x00);
        assert_eq!(
            parse(&bytes),
            Err(WebauthnError::Trailing {
                offset: 77,
                remaining: 1,
            })
        );
        let mut two = eddsa(P);
        let key_len = u64::try_from(two.len()).unwrap();
        two.extend_from_slice(&[0x01, 0x02]);
        assert_eq!(
            parse(&two),
            Err(WebauthnError::Trailing {
                offset: key_len,
                remaining: 2,
            })
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_per_cbor_item() {
        let bytes = es256(X, Y);
        let mut budget = Budget::for_input(0, 0, 11);
        assert_eq!(
            cose_key(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Ok(CoseKey::Es256 { x: X, y: Y })
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 10);
        assert_eq!(
            cose_key(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 43
            }))
        );
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            cose_key(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_map_against_container_depth() {
        let none = Limits::DEFAULT
            .with_override(crate::parse::LimitKind::ContainerDepth, 0)
            .expect("0 is below the ceiling");
        assert_eq!(
            cose_key(&es256(X, Y), &none, &mut plenty(), Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::TooDeep {
                limit: crate::parse::LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn leaves_following_octets_when_read_from_a_cursor() {
        let mut bytes = es256(X, Y);
        bytes.push(0xA0);
        let mut cursor = Cursor::new(&bytes);
        assert_eq!(
            cose_key_from(
                &mut cursor,
                &Limits::DEFAULT,
                &mut plenty(),
                Depth::CONTAINER_ROOT
            ),
            Ok(CoseKey::Es256 { x: X, y: Y })
        );
        assert_eq!(cursor.rest(), &[0xA0]);
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-HIS-036
        #[test]
        fn never_panics_on_any_input(bytes in vec(any::<u8>(), 0..96)) {
            let _ = parse(&bytes);
        }
    }
}
