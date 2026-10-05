//! The session layer: enrolling devices, issuing sessions, resolving a
//! request's cookie to its one principal, and ending sessions.
//!
//! Nothing here is built yet: every operation refuses.

use std::sync::Arc;

use gunmetal_core::authz::{DeviceClass, Principal, SessionHandle};
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::id::PublicId;
use gunmetal_core::problem::ProblemCode;
use gunmetal_core::time::Clock;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_http::problem::ApiError;
use gunmetal_secrets::random::Random;
use gunmetal_secrets::root::Root;

use super::cookie::Cookie;
use super::directory::Directory;
use super::epoch::EpochScope;
use super::error::SessionError;
use super::hook::Need;
use super::kind::Listener;
use super::lifetime::Lifetime;

/// The session layer's state.
pub struct Sessions {
    /// What every operation answers until it is built.
    refusal: SessionError,
    /// What every request is answered until requests are resolved.
    answer: ApiError,
    /// Whether any principal is current.
    current: bool,
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
        _store: Arc<IdentityStore>,
        _keys: Arc<Root>,
        _random: Arc<dyn Random + Send + Sync>,
        _clock: Arc<dyn Clock + Send + Sync>,
        _directory: Arc<dyn Directory>,
    ) -> Result<Self, SessionError> {
        Ok(Self {
            refusal: SessionError::KeyUnavailable,
            answer: ApiError::new(ProblemCode::InternalError),
            current: false,
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
    pub fn enrol(&self, _account: PublicId, _class: DeviceClass) -> Result<PublicId, SessionError> {
        Err(self.refusal.clone())
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
        _account: PublicId,
        _device: PublicId,
        _mode: Lifetime,
        _from: &ClientContext,
    ) -> Result<(Cookie, Principal), SessionError> {
        Err(self.refusal.clone())
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
        _listener: Listener,
        _cookie: &[u8],
        _need: Need,
    ) -> Result<Principal, ApiError> {
        Err(self.answer)
    }

    /// Ends a session: its token stops working and its address leaves the
    /// store. Ending a session that has already ended changes nothing.
    ///
    /// # Errors
    ///
    /// [`SessionError::Store`] or [`SessionError::Corrupt`] when the store
    /// fails.
    pub fn end(&self, _session: SessionHandle) -> Result<(), SessionError> {
        Err(self.refusal.clone())
    }

    /// Moves the epoch on for an account or a device, which ends every
    /// session in that scope at once (SEC-IAM-043, SEC-TM-028).
    ///
    /// # Errors
    ///
    /// [`SessionError::Store`] or [`SessionError::Corrupt`] when the store
    /// fails.
    pub fn bump_epoch(&self, _scope: EpochScope) -> Result<(), SessionError> {
        Err(self.refusal.clone())
    }

    /// Whether the session `principal` came from is still the one its
    /// device holds: nothing has ended it since. Answered from memory, for
    /// callers that hold a principal across a long response.
    #[must_use]
    pub const fn is_current(&self, _principal: &Principal) -> bool {
        self.current
    }
}
