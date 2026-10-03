//! The failure kinds every parser shares.
//!
//! A format's own error type wraps a [`ParseFault`] rather than spelling
//! truncation, budgets, depth and limits again, so every parser reports them
//! the same way (the plan's error conventions).

use super::limits::LimitKind;

/// Why a parse stopped, for the failures every format shares.
///
/// Every variant carries the absolute file offset where the parser was
/// working and the values involved, so a test can assert the whole fault
/// and an admin can be told what was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseFault {
    /// The input ended before a value it declares.
    Truncated {
        /// Where the value starts, in octets from the start of the file.
        offset: u64,
        /// Octets the value needs.
        needed: u64,
        /// Octets the input still held at `offset`.
        available: u64,
    },
    /// The parse spent its whole step budget (SEC-MED-007).
    BudgetExceeded {
        /// Where the parser was working when the budget ran out.
        offset: u64,
    },
    /// A structure nested deeper than its limit allows (SEC-MED-005).
    TooDeep {
        /// Which depth limit it was.
        limit: LimitKind,
        /// The depth the parser tried to reach.
        depth: u64,
        /// The deepest level the limit allows.
        max: u64,
        /// Where the structure that would have gone too deep starts.
        offset: u64,
    },
    /// A count, length or size went past its limit (SEC-MED-006).
    LimitExceeded {
        /// Which limit it was.
        limit: LimitKind,
        /// The count, length or size the input declared or reached.
        value: u64,
        /// The largest value the limit allows.
        max: u64,
        /// Where the structure that went past the limit starts.
        offset: u64,
    },
    /// A syncsafe integer had an octet with its high bit set.
    NotSyncsafe {
        /// Where the integer starts.
        offset: u64,
        /// The four octets as they were read.
        octets: [u8; 4],
    },
}

impl ParseFault {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the file.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Truncated { offset, .. }
            | Self::BudgetExceeded { offset }
            | Self::TooDeep { offset, .. }
            | Self::LimitExceeded { offset, .. }
            | Self::NotSyncsafe { offset, .. } => offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_where_each_kind_of_fault_happened() {
        let faults = [
            ParseFault::Truncated {
                offset: 3,
                needed: 4,
                available: 2,
            },
            ParseFault::BudgetExceeded { offset: 5 },
            ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 7,
            },
            ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 65_537,
                max: 65_536,
                offset: 11,
            },
            ParseFault::NotSyncsafe {
                offset: 13,
                octets: [0x00, 0x80, 0x00, 0x00],
            },
        ];
        assert_eq!(faults.map(|fault| fault.offset()), [3, 5, 7, 11, 13]);
    }
}
