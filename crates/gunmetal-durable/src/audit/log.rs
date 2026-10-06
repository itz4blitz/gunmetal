//! The audit log: one writer of JSON-lines segments, the address side
//! store, checkpoints and retention.

use std::io::{Read, Write};
use std::ops::Range;
use std::sync::{Arc, Mutex, PoisonError};

use gunmetal_core::audit_event::SecurityEvent;
use gunmetal_core::authz::{Action, Owner, Permit};
use gunmetal_core::client_context::{ClientContext, PathClass};
use gunmetal_core::id::PublicId;
use gunmetal_core::retention::{self, DataClassRetention, Purge, Schedule};
use gunmetal_core::time::Timestamp;
use gunmetal_core::token::mac::MacProvider;
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::{AUDIT_DIR, AUDIT_HEAD, AUDIT_RESERVE, AuditSeg, DataPath};
use gunmetal_fs::sqlite::Db;
use gunmetal_secrets::random::Random;

use crate::audit::addresses;
use crate::audit::chain;
use crate::audit::encode::{self, field_str, field_u64, hex, push_key, unhex32};
use crate::audit::error::{AuditError, BrokenAt};
use crate::audit::parse::{self, Json};
use crate::audit::record::{
    Outcome, OwnRecord, Page, RetentionReport, Seq, SignedHead, TruncatedRecord, WriteClass,
    class_name, in_range, parse_class,
};

/// HMAC key id, address commitment, and the salt plus client context used
/// to store the address.
type SourceCommit<'a> = (
    Option<u8>,
    Option<[u8; 32]>,
    Option<([u8; 16], &'a ClientContext)>,
);

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static FAIL: Cell<u32> = const { Cell::new(0) };
}

#[cfg(test)]
const FAIL_HEAD: u32 = 1;
#[cfg(test)]
const FAIL_WRITE: u32 = 2;
#[cfg(test)]
const FAIL_SYNC: u32 = 4;
#[cfg(test)]
const FAIL_RESERVE: u32 = 8;
#[cfg(test)]
const FAIL_SHRINK: u32 = 16;
#[cfg(test)]
const FAIL_COMMIT: u32 = 32;
#[cfg(test)]
const FAIL_PUT: u32 = 64;
#[cfg(test)]
const FAIL_REMOVE: u32 = 128;
#[cfg(test)]
const FAIL_ADDR_CKPT: u32 = 256;
#[cfg(test)]
const FAIL_LINE: u32 = 512;
#[cfg(test)]
const FAIL_CKPT_LINE: u32 = 1_024;
#[cfg(test)]
const FAIL_CKPT_HEAD: u32 = 2_048;

#[cfg(test)]
fn take_fail(bit: u32) -> bool {
    let bits = FAIL.get();
    if bits & bit == 0 {
        false
    } else {
        FAIL.set(bits & !bit);
        true
    }
}

#[cfg(test)]
fn arm_fail(bit: u32) {
    FAIL.set(FAIL.get() | bit);
}

/// Segment size, checkpoint period and reserve of a production log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Rotate the JSON-lines file after this many bytes.
    pub segment: usize,
    /// Write a checkpoint every this many records.
    pub checkpoint_every: u64,
    /// Write a checkpoint after this many milliseconds even if the count
    /// has not been reached.
    pub checkpoint_ms: i64,
    /// Pre-allocated reserve in bytes.
    pub reserve: usize,
}

impl Limits {
    /// 16 MB segments, a checkpoint every 1,000 records or hour, 64 KiB
    /// reserve.
    pub const DEFAULT: Self = Self {
        segment: 16_777_216,
        checkpoint_every: 1_000,
        checkpoint_ms: 3_600_000,
        reserve: 65_536,
    };

    /// Small limits so rotation and checkpoints are reached in unit tests.
    #[cfg(test)]
    pub(crate) const fn test() -> Self {
        Self {
            segment: 400,
            checkpoint_every: 2,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        }
    }
}

/// One stored line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    seq: Seq,
    ts: Timestamp,
    event: String,
    account: Option<PublicId>,
    class: Option<PathClass>,
    commit: Option<[u8; 32]>,
    kid: Option<u8>,
    outcome: Option<Outcome>,
    prev: [u8; 32],
    hash: [u8; 32],
    kind: Kind,
    raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Event,
    Checkpoint,
    Pruned,
}

struct State {
    halted: bool,
    full: bool,
    next: Seq,
    seg: AuditSeg,
    bytes: usize,
    head: [u8; 32],
    first: Seq,
    lines: Vec<Line>,
    checkpoint: Option<SignedHead>,
    last_checkpoint_at: Option<Timestamp>,
    db: Db,
}

const FIRST_SEG: AuditSeg = match AuditSeg::new(1) {
    Some(seg) => seg,
    None => panic!("segment 1 is inside the path's range"),
};

/// The security audit log of one data directory.
pub struct AuditLog {
    root: DataRoot,
    address: Arc<dyn MacProvider + Send + Sync>,
    signing: Arc<dyn MacProvider + Send + Sync>,
    random: Arc<dyn Random + Send + Sync>,
    limits: Limits,
    state: Mutex<State>,
}

impl AuditLog {
    /// Opens the log beneath `root`.
    ///
    /// # Errors
    ///
    /// [`AuditError`] when the directory, the side store or a segment
    /// cannot be used.
    pub fn open(
        root: DataRoot,
        address: Arc<dyn MacProvider + Send + Sync>,
        signing: Arc<dyn MacProvider + Send + Sync>,
        random: Arc<dyn Random + Send + Sync>,
    ) -> Result<Self, AuditError> {
        Self::open_with(root, address, signing, random, Limits::DEFAULT)
    }

    /// Opens with explicit rotation and checkpoint limits.
    ///
    /// # Errors
    ///
    /// [`AuditError`] when the directory, the side store or a segment
    /// cannot be used.
    pub fn open_with(
        root: DataRoot,
        address: Arc<dyn MacProvider + Send + Sync>,
        signing: Arc<dyn MacProvider + Send + Sync>,
        random: Arc<dyn Random + Send + Sync>,
        limits: Limits,
    ) -> Result<Self, AuditError> {
        drop(root.create_dir(&AUDIT_DIR));
        let db = addresses::open(&root)?;
        ensure_reserve(&root, limits.reserve)?;
        let (mut state, existed) = if let Ok(mut file) = root.open_read(&AUDIT_HEAD) {
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            (load_head(&text, db)?, true)
        } else {
            (
                State {
                    halted: false,
                    full: false,
                    next: 1,
                    seg: FIRST_SEG,
                    bytes: 0,
                    head: [0; 32],
                    first: 1,
                    lines: Vec::new(),
                    checkpoint: None,
                    last_checkpoint_at: None,
                    db,
                },
                false,
            )
        };
        if existed {
            load_segments(&root, &mut state)?;
        } else {
            persist_head(&root, &state)?;
        }
        Ok(Self {
            root,
            address,
            signing,
            random,
            limits,
            state: Mutex::new(state),
        })
    }

    /// Appends `event` at `now`. `from` is the request’s `ClientContext`
    /// when the event carries a source; producers that have none pass
    /// `None`.
    ///
    /// # Errors
    ///
    /// [`AuditError::DiskFull`] for an ordinary event when the disk is
    /// full; any I/O, MAC or randomness failure; [`AuditError::Halted`]
    /// after a partial failure.
    pub fn append_security_event(
        &self,
        now: Timestamp,
        event: &SecurityEvent,
        from: Option<&ClientContext>,
        class: WriteClass,
    ) -> Result<Seq, AuditError> {
        self.with(|state| {
            let seq = state.next;
            self.append_event(state, now, event, from, class)?;
            maybe_checkpoint(self, state, now)?;
            Ok(seq)
        })
    }

    /// The person’s own records (SEC-IAM-097).
    ///
    /// # Errors
    ///
    /// [`AuditError::Denied`] unless `permit` allows reading that account’s
    /// own data.
    pub fn read_own(
        &self,
        permit: &Permit,
        range: Range<u64>,
    ) -> Result<Page<OwnRecord>, AuditError> {
        let (Action::ReadOwnData, Some(Owner::Account(account))) =
            (permit.action(), permit.owner())
        else {
            return Err(AuditError::Denied);
        };
        self.reading(|state| {
            let mut records = Vec::new();
            for line in &state.lines {
                if line.kind != Kind::Event || line.account != Some(account) {
                    continue;
                }
                let addr = addresses::get(&state.db, line.seq)?.and_then(|(addr, _)| addr);
                records.push(own(line, addr));
            }
            Ok(in_range(Page { records }, range))
        })
    }

    /// The full log for a holder of the audit capability, with other
    /// people’s addresses truncated (SEC-OPS-027).
    ///
    /// # Errors
    ///
    /// [`AuditError::Denied`] unless `permit` allows reading the audit log.
    pub fn read_all(
        &self,
        permit: &Permit,
        range: Range<u64>,
    ) -> Result<Page<TruncatedRecord>, AuditError> {
        if permit.action() != Action::ReadAuditLog {
            return Err(AuditError::Denied);
        }
        self.reading(|state| {
            let mut records = Vec::new();
            for line in &state.lines {
                if line.kind != Kind::Event {
                    continue;
                }
                records.push(truncated(line));
            }
            addresses::fill_truncated(&state.db, &mut records)?;
            Ok(in_range(Page { records }, range))
        })
    }

    /// The latest signed checkpoint head (SEC-OPS-075).
    ///
    /// # Errors
    ///
    /// [`AuditError::Denied`] unless `permit` allows reading the audit log.
    pub fn head(&self, permit: &Permit) -> Result<SignedHead, AuditError> {
        if permit.action() != Action::ReadAuditLog {
            return Err(AuditError::Denied);
        }
        self.reading(|state| {
            state
                .checkpoint
                .clone()
                .ok_or(AuditError::Corrupt { seq: 0 })
        })
    }

    /// Coarsens and removes addresses, and prunes whole old segments, on
    /// the schedule (SEC-OPS-026, SEC-PRV-003, SEC-PRV-005).
    ///
    /// # Errors
    ///
    /// I/O, SQLite or MAC failures.
    pub fn apply_retention(
        &self,
        schedule: &Schedule,
        now: Timestamp,
    ) -> Result<RetentionReport, AuditError> {
        self.with(|state| apply_retention(self, state, schedule, now))
    }

    /// Walks the hash chain and checkpoint MACs.
    ///
    /// # Errors
    ///
    /// [`BrokenAt`] naming the first bad sequence number.
    pub fn verify_audit_log(&self) -> Result<(), BrokenAt> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        verify(&state, self.signing.as_ref())
    }

    /// Host-only: pretends the disk is full, so recovery writes can be
    /// tested (SEC-OPS-020).
    pub fn simulate_full_disk(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.full = true;
    }

    fn reading<R>(
        &self,
        work: impl FnOnce(&State) -> Result<R, AuditError>,
    ) -> Result<R, AuditError> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.halted {
            return Err(AuditError::Halted);
        }
        work(&state)
    }

    fn with<R>(
        &self,
        work: impl FnOnce(&mut State) -> Result<R, AuditError>,
    ) -> Result<R, AuditError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.halted {
            return Err(AuditError::Halted);
        }
        match work(&mut state) {
            Ok(value) => Ok(value),
            Err(AuditError::DiskFull) => Err(AuditError::DiskFull),
            Err(error) => {
                state.halted = true;
                Err(error)
            }
        }
    }

    fn append_event(
        &self,
        state: &mut State,
        now: Timestamp,
        event: &SecurityEvent,
        from: Option<&ClientContext>,
        class: WriteClass,
    ) -> Result<(), AuditError> {
        if state.full && class == WriteClass::Ordinary {
            return Err(AuditError::DiskFull);
        }
        let (name, account, source, outcome) = describe(event, from);
        let (kid, commit, salt): SourceCommit<'_> = match source {
            Some(ctx) => {
                // Not `[0; N]`: CodeQL's rust/hard-coded-cryptographic-value
                // treats a repeated literal as a salt source and does not
                // see `Random::fill` as a barrier. Index bytes are not a
                // constant, and a skipped fill is not the zeros a literal
                // would have left. A bitwise mix of the index is omitted
                // because `fill` overwrites it and the mix's mutants live.
                let mut salt = core::array::from_fn(|index| {
                    let [b0, ..] = index.to_le_bytes();
                    b0
                });
                self.random.fill(&mut salt)?;
                let committed = addresses::commit(self.address.as_ref(), ctx.addr(), &salt);
                #[cfg(test)]
                let committed = if take_fail(FAIL_COMMIT) {
                    Err(AuditError::MacUnavailable)
                } else {
                    committed
                };
                let (kid, tag) = committed?;
                (Some(kid), Some(tag), Some((salt, ctx)))
            }
            None => (None, None, None),
        };
        let seq = state.next;
        let prev = state.head;
        let canonical = event_canonical(
            seq,
            now,
            name,
            account.as_ref(),
            source,
            commit.as_ref(),
            kid,
            outcome,
            &prev,
        );
        let hash = chain::digest(&prev, &canonical);
        let raw = with_hash(&canonical, &hash);
        write_line(&self.root, state, &self.limits, &raw)?;
        if let Some((salt, ctx)) = salt {
            let stored = addresses::put(&state.db, seq, now.millis(), &salt, ctx.addr());
            #[cfg(test)]
            let stored = if take_fail(FAIL_PUT) {
                Err(AuditError::Io(std::io::ErrorKind::Other))
            } else {
                stored
            };
            stored?;
        }
        state.lines.push(Line {
            seq,
            ts: now,
            event: name.to_owned(),
            account,
            class: source.map(ClientContext::class),
            commit,
            kid,
            outcome: Some(outcome),
            prev,
            hash,
            kind: Kind::Event,
            raw,
        });
        state.head = hash;
        state.next = seq.saturating_add(1);
        persist_head(&self.root, state)?;
        if state.full && class == WriteClass::Recovery {
            shrink_reserve(&self.root)?;
        }
        Ok(())
    }
}

fn describe<'a>(
    event: &'a SecurityEvent,
    from: Option<&'a ClientContext>,
) -> (
    &'static str,
    Option<PublicId>,
    Option<&'a ClientContext>,
    Outcome,
) {
    match event {
        SecurityEvent::AuthnLoginFail { source, account }
        | SecurityEvent::AuthzFail { source, account } => (
            event.name().vocabulary(),
            *account,
            Some(from.unwrap_or(source)),
            Outcome::Fail,
        ),
        SecurityEvent::AuthnLoginSuccess { source, account } => (
            event.name().vocabulary(),
            Some(*account),
            Some(from.unwrap_or(source)),
            Outcome::Success,
        ),
        SecurityEvent::ExcessRateLimitExceeded { source } => (
            event.name().vocabulary(),
            None,
            Some(from.unwrap_or(source)),
            Outcome::Denied,
        ),
        SecurityEvent::GmDebugLoggingEnabled { account } => {
            (event.name().vocabulary(), *account, from, Outcome::Enabled)
        }
        SecurityEvent::GmEgressDenied {} => {
            (event.name().vocabulary(), None, from, Outcome::Denied)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn event_canonical(
    seq: Seq,
    now: Timestamp,
    name: &str,
    account: Option<&PublicId>,
    source: Option<&ClientContext>,
    commit: Option<&[u8; 32]>,
    kid: Option<u8>,
    outcome: Outcome,
    prev: &[u8; 32],
) -> String {
    let mut line = String::from("{");
    field_u64(&mut line, "seq", seq);
    field_str(&mut line, "ts", &gunmetal_core::time::format_rfc3339(now));
    field_str(&mut line, "event", name);
    if let Some(account) = account {
        push_key(&mut line, "actor");
        line.push('{');
        field_str(&mut line, "account", &account.to_string());
        line.push('}');
    }
    if let (Some(ctx), Some(commit), Some(kid)) = (source, commit, kid) {
        push_key(&mut line, "source");
        line.push('{');
        field_str(&mut line, "class", class_name(ctx.class()));
        field_str(
            &mut line,
            "via",
            if ctx.via_proxy() { "proxy" } else { "direct" },
        );
        field_str(&mut line, "commit", &hex(commit));
        field_u64(&mut line, "kid", u64::from(kid));
        line.push('}');
    }
    field_str(&mut line, "outcome", outcome.as_str());
    field_str(&mut line, "prev", &hex(prev));
    line.push('}');
    line
}

fn with_hash(canonical: &str, hash: &[u8; 32]) -> String {
    let mut line = canonical.trim_end_matches('}').to_owned();
    field_str(&mut line, "hash", &hex(hash));
    line.push('}');
    line
}

fn checkpoint_payload(seq: Seq, head: &[u8; 32]) -> Vec<u8> {
    let mut msg = Vec::from(encode::DOMAIN);
    msg.extend_from_slice(&seq.to_be_bytes());
    msg.extend_from_slice(head);
    msg
}

fn maybe_checkpoint(log: &AuditLog, state: &mut State, now: Timestamp) -> Result<(), AuditError> {
    let count = state.next.saturating_sub(1);
    let due_count = log.limits.checkpoint_every != 0 && count % log.limits.checkpoint_every == 0;
    let due_time = match state.last_checkpoint_at {
        Some(at) => now.millis().saturating_sub(at.millis()) >= log.limits.checkpoint_ms,
        None => state.lines.first().is_some_and(|first| {
            now.millis().saturating_sub(first.ts.millis()) >= log.limits.checkpoint_ms
        }),
    };
    if !(due_count || due_time) {
        return Ok(());
    }
    write_checkpoint(log, state, now)
}

fn write_checkpoint(log: &AuditLog, state: &mut State, now: Timestamp) -> Result<(), AuditError> {
    let kid = log.signing.current_kid();
    let payload = checkpoint_payload(state.next, &state.head);
    let mac = log
        .signing
        .mac(kid, &payload)
        .ok_or(AuditError::MacUnavailable)?;
    let seq = state.next;
    let prev = state.head;
    let mut canonical = String::from("{");
    field_u64(&mut canonical, "seq", seq);
    field_str(
        &mut canonical,
        "ts",
        &gunmetal_core::time::format_rfc3339(now),
    );
    field_str(&mut canonical, "event", "gm_audit_checkpoint");
    field_str(&mut canonical, "head", &hex(&state.head));
    field_u64(&mut canonical, "kid", u64::from(kid));
    field_str(&mut canonical, "sig", &hex(&mac));
    field_str(&mut canonical, "prev", &hex(&prev));
    canonical.push('}');
    let hash = chain::digest(&prev, &canonical);
    let raw = with_hash(&canonical, &hash);
    let wrote = write_line(&log.root, state, &log.limits, &raw);
    #[cfg(test)]
    let wrote = if take_fail(FAIL_CKPT_LINE) {
        Err(AuditError::Io(std::io::ErrorKind::Other))
    } else {
        wrote
    };
    wrote?;
    let signed = SignedHead {
        seq,
        head: state.head,
        kid,
        mac,
        at: now,
    };
    state.lines.push(Line {
        seq,
        ts: now,
        event: "gm_audit_checkpoint".to_owned(),
        account: None,
        class: None,
        commit: None,
        kid: Some(kid),
        outcome: None,
        prev,
        hash,
        kind: Kind::Checkpoint,
        raw,
    });
    state.head = hash;
    state.next = seq.saturating_add(1);
    state.checkpoint = Some(signed);
    state.last_checkpoint_at = Some(now);
    let headed = persist_head(&log.root, state);
    #[cfg(test)]
    let headed = if take_fail(FAIL_CKPT_HEAD) {
        Err(AuditError::Io(std::io::ErrorKind::Other))
    } else {
        headed
    };
    headed?;
    Ok(())
}

fn write_line(
    root: &DataRoot,
    state: &mut State,
    limits: &Limits,
    raw: &str,
) -> Result<(), AuditError> {
    let add = raw.len().saturating_add(1);
    if state.bytes > 0 && state.bytes.saturating_add(add) > limits.segment {
        let next = state
            .seg
            .next()
            .ok_or(AuditError::Corrupt { seq: state.next })?;
        state.seg = next;
        state.bytes = 0;
    }
    let path = DataPath::audit_segment(state.seg);
    #[cfg(test)]
    if take_fail(FAIL_LINE) {
        return Err(AuditError::Io(std::io::ErrorKind::Other));
    }
    let mut file = if state.bytes == 0 {
        root.create_new(&path)?
    } else {
        root.append(&path)?
    };
    let wrote = writeln!(file, "{raw}");
    #[cfg(test)]
    let wrote = if take_fail(FAIL_WRITE) {
        Err(std::io::Error::from(std::io::ErrorKind::WriteZero))
    } else {
        wrote
    };
    wrote?;
    let synced = file.sync_all();
    #[cfg(test)]
    let synced = if take_fail(FAIL_SYNC) {
        Err(std::io::Error::from(std::io::ErrorKind::WriteZero))
    } else {
        synced
    };
    synced?;
    state.bytes = state.bytes.saturating_add(add);
    Ok(())
}

fn persist_head(root: &DataRoot, state: &State) -> Result<(), AuditError> {
    #[cfg(test)]
    if take_fail(FAIL_HEAD) {
        return Err(AuditError::Io(std::io::ErrorKind::Other));
    }
    let mut text = String::from("{");
    field_u64(&mut text, "next", state.next);
    field_u64(&mut text, "seg", u64::from(state.seg.get()));
    field_u64(&mut text, "bytes", u64::try_from(state.bytes).unwrap_or(0));
    field_str(&mut text, "head", &hex(&state.head));
    field_u64(&mut text, "first", state.first);
    text.push('}');
    root.replace(&AUDIT_HEAD, text.as_bytes())?;
    Ok(())
}

fn load_head(text: &str, db: Db) -> Result<State, AuditError> {
    let map = parse::object(text.trim())?;
    let next = map
        .get("next")
        .and_then(Json::num)
        .ok_or(AuditError::Corrupt { seq: 0 })?;
    let seg = map
        .get("seg")
        .and_then(Json::num)
        .and_then(|n| u32::try_from(n).ok())
        .and_then(AuditSeg::new)
        .ok_or(AuditError::Corrupt { seq: 0 })?;
    let bytes = map
        .get("bytes")
        .and_then(Json::num)
        .ok_or(AuditError::Corrupt { seq: 0 })?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a stored segment length is a JSON u64 that fits usize on every architecture this crate builds"
    )]
    let bytes = bytes as usize;
    let head = map
        .get("head")
        .and_then(Json::str)
        .and_then(unhex32)
        .ok_or(AuditError::Corrupt { seq: 0 })?;
    let first = map.get("first").and_then(Json::num).unwrap_or(1);
    Ok(State {
        halted: false,
        full: false,
        next,
        seg,
        bytes,
        head,
        first,
        lines: Vec::new(),
        checkpoint: None,
        last_checkpoint_at: None,
        db,
    })
}

fn load_segments(root: &DataRoot, state: &mut State) -> Result<(), AuditError> {
    let last = state.seg.get();
    let mut n = 0_u32;
    while n <= last {
        if let Some(seg) = AuditSeg::new(n) {
            let path = DataPath::audit_segment(seg);
            if let Ok(mut file) = root.open_read(&path) {
                let mut text = String::new();
                file.read_to_string(&mut text)?;
                for raw in text.lines() {
                    if raw.is_empty() {
                        continue;
                    }
                    let line = parse_line(raw)?;
                    if line.seq < state.first {
                        continue;
                    }
                    if matches!(line.kind, Kind::Checkpoint | Kind::Pruned) {
                        state.checkpoint = Some(SignedHead {
                            seq: line.seq,
                            head: line.prev,
                            kid: line.kid.unwrap_or(0),
                            mac: checkpoint_mac_from_raw(raw).unwrap_or([0; 32]),
                            at: line.ts,
                        });
                        state.last_checkpoint_at = Some(line.ts);
                    }
                    state.lines.push(line);
                }
            }
        }
        n = n.saturating_add(1);
    }
    Ok(())
}

fn checkpoint_mac_from_raw(raw: &str) -> Option<[u8; 32]> {
    let map = parse::object(raw).ok()?;
    map.get("sig").and_then(Json::str).and_then(unhex32)
}

fn parse_line(raw: &str) -> Result<Line, AuditError> {
    let map = parse::object(raw)?;
    let seq = map
        .get("seq")
        .and_then(Json::num)
        .ok_or(AuditError::Corrupt { seq: 0 })?;
    let text = map
        .get("ts")
        .and_then(Json::str)
        .ok_or(AuditError::Corrupt { seq })?;
    let ts = gunmetal_core::time::parse_rfc3339(gunmetal_core::untrusted::Untrusted::new(text))
        .map_err(|_| AuditError::Corrupt { seq })?;
    let event = map
        .get("event")
        .and_then(Json::str)
        .ok_or(AuditError::Corrupt { seq })?
        .to_owned();
    let hash = map
        .get("hash")
        .and_then(Json::str)
        .and_then(unhex32)
        .ok_or(AuditError::Corrupt { seq })?;
    let prev = map
        .get("prev")
        .and_then(Json::str)
        .and_then(unhex32)
        .ok_or(AuditError::Corrupt { seq })?;
    let kind = match event.as_str() {
        "gm_audit_checkpoint" => Kind::Checkpoint,
        "gm_audit_pruned" => Kind::Pruned,
        _ => Kind::Event,
    };
    let account = match map.get("actor").and_then(Json::obj) {
        Some(actor) => match actor.get("account").and_then(Json::str) {
            Some(id) => PublicId::parse(id, gunmetal_core::id::IdKind::User).ok(),
            None => None,
        },
        None => None,
    };
    let source = map.get("source").and_then(Json::obj);
    let class = match source {
        Some(source) => match source.get("class").and_then(Json::str) {
            Some(text) => parse_class(text),
            None => None,
        },
        None => None,
    };
    let commit = match source {
        Some(source) => match source.get("commit").and_then(Json::str) {
            Some(text) => unhex32(text),
            None => None,
        },
        None => None,
    };
    let key_id = match source {
        Some(source) => match source.get("kid").and_then(Json::num) {
            Some(n) => u8::try_from(n).ok(),
            None => None,
        },
        None => None,
    }
    .or_else(|| match map.get("kid").and_then(Json::num) {
        Some(n) => u8::try_from(n).ok(),
        None => None,
    });
    let outcome = match map.get("outcome").and_then(Json::str) {
        Some(text) => Outcome::parse(text),
        None => None,
    };
    Ok(Line {
        seq,
        ts,
        event,
        account,
        class,
        commit,
        kid: key_id,
        outcome,
        prev,
        hash,
        kind,
        raw: raw.to_owned(),
    })
}

fn own(line: &Line, addr: Option<std::net::IpAddr>) -> OwnRecord {
    OwnRecord {
        seq: line.seq,
        ts: line.ts,
        event: line.event.clone(),
        account: line.account,
        addr,
        class: line.class,
        outcome: line.outcome.unwrap_or(Outcome::Denied),
        hash: line.hash,
    }
}

fn truncated(line: &Line) -> TruncatedRecord {
    TruncatedRecord {
        seq: line.seq,
        ts: line.ts,
        event: line.event.clone(),
        account: line.account,
        addr: None,
        class: line.class,
        outcome: line.outcome.unwrap_or(Outcome::Denied),
        hash: line.hash,
    }
}

fn apply_retention(
    log: &AuditLog,
    state: &mut State,
    schedule: &Schedule,
    now: Timestamp,
) -> Result<RetentionReport, AuditError> {
    let mut coarsened = 0_u64;
    let mut removed = 0_u64;
    let mut pruned = 0_u64;
    for (seq, ts, addr, _) in addresses::all(&state.db)? {
        let age = retention::age(Timestamp::from_millis(ts).unwrap_or(Timestamp::MIN), now);
        match retention::decide(DataClassRetention::EventAddress, age, 0, schedule) {
            Purge::Coarsen => {
                if let Some(addr) = addr {
                    addresses::coarsen(&state.db, seq, addr)?;
                    coarsened = coarsened.saturating_add(1);
                }
            }
            Purge::Remove => {
                let removed_row = addresses::remove(&state.db, seq);
                #[cfg(test)]
                let removed_row = if take_fail(FAIL_REMOVE) {
                    Err(AuditError::Io(std::io::ErrorKind::Other))
                } else {
                    removed_row
                };
                removed_row?;
                removed = removed.saturating_add(1);
            }
            Purge::Keep => {}
        }
    }
    let addr_ckpt = addresses::checkpoint(&state.db);
    #[cfg(test)]
    let addr_ckpt = if take_fail(FAIL_ADDR_CKPT) {
        Err(AuditError::Io(std::io::ErrorKind::Other))
    } else {
        addr_ckpt
    };
    addr_ckpt?;
    let mut drop_through = 0_u64;
    for line in &state.lines {
        if line.kind != Kind::Event {
            continue;
        }
        let age = retention::age(line.ts, now);
        if retention::decide(DataClassRetention::SecurityEvent, age, 0, schedule) == Purge::Remove {
            drop_through = line.seq;
        } else {
            break;
        }
    }
    if drop_through >= state.first {
        pruned = drop_through.saturating_sub(state.first).saturating_add(1);
        state.lines.retain(|line| line.seq > drop_through);
        state.first = drop_through.saturating_add(1);
        write_pruned(log, state, now, drop_through)?;
    }
    persist_head(&log.root, state)?;
    Ok(RetentionReport {
        coarsened,
        removed,
        pruned,
    })
}

fn write_pruned(
    log: &AuditLog,
    state: &mut State,
    now: Timestamp,
    through: Seq,
) -> Result<(), AuditError> {
    let kid = log.signing.current_kid();
    let seq = state.next;
    let prev = state.head;
    let payload = checkpoint_payload(seq, &prev);
    let mac = log
        .signing
        .mac(kid, &payload)
        .ok_or(AuditError::MacUnavailable)?;
    let mut canonical = String::from("{");
    field_u64(&mut canonical, "seq", seq);
    field_str(
        &mut canonical,
        "ts",
        &gunmetal_core::time::format_rfc3339(now),
    );
    field_str(&mut canonical, "event", "gm_audit_pruned");
    field_u64(&mut canonical, "through", through);
    field_u64(&mut canonical, "kid", u64::from(kid));
    field_str(&mut canonical, "sig", &hex(&mac));
    field_str(&mut canonical, "prev", &hex(&prev));
    canonical.push('}');
    let hash = chain::digest(&prev, &canonical);
    let raw = with_hash(&canonical, &hash);
    write_line(&log.root, state, &log.limits, &raw)?;
    state.lines.push(Line {
        seq,
        ts: now,
        event: "gm_audit_pruned".to_owned(),
        account: None,
        class: None,
        commit: None,
        kid: Some(kid),
        outcome: None,
        prev,
        hash,
        kind: Kind::Pruned,
        raw,
    });
    state.head = hash;
    state.next = seq.saturating_add(1);
    let signed = SignedHead {
        seq,
        head: prev,
        kid,
        mac,
        at: now,
    };
    state.checkpoint = Some(signed);
    state.last_checkpoint_at = Some(now);
    Ok(())
}

fn verify(state: &State, signing: &dyn MacProvider) -> Result<(), BrokenAt> {
    let mut prev = [0_u8; 32];
    if state.first != 1 {
        if let Some(first) = state.lines.first() {
            prev = first.prev;
        }
    }
    let mut expect = state.first;
    for line in &state.lines {
        if line.seq != expect {
            return Err(BrokenAt { seq: line.seq });
        }
        if line.prev != prev {
            return Err(BrokenAt { seq: line.seq });
        }
        let canonical = line.raw.rsplit_once(",\"hash\":").map(|(h, _)| {
            let mut c = h.to_owned();
            c.push('}');
            c
        });
        let Some(canonical) = canonical else {
            return Err(BrokenAt { seq: line.seq });
        };
        if chain::digest(&prev, &canonical) != line.hash {
            return Err(BrokenAt { seq: line.seq });
        }
        if matches!(line.kind, Kind::Checkpoint | Kind::Pruned) {
            let payload = checkpoint_payload(line.seq, &line.prev);
            let kid = line.kid.unwrap_or(0);
            let Some(mac) = signing.mac(kid, &payload) else {
                return Err(BrokenAt { seq: line.seq });
            };
            let stored = checkpoint_mac_from_raw(&line.raw).unwrap_or([0; 32]);
            if mac != stored {
                return Err(BrokenAt { seq: line.seq });
            }
        }
        prev = line.hash;
        expect = line.seq.saturating_add(1);
    }
    if expect != state.next {
        return Err(BrokenAt { seq: expect });
    }
    Ok(())
}

fn ensure_reserve(root: &DataRoot, bytes: usize) -> Result<(), AuditError> {
    #[cfg(test)]
    if take_fail(FAIL_RESERVE) {
        return Err(AuditError::Io(std::io::ErrorKind::Other));
    }
    if root.open_read(&AUDIT_RESERVE).is_ok() {
        Ok(())
    } else {
        let zeros = vec![0_u8; bytes];
        root.replace(&AUDIT_RESERVE, &zeros)?;
        Ok(())
    }
}

fn shrink_reserve(root: &DataRoot) -> Result<(), AuditError> {
    #[cfg(test)]
    if take_fail(FAIL_SHRINK) {
        return Err(AuditError::Io(std::io::ErrorKind::Other));
    }
    root.replace(&AUDIT_RESERVE, &[0_u8; 8])?;
    Ok(())
}

/// Compile-fail: readers take a `Permit`.
#[cfg(doctest)]
mod compile_fail {
    /// ```
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::audit::error::AuditError;
    /// use gunmetal_durable::audit::log::AuditLog;
    /// use gunmetal_durable::audit::record::{OwnRecord, Page};
    ///
    /// fn history(
    ///     log: &AuditLog,
    ///     permit: &Permit,
    /// ) -> Result<Page<OwnRecord>, AuditError> {
    ///     log.read_own(permit, 1..101)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_durable::audit::error::AuditError;
    /// use gunmetal_durable::audit::log::AuditLog;
    /// use gunmetal_durable::audit::record::{OwnRecord, Page};
    ///
    /// fn history(
    ///     log: &AuditLog,
    /// ) -> Result<Page<OwnRecord>, AuditError> {
    ///     log.read_own(1..101)
    /// }
    /// ```
    struct NoReadOwnWithoutAPermit;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_durable::audit::error::AuditError;
    /// use gunmetal_durable::audit::log::AuditLog;
    /// use gunmetal_durable::audit::record::{Page, TruncatedRecord};
    ///
    /// fn all(
    ///     log: &AuditLog,
    /// ) -> Result<Page<TruncatedRecord>, AuditError> {
    ///     log.read_all(1..101)
    /// }
    /// ```
    struct NoReadAllWithoutAPermit;
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod tests;
