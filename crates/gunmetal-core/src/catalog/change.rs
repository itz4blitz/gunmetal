//! What changed in the catalogue: the catalogue store returns these
//! (WP-067) and the change log records them (WP-066), so neither package
//! depends on the other.

use super::coded::coded;
use super::ids::RecordId;

coded! {
    /// Whether a record was written or removed.
    ChangeOp: u8 {
        /// The record was added or changed.
        Upsert = 1,
        /// The record was removed.
        Removal = 2,
    }
}

/// One change to one synced-library record. The record's kind is the
/// kind of its identifier, so a change cannot name a track's identifier as
/// an album.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogChange {
    /// The record that changed.
    pub record: RecordId,
    /// What happened to it.
    pub op: ChangeOp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_ops_have_these_codes() {
        let ops: Vec<(ChangeOp, u8)> = ChangeOp::ALL.iter().map(|o| (*o, o.code())).collect();
        assert_eq!(ops, [(ChangeOp::Upsert, 1), (ChangeOp::Removal, 2)]);
        let read: Vec<Option<ChangeOp>> = (0..=3).map(ChangeOp::from_code).collect();
        assert_eq!(
            read,
            [None, Some(ChangeOp::Upsert), Some(ChangeOp::Removal), None]
        );
    }
}
