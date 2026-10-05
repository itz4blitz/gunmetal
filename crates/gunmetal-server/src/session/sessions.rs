//! The session layer: enrolling devices, issuing sessions, resolving a
//! request's cookie to its one principal, and ending sessions.
//!
//! **Tokens.** A session token is 256 bits from the CSPRNG. The store keeps
//! only its keyed hash, made through the secrets crate's key ring, and a
//! request is checked by looking that hash up (SEC-IAM-037, SEC-OPS-016).
//!
//! **One session per device.** Signing in on a device ends the session the
//! device held, so a new token always replaces the old one (SEC-IAM-038).
//!
//! **Ending.** A session ends by signing out, by running out of time, by a
//! new sign-in on its device or by an epoch bump. Each of these moves the
//! device's epoch on and deletes the session's row, and with it the client
//! address, in one transaction; the store's log is then emptied into its
//! file, so the address is in neither (SEC-PRV-003). The emptying is best
//! effort: a log that is busy keeps its pages until the next session ends.
//!
//! **Revocation.** The current epoch of every device is also held in
//! memory, read back from the store by every write that can change it, so
//! [`Sessions::is_current`] answers without touching the store and a
//! revocation shows on the next request (SEC-IAM-043, SEC-API-017). When
//! the store cannot be read back, no epoch is trusted.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use gunmetal_core::authz::{
    DeviceClass, Elevation, Epoch, Principal, PrincipalFacts, SessionHandle, UserVerification,
};
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::problem::ProblemCode;
use gunmetal_core::time::Clock;
use gunmetal_core::token::hash_session;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_fs::sqlite::{Query, Row, Value};
use gunmetal_http::problem::ApiError;
use gunmetal_secrets::keyring::Purpose;
use gunmetal_secrets::mint::mint;
use gunmetal_secrets::random::Random;
use gunmetal_secrets::root::Root;

use super::cookie::Cookie;
use super::directory::{Directory, Standing};
use super::epoch::EpochScope;
use super::error::SessionError;
use super::hook::Need;
use super::kind::{Listener, TokenKind};
use super::lifetime::Lifetime;
use super::record::{self, Stored};
use super::token::Token;

/// The one answer to a request with no valid credential (SEC-API-003).
pub(super) const UNAUTHENTICATED: ApiError = ApiError::new(ProblemCode::Unauthenticated);

/// The answer to a session that may not use a route as it is.
const STEP_UP: ApiError = ApiError::new(ProblemCode::StepUpRequired);

/// A session's use is recorded at most once in this many milliseconds, so
/// that a busy client does not make the store write on every request. A
/// session can therefore end up to a minute before its idle limit, never
/// after it.
const TOUCH_MS: i64 = 60_000;

/// The session layer's state.
pub struct Sessions {
    store: Arc<IdentityStore>,
    keys: Arc<Root>,
    random: Arc<dyn Random + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    directory: Arc<dyn Directory>,
    /// Every device's current epoch, as last read from the store.
    epochs: Mutex<HashMap<PublicId, u64>>,
}

impl Sessions {
    /// Opens the session layer over an identity store that holds the parts
    /// of [`super::schema::PARTS`].
    ///
    /// # Errors
    ///
    /// [`SessionError::Store`] when the devices cannot be read, and
    /// [`SessionError::Corrupt`] when a device row is not one this module
    /// wrote.
    pub fn open(
        store: Arc<IdentityStore>,
        keys: Arc<Root>,
        random: Arc<dyn Random + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
        directory: Arc<dyn Directory>,
    ) -> Result<Self, SessionError> {
        let sessions = Self {
            store,
            keys,
            random,
            clock,
            directory,
            epochs: Mutex::new(HashMap::new()),
        };
        sessions.load().map(|epochs| Self {
            epochs: Mutex::new(epochs),
            ..sessions
        })
    }

    /// Enrols a device of `class` for `account` and returns its identifier,
    /// minted from the CSPRNG (SEC-HIS-012).
    ///
    /// # Errors
    ///
    /// [`SessionError::UnknownAccount`] when the directory does not know
    /// the account, [`SessionError::RandomnessUnavailable`] when no
    /// identifier can be minted, and [`SessionError::Store`] or
    /// [`SessionError::Corrupt`] when the store fails.
    pub fn enrol(&self, account: PublicId, class: DeviceClass) -> Result<PublicId, SessionError> {
        let now = self.clock.now().millis();
        self.directory
            .standing(account)
            .ok_or(SessionError::UnknownAccount)
            .and_then(|_| {
                mint(IdKind::Device, &*self.random).or(Err(SessionError::RandomnessUnavailable))
            })
            .and_then(|device| {
                self.change(&[record::ENROL
                    .bind(record::id_value(device))
                    .bind(record::id_value(account))
                    .bind(record::name_value(record::class_name(class)))
                    .bind(Value::Integer(now))])
                    .map(|_| device)
            })
    }

    /// Issues a session for `account` on its enrolled `device`, in the mode
    /// the person chose, and returns the cookie to set and the principal
    /// the session resolves to. Any session the device held ends.
    ///
    /// `from` is the client address the listener resolved; it is the only
    /// address the session layer takes (SEC-OPS-037).
    ///
    /// # Errors
    ///
    /// [`SessionError::UnknownAccount`] when the directory does not know
    /// the account, [`SessionError::UnknownDevice`] when the device is not
    /// enrolled for it, [`SessionError::SharedNeedsLimited`] for a shared
    /// session on a personal-class device,
    /// [`SessionError::RandomnessUnavailable`] when no token can be drawn,
    /// and [`SessionError::Store`], [`SessionError::Corrupt`] or
    /// [`SessionError::KeyUnavailable`] when the store or the key fails.
    /// Nothing is issued in any of these cases.
    pub fn issue(
        &self,
        account: PublicId,
        device: PublicId,
        mode: Lifetime,
        from: &ClientContext,
    ) -> Result<(Cookie, Principal), SessionError> {
        self.directory
            .standing(account)
            .ok_or(SessionError::UnknownAccount)
            .and_then(|standing| {
                self.class_of(account, device)
                    .map(|class| (standing, class))
            })
            .and_then(|(standing, class)| {
                mode.fits(class)
                    .then_some((standing, class))
                    .ok_or(SessionError::SharedNeedsLimited)
            })
            .and_then(|(standing, class)| {
                self.start(account, device, class, mode, from)
                    .map(|(token, stored)| {
                        (Cookie::session(&token, mode), principal(&stored, standing))
                    })
            })
    }

    /// Resolves a session cookie's value, presented to `listener`, to its
    /// one principal, for a route that needs `need` (SEC-IAM-002).
    ///
    /// # Errors
    ///
    /// The one `unauthenticated` answer when the value is not a token, the
    /// listener does not take session cookies, no session of the kind has
    /// it, the session has run out of time or the directory no longer knows
    /// its account (SEC-API-003); `step_up_required` when the route needs
    /// an administrator session or a fresh user verification, which no
    /// session yet has (SEC-IAM-041); and `internal_error` when the store
    /// fails or holds a row that cannot be read.
    pub fn authenticate(
        &self,
        listener: Listener,
        cookie: &[u8],
        need: Need,
    ) -> Result<Principal, ApiError> {
        let now = self.clock.now().millis();
        Some(cookie)
            .filter(|_| listener.accepts(TokenKind::WebSession))
            .and_then(Token::parse)
            .ok_or(UNAUTHENTICATED)
            .and_then(|token| self.hash(&token).map_err(internal))
            .and_then(|hash| {
                self.find(
                    &record::BY_TOKEN
                        .bind(Value::Blob(hash.to_vec()))
                        .bind(record::name_value(TokenKind::WebSession.name())),
                )
                .map_err(internal)
            })
            .and_then(|found| found.ok_or(UNAUTHENTICATED))
            .and_then(|stored| self.live(stored, now))
            .and_then(|stored| self.admit(&stored, need, now))
    }

    /// Ends a session: its token stops working and its address leaves the
    /// store. Ending a session that has already ended changes nothing.
    ///
    /// # Errors
    ///
    /// [`SessionError::Store`] or [`SessionError::Corrupt`] when the store
    /// fails.
    pub fn end(&self, session: SessionHandle) -> Result<(), SessionError> {
        self.change(&[
            record::BUMP_SESSION.bind(record::handle_value(session)),
            record::END_SESSION.bind(record::handle_value(session)),
        ])
        .and_then(|_| self.scrub())
    }

    /// Moves the epoch on for an account or a device, which ends every
    /// session in that scope at once (SEC-IAM-043, SEC-TM-028).
    ///
    /// # Errors
    ///
    /// [`SessionError::Store`] or [`SessionError::Corrupt`] when the store
    /// fails.
    pub fn bump_epoch(&self, scope: EpochScope) -> Result<(), SessionError> {
        let (bump, end, id) = match scope {
            EpochScope::Account(account) => (record::BUMP_ACCOUNT, record::END_ACCOUNT, account),
            EpochScope::Device(device) => (record::BUMP_DEVICE, record::END_DEVICE, device),
        };
        self.change(&[
            bump.bind(record::id_value(id)),
            end.bind(record::id_value(id)),
        ])
        .and_then(|_| self.scrub())
    }

    /// Whether the session `principal` came from is still the one its
    /// device holds: nothing has ended it since. Answered from memory, for
    /// callers that hold a principal across a long response.
    #[must_use]
    pub fn is_current(&self, principal: &Principal) -> bool {
        principal.device.and_then(|device| self.epoch_of(device)) == Some(principal.epoch.0)
    }

    /// Runs a read through the session-token door of the identity store.
    fn read(&self, query: &Query) -> Result<Vec<Row>, SessionError> {
        self.store
            .read_pre_principal(PrePrincipal::SessionToken, query)
            .map_err(SessionError::Store)
    }

    /// Every device's epoch as the store holds it.
    fn load(&self) -> Result<HashMap<PublicId, u64>, SessionError> {
        self.read(&record::EPOCHS)
            .and_then(|rows| record::epochs(&rows).ok_or(SessionError::Corrupt))
    }

    /// Runs `statements` in one transaction and reads every device's epoch
    /// back while no other change can run, whether or not the write
    /// succeeded. When the epochs cannot be read back, none is kept.
    fn change(&self, statements: &[Query]) -> Result<Vec<usize>, SessionError> {
        let mut epochs = lock(&self.epochs);
        let written = self.store.write(statements).map_err(SessionError::Store);
        let loaded = self.load();
        *epochs = loaded.clone().unwrap_or_default();
        written.and_then(|counts| loaded.map(|_| counts))
    }

    /// Empties the store's log into its file.
    fn scrub(&self) -> Result<(), SessionError> {
        self.read(&record::CHECKPOINT).map(|_| ())
    }

    /// The epoch the device has now, if it is enrolled.
    fn epoch_of(&self, device: PublicId) -> Option<u64> {
        lock(&self.epochs).get(&device).copied()
    }

    /// The keyed hash the store keeps for `token`.
    fn hash(&self, token: &Token) -> Result<[u8; 32], SessionError> {
        self.keys
            .key_ring(Purpose::SessionHash)
            .or(Err(SessionError::KeyUnavailable))
            .and_then(|ring| {
                hash_session(Untrusted::new(token.bytes()), &ring)
                    .or(Err(SessionError::KeyUnavailable))
            })
    }

    /// The class of `device`, if it is enrolled for `account`.
    fn class_of(&self, account: PublicId, device: PublicId) -> Result<DeviceClass, SessionError> {
        self.read(
            &record::DEVICE
                .bind(record::id_value(device))
                .bind(record::id_value(account)),
        )
        .and_then(|rows| {
            rows.first()
                .map(record::device_class)
                .ok_or(SessionError::UnknownDevice)
        })
    }

    /// Draws a token and a handle and writes the session, ending whatever
    /// session the device held.
    fn start(
        &self,
        account: PublicId,
        device: PublicId,
        class: DeviceClass,
        mode: Lifetime,
        from: &ClientContext,
    ) -> Result<(Token, Stored), SessionError> {
        let now = self.clock.now().millis();
        Token::draw(&*self.random)
            .and_then(|token| draw_handle(&*self.random).map(|handle| (token, handle)))
            .and_then(|(token, handle)| {
                self.hash(&token)
                    .and_then(|hash| {
                        self.change(&[
                            record::END_DEVICE.bind(record::id_value(device)),
                            record::BUMP_DEVICE.bind(record::id_value(device)),
                            record::ISSUE
                                .bind(Value::Blob(hash.to_vec()))
                                .bind(record::handle_value(handle))
                                .bind(record::name_value(TokenKind::WebSession.name()))
                                .bind(record::name_value(record::mode_name(mode)))
                                .bind(Value::Integer(now))
                                .bind(Value::Text(from.addr().to_string()))
                                .bind(record::id_value(device))
                                .bind(record::id_value(account)),
                        ])
                    })
                    .and_then(|counts| {
                        (counts.last() == Some(&1))
                            .then_some(())
                            .ok_or(SessionError::UnknownDevice)
                    })
                    .and_then(|()| self.scrub())
                    .map(|()| {
                        let stored = Stored {
                            handle,
                            mode,
                            account,
                            device,
                            class,
                            epoch: self.epoch_of(device).unwrap_or_default(),
                            created: now,
                            seen: now,
                        };
                        (token, stored)
                    })
            })
    }

    /// The session a lookup found, if any.
    fn find(&self, query: &Query) -> Result<Option<Stored>, SessionError> {
        self.read(query).and_then(|rows| {
            rows.first()
                .map(|row| record::stored(row).ok_or(SessionError::Corrupt))
                .transpose()
        })
    }

    /// Passes a session that still has time, and ends one that has not.
    fn live(&self, stored: Stored, now: i64) -> Result<Stored, ApiError> {
        if stored.mode.ended(
            now.saturating_sub(stored.created),
            now.saturating_sub(stored.seen),
        ) {
            self.end(stored.handle)
                .map_err(internal)
                .and(Err(UNAUTHENTICATED))
        } else {
            Ok(stored)
        }
    }

    /// The principal of a live session, for a route that needs `need`.
    fn admit(&self, stored: &Stored, need: Need, now: i64) -> Result<Principal, ApiError> {
        self.directory
            .standing(stored.account)
            .ok_or(UNAUTHENTICATED)
            .and_then(|standing| {
                if need.admin || need.fresh {
                    Err(STEP_UP)
                } else {
                    self.touch(stored, now)
                        .map_err(internal)
                        .map(|()| principal(stored, standing))
                }
            })
    }

    /// Records that a session was used, unless that was recorded less than
    /// [`TOUCH_MS`] ago.
    fn touch(&self, stored: &Stored, now: i64) -> Result<(), SessionError> {
        if now.saturating_sub(stored.seen) >= TOUCH_MS {
            self.store
                .write(&[record::TOUCH
                    .bind(record::handle_value(stored.handle))
                    .bind(Value::Integer(now))])
                .map_err(SessionError::Store)
                .map(|_| ())
        } else {
            Ok(())
        }
    }
}

/// Locks the epochs. A panic while another thread held them cannot have
/// left them half-written, since they are replaced whole.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A fresh session handle from `random`.
fn draw_handle(random: &dyn Random) -> Result<SessionHandle, SessionError> {
    let mut bytes = [0; 8];
    random
        .fill(&mut bytes)
        .or(Err(SessionError::RandomnessUnavailable))
        .map(|()| SessionHandle(u64::from_be_bytes(bytes)))
}

/// The answer to a request the session layer could not check. Why goes
/// nowhere near the client (SEC-API-073).
fn internal(_error: SessionError) -> ApiError {
    ApiError::new(ProblemCode::InternalError)
}

/// The one principal a session resolves to: what the directory says of its
/// account, on its device, with no elevation and no user verification.
fn principal(stored: &Stored, standing: Standing) -> Principal {
    Principal {
        facts: PrincipalFacts {
            kind: standing.kind,
            account: Some(stored.account),
            profile: standing.profile,
            capabilities: standing.capabilities,
            libraries: standing.libraries,
            device: stored.class,
            elevation: Elevation::Ordinary,
            verification: UserVerification::Stale,
            reach: standing.reach,
            scope: None,
        },
        device: Some(stored.device),
        session: stored.handle,
        epoch: Epoch(stored.epoch),
    }
}
