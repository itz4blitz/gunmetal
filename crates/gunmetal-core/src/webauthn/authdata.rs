//! Authenticator data: RP ID hash, flags, sign count and attested
//! credential data (`WebAuthn` Level 3 section 6.1).

use super::cbor::{self, Cbor, WebauthnError};
use super::cose::{self, CoseKey};
use crate::parse::{Budget, Cursor, Depth, Limits};

/// User present (bit 0).
const UP: u8 = 0x01;
/// User verified (bit 2).
const UV: u8 = 0x04;
/// Backup eligible (bit 3).
const BE: u8 = 0x08;
/// Backup state (bit 4).
const BS: u8 = 0x10;
/// Attested credential data (bit 6).
const AT: u8 = 0x40;
/// Extension data (bit 7).
const ED: u8 = 0x80;
/// Largest credential id `WebAuthn` Level 3 section 6.5.1 allows.
const MAX_CREDENTIAL_ID: u16 = 1023;

/// The flags octet of authenticator data, decoded into the bits `WebAuthn`
/// Level 3 section 6.1.3 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "WebAuthn names each flag bit; packing them would hide the fields WP-081 reads"
)]
pub struct Flags {
    /// The flags octet as it was read, including reserved bits.
    pub raw: u8,
    /// Bit 0: user present.
    pub user_present: bool,
    /// Bit 2: user verified.
    pub user_verified: bool,
    /// Bit 3: backup eligible.
    pub backup_eligible: bool,
    /// Bit 4: backup state.
    pub backup_state: bool,
    /// Bit 6: attested credential data follows.
    pub attested_credential_data: bool,
    /// Bit 7: extension data follows.
    pub extension_data: bool,
}

impl Flags {
    fn from_raw(raw: u8) -> Self {
        Self {
            raw,
            user_present: raw & UP != 0,
            user_verified: raw & UV != 0,
            backup_eligible: raw & BE != 0,
            backup_state: raw & BS != 0,
            attested_credential_data: raw & AT != 0,
            extension_data: raw & ED != 0,
        }
    }
}

/// The attested credential data that follows the 37-octet prefix when
/// flag bit 6 is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedCredential<'a> {
    /// The authenticator's AAGUID.
    pub aaguid: [u8; 16],
    /// The credential id, at most 1023 octets.
    pub credential_id: &'a [u8],
    /// The credential public key.
    pub public_key: CoseKey,
}

/// Authenticator data: the RP ID hash, flags, signature counter, and
/// optional attested credential data and extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthData<'a> {
    /// SHA-256 of the relying-party ID, 32 octets.
    pub rp_id_hash: [u8; 32],
    /// The flags octet, decoded.
    pub flags: Flags,
    /// The signature counter.
    pub sign_count: u32,
    /// Attested credential data, when flag bit 6 is set.
    pub attested: Option<AttestedCredential<'a>>,
    /// Extension CBOR, when flag bit 7 is set.
    pub extensions: Option<Cbor<'a>>,
}

/// Reads authenticator data from `bytes`.
///
/// # Errors
///
/// Returns a typed error when the input is shorter than 37 octets, the
/// backup-state bit is set without backup-eligible, attested credential
/// data or extensions are missing or malformed, a credential id is empty
/// or longer than 1023 octets, or the parse hits a CBOR, budget or limit
/// fault.
pub fn auth_data<'a>(
    bytes: &'a [u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<AuthData<'a>, WebauthnError> {
    let mut cursor = Cursor::new(bytes);
    budget.charge(1, cursor.offset())?;
    let rp_id_hash = cursor.array()?;
    let flags_at = cursor.offset();
    let raw = cursor.u8()?;
    let flags = Flags::from_raw(raw);
    let sign_count = cursor.u32_be()?;
    if flags.backup_state && !flags.backup_eligible {
        return Err(WebauthnError::BackupState {
            offset: flags_at,
            flags: raw,
        });
    }
    let attested = if flags.attested_credential_data {
        Some(read_attested(&mut cursor, limits, budget, depth)?)
    } else {
        None
    };
    let extensions = if flags.extension_data {
        Some(read_extensions(&mut cursor, limits, budget, depth)?)
    } else {
        None
    };
    cbor::require_empty(&cursor)?;
    Ok(AuthData {
        rp_id_hash,
        flags,
        sign_count,
        attested,
        extensions,
    })
}

fn read_attested<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<AttestedCredential<'a>, WebauthnError> {
    let aaguid = cursor.array()?;
    let length_at = cursor.offset();
    let length = cursor.u16_be()?;
    if length == 0 || length > MAX_CREDENTIAL_ID {
        return Err(WebauthnError::CredentialId {
            offset: length_at,
            length,
        });
    }
    let credential_id = cursor.take(u64::from(length))?;
    let public_key = cose::cose_key_from(cursor, limits, budget, depth)?;
    Ok(AttestedCredential {
        aaguid,
        credential_id,
        public_key,
    })
}

fn read_extensions<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<Cbor<'a>, WebauthnError> {
    let offset = cursor.offset();
    let value = cbor::decode_from(cursor, limits, budget, depth)?;
    match value {
        Cbor::Map(_) => Ok(value),
        _ => Err(WebauthnError::CoseField { offset, label: 0 }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{LimitKind, ParseFault};
    use proptest::collection::vec;
    use proptest::prelude::*;

    const RP: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        0x1E, 0x1F,
    ];
    const AAGUID: [u8; 16] = [
        0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE,
        0xAF,
    ];
    const X: [u8; 32] = [0x11; 32];
    const Y: [u8; 32] = [0x22; 32];
    const P: [u8; 32] = [0x33; 32];
    const CRED_ID: &[u8] = &[0xAB, 0xCD, 0xEF];

    fn plenty() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    fn parse(bytes: &[u8]) -> Result<AuthData<'_>, WebauthnError> {
        auth_data(
            bytes,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    fn prefix(flags: u8, sign_count: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&RP);
        out.push(flags);
        out.extend_from_slice(&sign_count.to_be_bytes());
        out
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
        } else {
            out.push(lead | 0x1A);
            out.extend_from_slice(&u32::try_from(n).unwrap().to_be_bytes());
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

    fn cbor_bytes(value: &[u8]) -> Vec<u8> {
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

    fn es256_key() -> Vec<u8> {
        map(&[
            (unsigned(1), unsigned(2)),
            (unsigned(3), negative_arg(6)),
            (negative_arg(0), unsigned(1)),
            (negative_arg(1), cbor_bytes(&X)),
            (negative_arg(2), cbor_bytes(&Y)),
        ])
    }

    fn eddsa_key() -> Vec<u8> {
        map(&[
            (unsigned(1), unsigned(1)),
            (unsigned(3), negative_arg(7)),
            (negative_arg(0), unsigned(6)),
            (negative_arg(1), cbor_bytes(&P)),
        ])
    }

    fn attested(id: &[u8], key: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&AAGUID);
        out.extend_from_slice(&u16::try_from(id.len()).expect("id fits u16").to_be_bytes());
        out.extend_from_slice(id);
        out.extend_from_slice(key);
        out
    }

    fn expected_flags(raw: u8) -> Flags {
        Flags {
            raw,
            user_present: raw & 0x01 != 0,
            user_verified: raw & 0x04 != 0,
            backup_eligible: raw & 0x08 != 0,
            backup_state: raw & 0x10 != 0,
            attested_credential_data: raw & 0x40 != 0,
            extension_data: raw & 0x80 != 0,
        }
    }

    /// Verifies: SEC-IAM-018, SEC-HIS-036
    #[test]
    fn reads_the_37_octet_prefix() {
        let bytes = prefix(UP | UV, 0x0102_0304);
        assert_eq!(
            parse(&bytes),
            Ok(AuthData {
                rp_id_hash: RP,
                flags: expected_flags(UP | UV),
                sign_count: 0x0102_0304,
                attested: None,
                extensions: None,
            })
        );
    }

    /// Verifies: SEC-IAM-018, SEC-IAM-020, SEC-IAM-021
    #[test]
    fn decodes_every_flags_combination() {
        let key = es256_key();
        for raw in 0u8..=255 {
            let mut bytes = prefix(raw, 9);
            if raw & AT != 0 {
                bytes.extend_from_slice(&attested(CRED_ID, &key));
            }
            if raw & ED != 0 {
                bytes.push(0xA0);
            }
            let result = parse(&bytes);
            if raw & BS != 0 && raw & BE == 0 {
                assert_eq!(
                    result,
                    Err(WebauthnError::BackupState {
                        offset: 32,
                        flags: raw,
                    }),
                    "flags {raw:#04X}"
                );
                continue;
            }
            let got = result.expect("valid flags parse");
            assert_eq!(got.rp_id_hash, RP, "flags {raw:#04X}");
            assert_eq!(got.flags, expected_flags(raw), "flags {raw:#04X}");
            assert_eq!(got.sign_count, 9, "flags {raw:#04X}");
            assert_eq!(got.attested.is_some(), raw & AT != 0, "flags {raw:#04X}");
            assert_eq!(got.extensions.is_some(), raw & ED != 0, "flags {raw:#04X}");
            if raw & AT != 0 {
                let attested = got.attested.expect("AT set");
                assert_eq!(attested.aaguid, AAGUID);
                assert_eq!(attested.credential_id, CRED_ID);
                assert_eq!(attested.public_key, CoseKey::Es256 { x: X, y: Y });
            }
            if raw & ED != 0 {
                assert_eq!(got.extensions, Some(Cbor::Map(Vec::new())));
            }
        }
    }

    /// Verifies: SEC-IAM-018, SEC-HIS-036
    #[test]
    fn reads_attested_credential_data_with_an_eddsa_key() {
        let mut bytes = prefix(AT, 0);
        bytes.extend_from_slice(&attested(&[0x01], &eddsa_key()));
        let got = parse(&bytes).expect("attested EdDSA");
        assert_eq!(
            got.attested,
            Some(AttestedCredential {
                aaguid: AAGUID,
                credential_id: &[0x01],
                public_key: CoseKey::Eddsa { x: P },
            })
        );
        assert_eq!(got.extensions, None);
    }

    /// Verifies: SEC-IAM-018
    #[test]
    fn reads_attested_credential_data_then_extensions() {
        let mut bytes = prefix(AT | ED, 1);
        bytes.extend_from_slice(&attested(CRED_ID, &es256_key()));
        bytes.extend_from_slice(&map(&[(unsigned(1), unsigned(2))]));
        let got = parse(&bytes).expect("AT and ED");
        assert_eq!(
            got.attested.as_ref().map(|item| item.credential_id),
            Some(CRED_ID)
        );
        assert_eq!(
            got.extensions,
            Some(Cbor::Map(vec![(Cbor::Unsigned(1), Cbor::Unsigned(2))]))
        );
    }

    /// Verifies: SEC-IAM-018
    #[test]
    fn accepts_a_1023_octet_credential_id_and_refuses_zero_or_1024() {
        let key = es256_key();
        let long: Vec<u8> = (0..1023).map(|_| 0xCD).collect();
        let mut ok = prefix(AT, 0);
        ok.extend_from_slice(&attested(&long, &key));
        let got = parse(&ok).expect("1023-octet id");
        assert_eq!(got.attested.expect("AT").credential_id.len(), 1023);

        let mut empty = prefix(AT, 0);
        empty.extend_from_slice(&AAGUID);
        empty.extend_from_slice(&0u16.to_be_bytes());
        assert_eq!(
            parse(&empty),
            Err(WebauthnError::CredentialId {
                offset: 53,
                length: 0,
            })
        );

        let mut too_long = prefix(AT, 0);
        too_long.extend_from_slice(&AAGUID);
        too_long.extend_from_slice(&1024u16.to_be_bytes());
        assert_eq!(
            parse(&too_long),
            Err(WebauthnError::CredentialId {
                offset: 53,
                length: 1024,
            })
        );
    }

    /// Verifies: SEC-IAM-018
    #[test]
    fn refuses_extension_data_that_is_not_a_map() {
        let mut bytes = prefix(ED, 0);
        bytes.push(0x00);
        assert_eq!(
            parse(&bytes),
            Err(WebauthnError::CoseField {
                offset: 37,
                label: 0,
            })
        );
        let mut array = prefix(ED, 0);
        array.push(0x80);
        assert_eq!(
            parse(&array),
            Err(WebauthnError::CoseField {
                offset: 37,
                label: 0,
            })
        );
    }

    /// Verifies: SEC-IAM-018, SEC-HIS-036
    #[test]
    fn refuses_trailing_octets() {
        let mut prefix_only = prefix(UP, 0);
        prefix_only.push(0x00);
        assert_eq!(
            parse(&prefix_only),
            Err(WebauthnError::Trailing {
                offset: 37,
                remaining: 1,
            })
        );
        let mut with_key = prefix(AT, 0);
        with_key.extend_from_slice(&attested(CRED_ID, &es256_key()));
        with_key.push(0xFF);
        // 37-octet prefix, 16-octet AAGUID, 2-octet length, 3-octet id, 77-octet ES256 key.
        assert_eq!(
            parse(&with_key),
            Err(WebauthnError::Trailing {
                offset: 135,
                remaining: 1,
            })
        );
        let mut with_ext = prefix(ED, 0);
        with_ext.push(0xA0);
        with_ext.push(0x01);
        assert_eq!(
            parse(&with_ext),
            Err(WebauthnError::Trailing {
                offset: 38,
                remaining: 1,
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-MED-004
    #[test]
    fn reports_truncation_at_each_boundary() {
        assert_eq!(
            parse(&[]),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 32,
                available: 0,
            }))
        );
        assert_eq!(
            parse(&RP[..31]),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 32,
                available: 31,
            }))
        );
        let mut flags_only = RP.to_vec();
        assert_eq!(
            parse(&flags_only),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 32,
                needed: 1,
                available: 0,
            }))
        );
        flags_only.push(UP);
        assert_eq!(
            parse(&flags_only),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 33,
                needed: 4,
                available: 0,
            }))
        );
        flags_only.extend_from_slice(&[0, 0, 0]);
        assert_eq!(
            parse(&flags_only),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 33,
                needed: 4,
                available: 3,
            }))
        );
        let mut at = prefix(AT, 0);
        assert_eq!(
            parse(&at),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 37,
                needed: 16,
                available: 0,
            }))
        );
        at.extend_from_slice(&AAGUID);
        assert_eq!(
            parse(&at),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 53,
                needed: 2,
                available: 0,
            }))
        );
        at.extend_from_slice(&1u16.to_be_bytes());
        assert_eq!(
            parse(&at),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 55,
                needed: 1,
                available: 0,
            }))
        );
        let ed = prefix(ED, 0);
        assert_eq!(
            parse(&ed),
            Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 37,
                needed: 1,
                available: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_for_the_prefix() {
        let bytes = prefix(UP, 0);
        let mut budget = Budget::for_input(0, 0, 1);
        assert_eq!(
            auth_data(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT)
                .map(|data| data.sign_count),
            Ok(0)
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            auth_data(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))
        );
        let mut with_key = prefix(AT, 0);
        with_key.extend_from_slice(&attested(CRED_ID, &es256_key()));
        let mut budget = Budget::for_input(0, 0, 1);
        assert_eq!(
            auth_data(
                &with_key,
                &Limits::DEFAULT,
                &mut budget,
                Depth::CONTAINER_ROOT
            ),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 58
            }))
        );
    }

    /// Verifies: SEC-IAM-021
    #[test]
    fn keeps_backup_eligible_without_backup_state() {
        let bytes = prefix(BE, 0);
        let got = parse(&bytes).expect("BE without BS");
        assert_eq!(
            (got.flags.backup_eligible, got.flags.backup_state),
            (true, false)
        );
        let both = prefix(BE | BS, 0);
        let got = parse(&both).expect("BE and BS");
        assert_eq!(
            (got.flags.backup_eligible, got.flags.backup_state),
            (true, true)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn refuses_a_cose_map_past_the_children_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::Children, 4)
            .expect("4 is below the ceiling");
        let mut bytes = prefix(AT, 0);
        bytes.extend_from_slice(&attested(CRED_ID, &es256_key()));
        assert_eq!(
            auth_data(&bytes, &limits, &mut plenty(), Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 5,
                max: 4,
                offset: 58,
            }))
        );
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-HIS-036
        #[test]
        fn never_panics_on_any_input(bytes in vec(any::<u8>(), 0..160)) {
            let _ = parse(&bytes);
        }
    }
}
