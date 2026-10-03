//! The attestation object a registration returns (`WebAuthn` Level 3
//! section 6.5.4), for attestation format "none" only.
//!
//! The object is a CBOR map with the text keys `fmt`, `attStmt` and
//! `authData`. Format "none" carries an empty attestation statement, so
//! the object is only a wrapper around authenticator data, which for a
//! registration must hold attested credential data. Other keys are
//! ignored; a repeated key is refused by the CBOR reader.

use super::authdata::{self, AttestedCredential, AuthData, Flags};
use super::cbor::{self, AttestationField, Cbor, WebauthnError};
use crate::parse::{Budget, Cursor, Depth, Limits};

/// A registration's attestation object in format "none": its authenticator
/// data, with the attested credential data it must hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation<'a> {
    /// SHA-256 of the relying-party ID, 32 octets.
    pub rp_id_hash: [u8; 32],
    /// The flags octet, decoded.
    pub flags: Flags,
    /// The signature counter.
    pub sign_count: u32,
    /// The new credential.
    pub credential: AttestedCredential<'a>,
    /// Extension CBOR, when flag bit 7 is set.
    pub extensions: Option<Cbor<'a>>,
}

/// Reads an attestation object of format "none" that occupies the whole of
/// `bytes`.
///
/// The parse spends one step per CBOR item of the object and then reads
/// the authenticator data as [`authdata::auth_data`] does. Each step stands
/// for a type octet of its own, or for the authenticator data's fixed
/// prefix, so `n` octets cost at most `n + 1` steps (k = 1, c = 1;
/// SEC-MED-007). Offsets in errors count from the start of the object.
///
/// # Errors
///
/// Returns a typed error when the object is not a CBOR map, when `fmt` is
/// not "none", when `attStmt` is not an empty map, when `authData` is not a
/// byte string or not valid authenticator data, when the authenticator data
/// has no attested credential data, when octets follow the map, or when
/// the parse hits a CBOR, budget or limit fault.
pub fn attestation_object<'a>(
    bytes: &'a [u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<Attestation<'a>, WebauthnError> {
    let mut cursor = Cursor::new(bytes);
    let value = cbor::decode_from(&mut cursor, limits, budget, depth)?;
    cbor::require_empty(&cursor)?;
    let Cbor::Map(entries) = value else {
        return Err(wrong(AttestationField::Object));
    };
    if field(&entries, "fmt") != Some(&Cbor::Text("none")) {
        return Err(wrong(AttestationField::Fmt));
    }
    if field(&entries, "attStmt") != Some(&Cbor::Map(Vec::new())) {
        return Err(wrong(AttestationField::AttStmt));
    }
    let Some(&Cbor::Bytes(body)) = field(&entries, "authData") else {
        return Err(wrong(AttestationField::AuthData));
    };
    let start = offset_within(bytes, body);
    let AuthData {
        rp_id_hash,
        flags,
        sign_count,
        attested,
        extensions,
    } = authdata::auth_data_from(Cursor::at(body, start), limits, budget, depth)?;
    let Some(credential) = attested else {
        return Err(WebauthnError::MissingCredential {
            offset: start.saturating_add(32),
        });
    };
    Ok(Attestation {
        rp_id_hash,
        flags,
        sign_count,
        credential,
        extensions,
    })
}

/// The value of the text key `name` in a decoded map.
fn field<'m, 'a>(entries: &'m [(Cbor<'a>, Cbor<'a>)], name: &str) -> Option<&'m Cbor<'a>> {
    entries
        .iter()
        .find(|(key, _)| *key == Cbor::Text(name))
        .map(|(_, value)| value)
}

/// Where `part`, a slice the CBOR reader borrowed from `whole`, starts
/// within `whole`. The reader returns byte strings as borrowed slices
/// without their offsets, and the authenticator data reports its errors
/// from the start of the whole object.
fn offset_within(whole: &[u8], part: &[u8]) -> u64 {
    let at = part.as_ptr().addr().saturating_sub(whole.as_ptr().addr());
    u64::try_from(at).unwrap_or(u64::MAX)
}

/// The error for `field`, at the start of the object.
const fn wrong(field: AttestationField) -> WebauthnError {
    WebauthnError::Attestation { offset: 0, field }
}

#[cfg(test)]
mod tests {
    use super::super::cose::CoseKey;
    use super::super::test_support::{
        bytes, len, map, negative_arg, on_small_stack, repeated, text, unsigned,
    };
    use super::*;
    use crate::parse::ParseFault;
    use proptest::collection::vec;
    use proptest::prelude::*;

    const RP: [u8; 32] = [0x5A; 32];
    const AAGUID: [u8; 16] = [0xA7; 16];
    const X: [u8; 32] = [0x11; 32];
    const Y: [u8; 32] = [0x22; 32];
    const CRED_ID: &[u8] = &[0xC1, 0xC2];

    fn plenty() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    fn parse(input: &[u8]) -> Result<Attestation<'_>, WebauthnError> {
        attestation_object(
            input,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    fn es256_key() -> Vec<u8> {
        map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), bytes(&X)),
            (negative_arg(2), bytes(&Y)),
        ])
    }

    /// Authenticator data with `flags`, a sign count of 7 and, when bit 6
    /// is set, an ES256 credential.
    fn auth(flags: u8) -> Vec<u8> {
        let mut out = RP.to_vec();
        out.push(flags);
        out.extend_from_slice(&7u32.to_be_bytes());
        if flags & 0x40 != 0 {
            out.extend_from_slice(&AAGUID);
            out.extend_from_slice(&[0x00, 0x02]);
            out.extend_from_slice(CRED_ID);
            out.extend_from_slice(&es256_key());
        }
        out
    }

    /// The object in CTAP2 canonical key order.
    fn object(auth_data: &[u8]) -> Vec<u8> {
        map(&[
            (text("fmt"), text("none")),
            (text("attStmt"), map(&[])),
            (text("authData"), bytes(auth_data)),
        ])
    }

    fn flags(raw: u8, user_verified: bool) -> Flags {
        Flags {
            raw,
            user_present: true,
            user_verified,
            backup_eligible: false,
            backup_state: false,
            attested_credential_data: true,
            extension_data: false,
        }
    }

    fn expected(raw: u8) -> Attestation<'static> {
        Attestation {
            rp_id_hash: RP,
            flags: flags(raw, raw & 0x04 != 0),
            sign_count: 7,
            credential: AttestedCredential {
                aaguid: AAGUID,
                credential_id: CRED_ID,
                public_key: CoseKey::Es256 { x: X, y: Y },
            },
            extensions: None,
        }
    }

    #[test]
    fn reads_a_none_attestation_with_its_credential() {
        let input = object(&auth(0x45));
        assert_eq!(&input[..19], b"\xA3\x63fmt\x64none\x67attStmt\xA0");
        assert_eq!(parse(&input), Ok(expected(0x45)));
    }

    #[test]
    fn reads_keys_in_any_order_and_ignores_unknown_ones() {
        let input = map(&[
            (text("authData"), bytes(&auth(0x41))),
            (unsigned(1), unsigned(2)),
            (text("attStmt"), map(&[])),
            (text("extra"), map(&[(unsigned(0), unsigned(0))])),
            (text("fmt"), text("none")),
        ]);
        assert_eq!(parse(&input), Ok(expected(0x41)));
    }

    #[test]
    fn reports_offsets_inside_the_authenticator_data_from_the_object_start() {
        // Map head, "authData" (9 octets), byte-string head 0x58 0x25: the
        // authenticator data starts at octet 12, so its flags are at 44.
        let input = map(&[
            (text("authData"), bytes(&auth(0x11))),
            (text("fmt"), text("none")),
            (text("attStmt"), map(&[])),
        ]);
        assert_eq!(
            parse(&input),
            Err(WebauthnError::BackupState {
                offset: 44,
                flags: 0x11,
            })
        );
        let short = map(&[
            (text("authData"), bytes(&RP[..30])),
            (text("fmt"), text("none")),
            (text("attStmt"), map(&[])),
        ]);
        assert_eq!(
            parse(&short),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 12,
                needed: 32,
                available: 30,
            }))
        );
    }

    #[test]
    fn reports_empty_authenticator_data_at_the_end_of_the_object() {
        // "fmt", "none", "attStmt", {} take 19 octets with the map head;
        // "authData" takes 9 and the empty byte string's head 1, so the
        // missing authenticator data would start at octet 29.
        let input = object(&[]);
        assert_eq!(input.len(), 29);
        assert_eq!(
            parse(&input),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 29,
                needed: 32,
                available: 0,
            }))
        );
    }

    #[test]
    fn reports_a_bad_credential_key_from_the_object_start() {
        // "fmt", "none", "attStmt", {} take 19 octets with the map head;
        // "authData" and the two-octet byte-string head take 11 more, so
        // the authenticator data starts at 30. Its 37-octet prefix, 16-octet
        // AAGUID, 2-octet length and 2-octet id put the key at 30 + 57.
        let mut head = RP.to_vec();
        head.push(0x41);
        head.extend_from_slice(&7u32.to_be_bytes());
        head.extend_from_slice(&AAGUID);
        head.extend_from_slice(&[0x00, 0x02]);
        head.extend_from_slice(CRED_ID);

        let mut not_map = head.clone();
        not_map.push(0x00);
        assert_eq!(
            parse(&object(&not_map)),
            Err(WebauthnError::NotMap { offset: 87 })
        );

        let mut rsa = head.clone();
        rsa.extend_from_slice(&[0xA2, 0x01, 0x03, 0x03, 0x39, 0x01, 0x00]);
        assert_eq!(
            parse(&object(&rsa)),
            Err(WebauthnError::Algorithm {
                offset: 87,
                kty: Some(3),
                alg: Some(-257),
                crv: None,
            })
        );

        let mut no_x = head;
        no_x.extend_from_slice(&map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(2), bytes(&Y)),
        ]));
        assert_eq!(
            parse(&object(&no_x)),
            Err(WebauthnError::CoseField {
                offset: 87,
                label: -2,
            })
        );
    }

    #[test]
    fn refuses_authenticator_data_without_a_credential() {
        // "fmt", "none", "attStmt", {} take 19 octets with the map head;
        // "authData" and the byte-string head take 11 more, so the flags
        // are at 30 + 32.
        assert_eq!(
            parse(&object(&auth(0x05))),
            Err(WebauthnError::MissingCredential { offset: 62 })
        );
        assert_eq!(WebauthnError::MissingCredential { offset: 62 }.offset(), 62);
    }

    #[test]
    fn refuses_every_format_but_none() {
        for fmt in [
            text("packed"),
            text("None"),
            text(""),
            bytes(b"none"),
            unsigned(0),
        ] {
            let input = map(&[
                (text("fmt"), fmt),
                (text("attStmt"), map(&[])),
                (text("authData"), bytes(&auth(0x45))),
            ]);
            assert_eq!(parse(&input), Err(wrong(AttestationField::Fmt)));
        }
        let missing = map(&[
            (text("attStmt"), map(&[])),
            (text("authData"), bytes(&auth(0x45))),
        ]);
        assert_eq!(parse(&missing), Err(wrong(AttestationField::Fmt)));
    }

    #[test]
    fn refuses_a_statement_that_is_not_an_empty_map() {
        for statement in [
            Some(map(&[(text("sig"), bytes(&[1]))])),
            Some(bytes(&[])),
            Some(head_array()),
            None,
        ] {
            let mut pairs = vec![(text("fmt"), text("none"))];
            if let Some(value) = statement {
                pairs.push((text("attStmt"), value));
            }
            pairs.push((text("authData"), bytes(&auth(0x45))));
            assert_eq!(parse(&map(&pairs)), Err(wrong(AttestationField::AttStmt)));
        }
    }

    fn head_array() -> Vec<u8> {
        vec![0x80]
    }

    #[test]
    fn refuses_authenticator_data_that_is_not_a_byte_string() {
        for auth_data in [Some(text("data")), Some(map(&[])), None] {
            let mut pairs = vec![(text("fmt"), text("none")), (text("attStmt"), map(&[]))];
            if let Some(value) = auth_data {
                pairs.push((text("authData"), value));
            }
            assert_eq!(parse(&map(&pairs)), Err(wrong(AttestationField::AuthData)));
        }
    }

    #[test]
    fn refuses_an_object_that_is_not_a_map() {
        for input in [vec![0x80], vec![0x00], text("none")] {
            assert_eq!(parse(&input), Err(wrong(AttestationField::Object)));
        }
        assert_eq!(wrong(AttestationField::Object).offset(), 0);
    }

    #[test]
    fn refuses_octets_after_the_object_and_repeated_keys() {
        let mut input = object(&auth(0x45));
        let end = len(&input);
        input.push(0xF6);
        assert_eq!(
            parse(&input),
            Err(WebauthnError::Trailing {
                offset: end,
                remaining: 1,
            })
        );
        let repeated = map(&[(text("fmt"), text("none")), (text("fmt"), text("none"))]);
        assert_eq!(
            parse(&repeated),
            Err(WebauthnError::DuplicateKey { offset: 10 })
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_per_item_and_one_for_the_prefix() {
        // Seven items in the object, one for the prefix, eleven in the key.
        let input = object(&auth(0x45));
        let mut budget = Budget::for_input(0, 0, 19);
        assert_eq!(
            attestation_object(&input, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Ok(expected(0x45))
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 7);
        assert_eq!(
            attestation_object(&input, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 30
            }))
        );
    }

    proptest! {
        /// Verifies: SEC-MED-001
        #[test]
        fn never_panics_on_any_input(input in vec(any::<u8>(), 0..192)) {
            on_small_stack(move || {
                let _ = parse(&input);
            });
        }

        /// Verifies: SEC-MED-007
        #[test]
        fn spends_at_most_one_step_per_octet_plus_one(
            input in prop_oneof![
                vec(any::<u8>(), 0..256),
                (any::<u8>(), 0usize..512).prop_map(|(octet, count)| repeated(&[octet], count)),
                (any::<u8>(), vec(any::<u8>(), 0..64)).prop_map(|(raw, tail)| {
                    let mut auth_data = auth(raw | 0x40);
                    auth_data.extend_from_slice(&tail);
                    object(&auth_data)
                }),
            ],
        ) {
            let octets = len(&input);
            let spent = on_small_stack(move || {
                let mut budget = plenty();
                let _ = attestation_object(
                    &input,
                    &Limits::DEFAULT,
                    &mut budget,
                    Depth::CONTAINER_ROOT,
                );
                u64::MAX.checked_sub(budget.remaining()).unwrap()
            });
            prop_assert!(spent <= octets.checked_add(1).unwrap());
        }
    }
}
