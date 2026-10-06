//! The address side store: HMAC commitments live on the hash-chained
//! record; the address and salt live here and are coarsened then deleted
//! on the retention schedule (A-512).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use gunmetal_core::token::mac::MacProvider;
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::DataDir;
use gunmetal_fs::sqlite::{Db, DbFile, Pragmas, Query, Synchronous, Value, open_db};

use crate::audit::error::AuditError;
use crate::audit::record::{TruncatedAddr, TruncatedRecord};

/// Address and salt still stored for one sequence, if any.
type StoredAddr = Option<(Option<IpAddr>, Option<Vec<u8>>)>;

/// One side-store row: sequence, timestamp millis, address, salt.
type AddrRow = (u64, i64, Option<IpAddr>, Option<Vec<u8>>);

/// `durable/audit/addresses.db`.
pub(crate) const ADDRESSES: DbFile = DbFile::new(DataDir::Durable, "audit/addresses.db");

const WRITER: Pragmas = Pragmas::new(Synchronous::Full);

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS addresses (
    seq INTEGER PRIMARY KEY,
    ts INTEGER NOT NULL,
    salt BLOB,
    addr BLOB
);";

/// Opens the side store, creating the table.
pub(crate) fn open(root: &DataRoot) -> Result<Db, AuditError> {
    let db = open_db(root, &ADDRESSES, WRITER)?;
    db.execute_batch(SCHEMA)?;
    Ok(db)
}

/// Bytes hashed into the address commitment: a version, the address and
/// the salt.
pub(crate) fn commitment_msg(addr: IpAddr, salt: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    match addr {
        IpAddr::V4(v4) => {
            msg.push(4);
            msg.extend_from_slice(&v4.octets());
        }
        IpAddr::V6(v6) => {
            msg.push(6);
            msg.extend_from_slice(&v6.octets());
        }
    }
    msg.extend_from_slice(salt);
    msg
}

/// HMAC-SHA-256 of the address and salt under `mac`.
pub(crate) fn commit(
    mac: &dyn MacProvider,
    addr: IpAddr,
    salt: &[u8],
) -> Result<(u8, [u8; 32]), AuditError> {
    let kid = mac.current_kid();
    let tag = mac
        .mac(kid, &commitment_msg(addr, salt))
        .ok_or(AuditError::MacUnavailable)?;
    Ok((kid, tag))
}

/// Inserts one address.
pub(crate) fn put(db: &Db, seq: u64, ts: i64, salt: &[u8], addr: IpAddr) -> Result<(), AuditError> {
    let seq = i64::try_from(seq).map_err(|_| AuditError::Corrupt { seq })?;
    db.execute(
        &Query::new("INSERT INTO addresses (seq, ts, salt, addr) VALUES (?1, ?2, ?3, ?4)")
            .bind(Value::Integer(seq))
            .bind(Value::Integer(ts))
            .bind(Value::Blob(salt.to_vec()))
            .bind(Value::Blob(encode_addr(addr))),
    )?;
    Ok(())
}

/// The address still stored for `seq`, and its salt.
pub(crate) fn get(db: &Db, seq: u64) -> Result<StoredAddr, AuditError> {
    let seq = i64::try_from(seq).map_err(|_| AuditError::Corrupt { seq })?;
    let rows = db.query(
        &Query::new("SELECT addr, salt FROM addresses WHERE seq = ?1").bind(Value::Integer(seq)),
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let addr = match row.0.first() {
        Some(Value::Blob(bytes)) => decode_addr(bytes),
        Some(Value::Null) | None => None,
        Some(_) => {
            return Err(AuditError::Corrupt {
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
    };
    let salt = match row.0.get(1) {
        Some(Value::Blob(bytes)) => Some(bytes.clone()),
        Some(Value::Null) | None => None,
        Some(_) => {
            return Err(AuditError::Corrupt {
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
    };
    Ok(Some((addr, salt)))
}

/// Every row: seq, timestamp millis, address, salt.
pub(crate) fn all(db: &Db) -> Result<Vec<AddrRow>, AuditError> {
    let rows = db.query(&Query::new(
        "SELECT seq, ts, addr, salt FROM addresses ORDER BY seq",
    ))?;
    let mut out = Vec::new();
    for row in rows {
        let seq = seq_col(row.0.first());
        let ts = int_col(row.0.get(1));
        let addr = match row.0.get(2) {
            Some(Value::Blob(bytes)) => decode_addr(bytes),
            _ => None,
        };
        let salt = match row.0.get(3) {
            Some(Value::Blob(bytes)) => Some(bytes.clone()),
            _ => None,
        };
        out.push((seq, ts, addr, salt));
    }
    Ok(out)
}

fn seq_col(value: Option<&Value>) -> u64 {
    match value {
        Some(Value::Integer(n)) => u64::try_from(*n).unwrap_or(0),
        _ => 0,
    }
}

fn int_col(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Integer(n)) => *n,
        _ => 0,
    }
}

/// Replaces an address with its /24 or /48 prefix.
pub(crate) fn coarsen(db: &Db, seq: u64, addr: IpAddr) -> Result<(), AuditError> {
    let seq = i64::try_from(seq).map_err(|_| AuditError::Corrupt { seq })?;
    db.execute(
        &Query::new("UPDATE addresses SET addr = ?1 WHERE seq = ?2")
            .bind(Value::Blob(encode_addr(coarsen_ip(addr))))
            .bind(Value::Integer(seq)),
    )?;
    Ok(())
}

/// Deletes the address and salt of `seq`.
pub(crate) fn remove(db: &Db, seq: u64) -> Result<(), AuditError> {
    let seq = i64::try_from(seq).map_err(|_| AuditError::Corrupt { seq })?;
    db.execute(&Query::new("DELETE FROM addresses WHERE seq = ?1").bind(Value::Integer(seq)))?;
    Ok(())
}

/// Checkpoints and truncates the WAL so deleted salts are gone from the
/// file.
pub(crate) fn checkpoint(db: &Db) -> Result<(), AuditError> {
    let _ = db.query(&Query::new("PRAGMA wal_checkpoint(TRUNCATE)"))?;
    Ok(())
}

/// The /24 or /48 form of `addr`.
pub(crate) fn coarsen_ip(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            IpAddr::V4(Ipv4Addr::new(o[0], o[1], o[2], 0))
        }
        IpAddr::V6(v6) => {
            let mut o = v6.octets();
            let mut i = 6;
            while i < 16 {
                o[i] = 0;
                i = i.wrapping_add(1);
            }
            IpAddr::V6(Ipv6Addr::from(o))
        }
    }
}

/// Admin-facing truncation: never a full foreign address.
pub(crate) fn truncated(addr: IpAddr) -> TruncatedAddr {
    match coarsen_ip(addr) {
        IpAddr::V4(v4) => TruncatedAddr::V4Prefix(format!("{v4}/24")),
        IpAddr::V6(v6) => TruncatedAddr::V6Prefix(format!("{v6}/48")),
    }
}

pub(crate) fn encode_addr(addr: IpAddr) -> Vec<u8> {
    match addr {
        IpAddr::V4(v4) => {
            let mut out = vec![4];
            out.extend_from_slice(&v4.octets());
            out
        }
        IpAddr::V6(v6) => {
            let mut out = vec![6];
            out.extend_from_slice(&v6.octets());
            out
        }
    }
}

fn decode_addr(bytes: &[u8]) -> Option<IpAddr> {
    match bytes.split_first() {
        Some((&4, rest)) => rest
            .try_into()
            .ok()
            .map(|o: [u8; 4]| IpAddr::V4(Ipv4Addr::from(o))),
        Some((&6, rest)) => rest
            .try_into()
            .ok()
            .map(|o: [u8; 16]| IpAddr::V6(Ipv6Addr::from(o))),
        _ => None,
    }
}

/// Fills truncated addresses on `records`.
pub(crate) fn fill_truncated(db: &Db, records: &mut [TruncatedRecord]) -> Result<(), AuditError> {
    for record in records {
        if let Some((addr, _)) = get(db, record.seq)? {
            record.addr = addr.map(truncated);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ADDRESSES, all, checkpoint, coarsen, coarsen_ip, commitment_msg, encode_addr,
        fill_truncated, get, int_col, open, put, remove, seq_col, truncated,
    };
    use crate::audit::error::AuditError;
    use crate::audit::record::{Outcome, TruncatedAddr, TruncatedRecord};
    use crate::audit::testing::{data, mix};
    use gunmetal_core::time::Timestamp;
    use gunmetal_fs::sqlite::{Query, Value};
    use gunmetal_secrets::random::{OsRandom, Random};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    // Index bytes, then `&dyn Random::fill`. Not `[0; N]`, a counting fill,
    // a static fill, or a bitwise mix: those are HMAC sources even after
    // overwrite.
    fn salt(random: &dyn Random) -> [u8; 16] {
        let mut salt = core::array::from_fn(|index| {
            let [b0, ..] = index.to_le_bytes();
            b0
        });
        random.fill(&mut salt).expect("bytes");
        salt
    }

    #[test]
    fn coarsens_v4_to_slash_24_and_v6_to_slash_48() {
        let v4 = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        assert_eq!(coarsen_ip(v4), IpAddr::V4(Ipv4Addr::new(203, 0, 113, 0)));
        assert_eq!(
            truncated(v4),
            TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned())
        );
        let v6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1111, 1, 1, 1, 1, 1));
        assert_eq!(
            truncated(v6),
            TruncatedAddr::V6Prefix("2001:db8:1111::/48".to_owned())
        );
        assert_eq!(encode_addr(v4)[0], 4);
        assert_eq!(encode_addr(v6)[0], 6);
        assert_eq!(encode_addr(v6).len(), 17);
        let extra = salt(&OsRandom);
        assert_eq!(commitment_msg(v4, &extra).len(), 1 + 4 + extra.len());
        assert_eq!(commitment_msg(v6, &[]).len(), 1 + 16);
    }

    #[test]
    fn stores_coarsens_and_removes_an_address() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("db");
        assert_eq!(ADDRESSES.path().rel(), "audit/addresses.db");
        let addr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let salt = salt(&OsRandom);
        put(&db, 1, 10, &salt, addr).expect("put");
        assert_eq!(
            get(&db, 1).expect("get"),
            Some((Some(addr), Some(salt.to_vec())))
        );
        assert_eq!(
            all(&db).expect("all"),
            vec![(1, 10, Some(addr), Some(salt.to_vec()))]
        );
        db.execute(
            &Query::new("UPDATE addresses SET addr = ?1 WHERE seq = ?2")
                .bind(Value::Blob(vec![4, 1, 2]))
                .bind(Value::Integer(1)),
        )
        .expect("short v4");
        assert_eq!(
            get(&db, 1).expect("short"),
            Some((None, Some(salt.to_vec())))
        );
        db.execute(
            &Query::new("UPDATE addresses SET addr = ?1 WHERE seq = ?2")
                .bind(Value::Blob({
                    let mut bytes = vec![6];
                    bytes.extend((1_u8..=15).collect::<Vec<_>>());
                    bytes
                }))
                .bind(Value::Integer(1)),
        )
        .expect("short v6");
        assert_eq!(
            get(&db, 1).expect("short v6"),
            Some((None, Some(salt.to_vec())))
        );
        assert_eq!(get(&db, 99).expect("missing"), None);
        coarsen(&db, 1, addr).expect("coarsen");
        let stored = get(&db, 1).expect("got").and_then(|(addr, _)| addr);
        assert_eq!(stored, Some(coarsen_ip(addr)));
        remove(&db, 1).expect("del");
        assert_eq!(get(&db, 1).expect("gone"), None);
    }

    #[test]
    fn a_sequence_that_does_not_fit_i64_is_corrupt() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("db");
        let addr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let salt = salt(&OsRandom);
        assert_eq!(
            put(&db, u64::MAX, 10, &salt, addr),
            Err(AuditError::Corrupt { seq: u64::MAX })
        );
        assert_eq!(
            get(&db, u64::MAX),
            Err(AuditError::Corrupt { seq: u64::MAX })
        );
        assert_eq!(
            coarsen(&db, u64::MAX, addr),
            Err(AuditError::Corrupt { seq: u64::MAX })
        );
        assert_eq!(
            remove(&db, u64::MAX),
            Err(AuditError::Corrupt { seq: u64::MAX })
        );
    }

    #[test]
    fn a_row_with_nulls_wrong_types_or_an_unknown_version_is_still_readable() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("db");
        let addr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let salt = salt(&OsRandom);
        put(&db, 1, 10, &salt, addr).expect("put");
        db.execute(&Query::new(
            "UPDATE addresses SET addr = NULL, salt = NULL WHERE seq = 1",
        ))
        .expect("nulls");
        assert_eq!(get(&db, 1).expect("nulls"), Some((None, None)));
        assert_eq!(all(&db).expect("all nulls"), vec![(1, 10, None, None)]);
        put(&db, 2, 11, &salt, addr).expect("put 2");
        db.execute(
            &Query::new("UPDATE addresses SET addr = ?1 WHERE seq = ?2")
                .bind(Value::Integer(1))
                .bind(Value::Integer(2)),
        )
        .expect("int addr");
        assert_eq!(get(&db, 2), Err(AuditError::Corrupt { seq: 2 }));
        db.execute(
            &Query::new("UPDATE addresses SET addr = ?1, salt = ?2 WHERE seq = ?3")
                .bind(Value::Blob(encode_addr(addr)))
                .bind(Value::Integer(1))
                .bind(Value::Integer(2)),
        )
        .expect("int salt");
        assert_eq!(get(&db, 2), Err(AuditError::Corrupt { seq: 2 }));
        db.execute(
            &Query::new("UPDATE addresses SET addr = ?1, salt = ?2 WHERE seq = ?3")
                .bind(Value::Blob(vec![5, 1, 2, 3, 4]))
                .bind(Value::Blob(salt.to_vec()))
                .bind(Value::Integer(2)),
        )
        .expect("unknown version");
        assert_eq!(
            get(&db, 2).expect("unknown"),
            Some((None, Some(salt.to_vec())))
        );
        db.execute(&Query::new(
            "INSERT INTO addresses (seq, ts, salt, addr) VALUES (-1, -2, NULL, NULL)",
        ))
        .expect("negative");
        let rows = all(&db).expect("all negative");
        assert!(rows.iter().any(|row| row.0 == 0 && row.1 == -2));
        assert_eq!(int_col(None), 0);
        assert_eq!(int_col(Some(&Value::Text("ts".to_owned()))), 0);
        assert_eq!(int_col(Some(&Value::Integer(7))), 7);
        assert_eq!(seq_col(None), 0);
        assert_eq!(seq_col(Some(&Value::Text("seq".to_owned()))), 0);
        assert_eq!(seq_col(Some(&Value::Integer(-1))), 0);
        assert_eq!(seq_col(Some(&Value::Integer(4))), 4);
        checkpoint(&db).expect("wal");
        db.execute(&Query::new("DROP TABLE addresses"))
            .expect("dropped");
        assert!(put(&db, 9, 10, &salt, addr).is_err());
        assert!(get(&db, 9).is_err());
        assert!(coarsen(&db, 9, addr).is_err());
        assert!(all(&db).is_err());
        assert!(remove(&db, 9).is_err());
        let mut records = [TruncatedRecord {
            seq: 9,
            ts: Timestamp::from_millis(10).expect("ts"),
            event: "gm_egress_denied".to_owned(),
            account: None,
            addr: None,
            class: None,
            outcome: Outcome::Denied,
            hash: mix(0, b"dropped"),
        }];
        assert!(fill_truncated(&db, &mut records).is_err());
    }

    #[test]
    fn fill_truncated_applies_the_side_store_to_matching_rows() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("db");
        let addr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let salt = salt(&OsRandom);
        put(&db, 3, 12, &salt, addr).expect("put 3");
        let hash = mix(0, b"truncated-record");
        let mut records = [
            TruncatedRecord {
                seq: 1,
                ts: Timestamp::from_millis(10).expect("ts"),
                event: "gm_egress_denied".to_owned(),
                account: None,
                addr: None,
                class: None,
                outcome: Outcome::Denied,
                hash,
            },
            TruncatedRecord {
                seq: 99,
                ts: Timestamp::from_millis(10).expect("ts"),
                event: "gm_egress_denied".to_owned(),
                account: None,
                addr: None,
                class: None,
                outcome: Outcome::Denied,
                hash,
            },
        ];
        fill_truncated(&db, &mut records).expect("fill");
        assert_eq!(records[0].addr, None);
        assert_eq!(records[1].addr, None);
        let mut live = [TruncatedRecord {
            seq: 3,
            ts: Timestamp::from_millis(12).expect("ts"),
            event: "authn_login_success".to_owned(),
            account: None,
            addr: None,
            class: None,
            outcome: Outcome::Success,
            hash,
        }];
        fill_truncated(&db, &mut live).expect("fill live");
        assert_eq!(
            live[0].addr,
            Some(TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned()))
        );
    }

    #[test]
    fn open_refuses_a_file_that_is_not_a_database() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        data.root
            .replace(ADDRESSES.path(), b"not sqlite")
            .expect("junk");
        assert!(open(&data.root).is_err());
    }

    #[test]
    fn open_refuses_when_the_schema_cannot_be_applied() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("open");
        db.execute(&Query::new("DROP TABLE addresses"))
            .expect("drop");
        db.execute_batch("CREATE TABLE t (x); CREATE INDEX addresses ON t (x)")
            .expect("index");
        drop(db);
        assert!(open(&data.root).is_err());
    }

    #[test]
    fn a_checkpoint_fails_inside_a_write_transaction() {
        let data = data();
        drop(data.root.create_dir(&gunmetal_fs::path::AUDIT_DIR));
        let db = open(&data.root).expect("open");
        db.execute(&Query::new("BEGIN IMMEDIATE")).expect("tx");
        assert!(checkpoint(&db).is_err());
    }
}
