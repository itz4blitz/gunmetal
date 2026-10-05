//! The pre-authentication handle: the one way a pathway reads a stored
//! credential before there is a principal (SEC-TM-024, SEC-API-010).
//!
//! Every read of the identity store that returns a user-visible object
//! takes a `Permit` from the policy. A sign-in pathway cannot have one: it
//! reads a credential in order to learn who is asking. The identity store
//! keeps a short written list of the reads that are made before there is a
//! principal, and the credential lookup on that list belongs to the
//! verifier. [`PreAuth`] wraps it. The verifier makes the handle and lends
//! it to a pathway's check for one attempt; no pathway module can make one
//! or call the store's lookup itself, and what the handle reads with is a
//! connection that refuses every write.

use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_fs::sqlite::{Query, Row};

use crate::verifier::pathway::Fault;

/// The handle a pathway's check reads stored credentials through.
pub struct PreAuth<'a> {
    store: &'a IdentityStore,
}

impl<'a> PreAuth<'a> {
    /// The handle onto `store`. Only the verifier makes one.
    pub(super) const fn new(store: &'a IdentityStore) -> Self {
        Self { store }
    }

    /// Runs `query`, a static statement with bound values, as the
    /// credential lookup made before there is a principal.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the identity store fails or refuses the
    /// query, which includes any write.
    pub fn lookup(&self, query: &Query) -> Result<Vec<Row>, Fault> {
        self.store
            .read_pre_principal(PrePrincipal::Credential, query)
            .map_err(Fault::from)
    }
}

/// Compile-fail tests: code outside the verifier cannot make the handle.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so the test shares its imports with the control, which
/// compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the verifier, a pathway reads through the handle
    /// it was handed.
    ///
    /// ```
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::{Query, Row};
    /// use gunmetal_server::verifier::pathway::Fault;
    /// use gunmetal_server::verifier::preauth::PreAuth;
    ///
    /// fn read(lookup: &PreAuth<'_>, query: &Query) -> Result<Vec<Row>, Fault> {
    ///     lookup.lookup(query)
    /// }
    ///
    /// fn same(store: &IdentityStore) -> &IdentityStore {
    ///     store
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// A pathway cannot make a handle of its own, so the only credential
    /// lookup it can run is the one the verifier lends it.
    ///
    /// ```compile_fail,E0624
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::{Query, Row};
    /// use gunmetal_server::verifier::pathway::Fault;
    /// use gunmetal_server::verifier::preauth::PreAuth;
    ///
    /// fn read(lookup: &PreAuth<'_>, query: &Query) -> Result<Vec<Row>, Fault> {
    ///     lookup.lookup(query)
    /// }
    ///
    /// fn handle(store: &IdentityStore) -> PreAuth<'_> {
    ///     PreAuth::new(store)
    /// }
    /// ```
    struct NoHandleOutsideTheVerifier;
}

#[cfg(test)]
mod tests {
    use gunmetal_durable::identity::error::{IdentityError, Step};
    use gunmetal_durable::identity::pre_principal::PrePrincipal;
    use gunmetal_fs::sqlite::{DbError, Query, Row};

    use super::*;
    use crate::verifier::pathway::Fault;
    use crate::verifier::testing::{STAND_IN, add_name, data, open, text};

    const NAMES: Query = Query::new("SELECT name FROM stand_in ORDER BY name");

    /// Verifies: SEC-TM-024, SEC-API-010
    #[test]
    fn reads_as_the_credential_lookup_and_cannot_write() {
        let data = data("preauth");
        let store = open(&data.root, &[STAND_IN]);
        store
            .write(&[add_name("ada"), add_name("bob")])
            .expect("written");
        let lookup = PreAuth::new(&store);
        let stored = Ok(vec![Row(vec![text("ada")]), Row(vec![text("bob")])]);
        assert_eq!(lookup.lookup(&NAMES), stored);
        // The handle's connection refuses a write, and the refusal names the
        // lookup it was made as: the credential lookup, not another's.
        assert_eq!(
            lookup.lookup(&add_name("eve")),
            Err(Fault::Storage(Box::new(IdentityError::Db {
                step: Step::PrePrincipal(PrePrincipal::Credential),
                // SQLITE_READONLY
                error: DbError::Sqlite { code: 8 },
            })))
        );
        assert_eq!(lookup.lookup(&NAMES), stored);
    }
}
