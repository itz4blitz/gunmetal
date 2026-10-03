//! Which libraries a decision lets its holder see.
//!
//! A [`LibrarySet`] is never built from outside the core: the only one a
//! server holds is inside a [`Permit`](super::decide::Permit), which only
//! [`decide`](super::decide::decide) mints. Storage readers take the permit
//! and filter by its set, so a handler that has not asked the policy has no
//! set to filter by (SEC-API-010, SEC-IAM-070).

use crate::id::PublicId;

/// The libraries a principal may see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySet(Reach);

/// The two shapes a set can take.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reach {
    /// Every library, including any added later.
    Every,
    /// Only these libraries.
    Listed(Vec<PublicId>),
}

impl LibrarySet {
    /// Every library.
    pub(crate) const fn every() -> Self {
        Self(Reach::Every)
    }

    /// Only the libraries in `libraries`.
    pub(crate) const fn listed(libraries: Vec<PublicId>) -> Self {
        Self(Reach::Listed(libraries))
    }

    /// The libraries in both sets.
    #[must_use]
    pub(crate) fn intersection(self, other: Self) -> Self {
        match (self.0, other.0) {
            (Reach::Every, reach) | (reach, Reach::Every) => Self(reach),
            (Reach::Listed(left), Reach::Listed(right)) => Self::listed(
                left.into_iter()
                    .filter(|library| right.contains(library))
                    .collect(),
            ),
        }
    }

    /// Whether the set holds `library`.
    #[must_use]
    pub fn contains(&self, library: &PublicId) -> bool {
        match &self.0 {
            Reach::Every => true,
            Reach::Listed(libraries) => libraries.contains(library),
        }
    }

    /// Whether the set holds the library `row` belongs to.
    #[must_use]
    pub fn admits<T: HasLibrary + ?Sized>(&self, row: &T) -> bool {
        self.contains(&row.library())
    }

    /// The libraries a query must filter by: `None` when the set holds every
    /// library, so no filter applies.
    #[must_use]
    pub fn restriction(&self) -> Option<&[PublicId]> {
        match &self.0 {
            Reach::Every => None,
            Reach::Listed(libraries) => Some(libraries),
        }
    }
}

/// A stored row that belongs to one library. Every catalogue row implements
/// it, so the authorisation layer (WP-065) can check any row against a
/// permit's [`LibrarySet`].
pub trait HasLibrary {
    /// The library the row belongs to.
    fn library(&self) -> PublicId;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::IdKind;

    pub(crate) fn library(n: u8) -> PublicId {
        PublicId::parse(&format!("lib_{n:026}"), IdKind::Library).unwrap()
    }

    struct Row(PublicId);

    impl HasLibrary for Row {
        fn library(&self) -> PublicId {
            self.0
        }
    }

    #[test]
    fn every_library_needs_no_filter_and_holds_any_library() {
        let every = LibrarySet::every();
        assert_eq!(every.restriction(), None);
        assert!(every.contains(&library(1)));
        assert!(every.admits(&Row(library(9))));
    }

    #[test]
    fn a_listed_set_filters_by_its_list() {
        let set = LibrarySet::listed(vec![library(1), library(3)]);
        assert_eq!(set.restriction(), Some(&[library(1), library(3)][..]));
        let held: Vec<bool> = (0..5).map(|n| set.contains(&library(n))).collect();
        assert_eq!(held, [false, true, false, true, false]);
        assert!(set.admits(&Row(library(3))));
        assert!(!set.admits(&Row(library(2))));
    }

    #[test]
    fn intersection_keeps_what_both_hold() {
        let left = || LibrarySet::listed(vec![library(1), library(2), library(3)]);
        let right = || LibrarySet::listed(vec![library(3), library(4), library(2)]);
        assert_eq!(
            left().intersection(right()),
            LibrarySet::listed(vec![library(2), library(3)])
        );
        assert_eq!(LibrarySet::every().intersection(right()), right());
        assert_eq!(left().intersection(LibrarySet::every()), left());
        assert_eq!(
            LibrarySet::every().intersection(LibrarySet::every()),
            LibrarySet::every()
        );
        assert_eq!(
            left().intersection(LibrarySet::listed(Vec::new())),
            LibrarySet::listed(Vec::new())
        );
    }
}
