//! The session layer's statements and the rows they return.
//!
//! Every statement is static text with bound values (SEC-API-066). A row
//! that does not read back as what the statement writes is not trusted:
//! the readers here answer `None`, and the caller refuses.

use std::collections::HashMap;

use gunmetal_core::authz::{DeviceClass, SessionHandle};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_fs::sqlite::{Query, Row, Value};

use super::lifetime::Lifetime;

/// Enrols a device: its identifier, account, class and the time.
pub(super) const ENROL: Query =
    Query::new("INSERT INTO devices (device, account, class, enrolled) VALUES (?1, ?2, ?3, ?4)");

/// Every device's revocation epoch.
pub(super) const EPOCHS: Query = Query::new("SELECT device, epoch FROM devices");

/// Whether a device enrolled for an account is of the limited class.
pub(super) const DEVICE: Query =
    Query::new("SELECT class = 'limited' FROM devices WHERE device = ?1 AND account = ?2");

/// Moves one device's epoch on.
pub(super) const BUMP_DEVICE: Query =
    Query::new("UPDATE devices SET epoch = epoch + 1 WHERE device = ?1");

/// Ends every session of one device.
pub(super) const END_DEVICE: Query = Query::new("DELETE FROM sessions WHERE device = ?1");

/// Moves the epoch of every device of an account on.
pub(super) const BUMP_ACCOUNT: Query =
    Query::new("UPDATE devices SET epoch = epoch + 1 WHERE account = ?1");

/// Ends every session of an account.
pub(super) const END_ACCOUNT: Query = Query::new("DELETE FROM sessions WHERE account = ?1");

/// Moves the epoch of the device that holds a session on.
pub(super) const BUMP_SESSION: Query = Query::new(
    "UPDATE devices SET epoch = epoch + 1 \
     WHERE device = (SELECT device FROM sessions WHERE handle = ?1)",
);

/// Ends one session.
pub(super) const END_SESSION: Query = Query::new("DELETE FROM sessions WHERE handle = ?1");

/// Writes a session for a device of an account, at the device's epoch: the
/// token's hash, the handle, the kind, the mode, the time and the address,
/// then the device and the account.
pub(super) const ISSUE: Query = Query::new(
    "INSERT INTO sessions \
     (token_hash, handle, kind, lifetime, account, device, epoch, created, last_seen, address) \
     SELECT ?1, ?2, ?3, ?4, account, device, epoch, ?5, ?5, ?6 FROM devices \
     WHERE device = ?7 AND account = ?8",
);

/// The session a token's hash names, if it is of the kind asked for.
pub(super) const BY_TOKEN: Query = Query::new(
    "SELECT s.handle, s.lifetime = 'shared', s.account, s.device, d.class = 'limited', \
     s.epoch, s.created, s.last_seen \
     FROM sessions s JOIN devices d ON d.device = s.device \
     WHERE s.token_hash = ?1 AND s.kind = ?2",
);

/// Records that a session was used.
pub(super) const TOUCH: Query = Query::new("UPDATE sessions SET last_seen = ?2 WHERE handle = ?1");

/// Moves the store's log into its file and empties the log, so that what a
/// statement removed is in neither.
pub(super) const CHECKPOINT: Query = Query::new("PRAGMA wal_checkpoint(TRUNCATE)");

/// A live session as the store holds it, with its device's class.
pub(super) struct Stored {
    /// The session's handle.
    pub(super) handle: SessionHandle,
    /// The mode the browser signed in with.
    pub(super) mode: Lifetime,
    /// The account that signed in.
    pub(super) account: PublicId,
    /// The device it signed in on.
    pub(super) device: PublicId,
    /// The device's class.
    pub(super) class: DeviceClass,
    /// The epoch the session carries.
    pub(super) epoch: u64,
    /// When the session was issued, in milliseconds.
    pub(super) created: i64,
    /// When the session's use was last recorded, in milliseconds.
    pub(super) seen: i64,
}

/// An identifier as a bound value.
pub(super) fn id_value(id: PublicId) -> Value {
    Value::Text(id.to_string())
}

/// A session handle as a bound value: its eight bytes, most significant
/// first.
pub(super) fn handle_value(handle: SessionHandle) -> Value {
    Value::Blob(handle.0.to_be_bytes().to_vec())
}

/// A fixed name as a bound value.
pub(super) fn name_value(name: &str) -> Value {
    Value::Text(name.to_owned())
}

/// The name a device class is stored under.
pub(super) const fn class_name(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::Personal => "personal",
        DeviceClass::Limited => "limited",
    }
}

/// The name a session's mode is stored under.
pub(super) const fn mode_name(mode: Lifetime) -> &'static str {
    match mode {
        Lifetime::Personal => "personal",
        Lifetime::Shared => "shared",
    }
}

/// The class [`DEVICE`] found. Anything but a plain "not limited" reads as
/// limited, the class that may do less.
pub(super) fn device_class(row: &Row) -> DeviceClass {
    if row.0.first() == Some(&Value::Integer(0)) {
        DeviceClass::Personal
    } else {
        DeviceClass::Limited
    }
}

/// The session [`BY_TOKEN`] found, or `None` when the row is not one this
/// module wrote.
pub(super) fn stored(row: &Row) -> Option<Stored> {
    let [
        Value::Blob(handle),
        Value::Integer(shared),
        Value::Text(account),
        Value::Text(device),
        Value::Integer(limited),
        Value::Integer(epoch),
        Value::Integer(created),
        Value::Integer(seen),
    ] = row.0.as_slice()
    else {
        return None;
    };
    Some(Stored {
        handle: SessionHandle(u64::from_be_bytes(handle.as_slice().try_into().ok()?)),
        mode: if *shared == 0 {
            Lifetime::Personal
        } else {
            Lifetime::Shared
        },
        account: PublicId::parse(account, IdKind::User).ok()?,
        device: PublicId::parse(device, IdKind::Device).ok()?,
        class: if *limited == 0 {
            DeviceClass::Personal
        } else {
            DeviceClass::Limited
        },
        epoch: epoch.unsigned_abs(),
        created: *created,
        seen: *seen,
    })
}

/// Every device's epoch from the rows [`EPOCHS`] found, or `None` when a
/// row is not one this module wrote.
pub(super) fn epochs(rows: &[Row]) -> Option<HashMap<PublicId, u64>> {
    rows.iter()
        .map(|row| match row.0.as_slice() {
            [Value::Text(device), Value::Integer(epoch)] => PublicId::parse(device, IdKind::Device)
                .ok()
                .map(|device| (device, epoch.unsigned_abs())),
            _ => None,
        })
        .collect()
}
