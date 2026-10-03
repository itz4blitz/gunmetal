//! The harness for the Ogg page and packet parser in
//! [`gunmetal_core::formats::ogg`].

use gunmetal_core::formats::ogg::{
    self, IndexEntry, PacketError, Page, PageError, PageIndex, STEPS_PER_OCTET,
};
use gunmetal_core::parse::{Budget, Cursor, Limits, ParseFault};

/// One item of a walk over packets: a packet's page offset, granule
/// position and length, or the error.
pub type PacketItem = Result<(u64, Option<u64>, usize), PacketError>;

/// What the Ogg parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// Every item [`ogg::pages`] yielded over the input.
    pub pages: Vec<Result<Page<'a>, PageError>>,
    /// The stream the rest follows: the serial number of the first page,
    /// or 0 when there is none.
    pub serial: u32,
    /// Every item [`ogg::packets`] yielded for that stream: each packet's
    /// page offset, granule position and length, or the error.
    pub packets: Vec<PacketItem>,
    /// The other streams the packet walk recorded.
    pub others: Vec<u32>,
    /// [`ogg::last_granule`] of the stream over the whole input.
    pub last: Result<Option<u64>, ParseFault>,
    /// The stream's [`PageIndex`] after every page read was pushed: its
    /// entries and its stride.
    pub index: (Vec<IndexEntry>, u64),
    /// [`PageIndex::seek`] to the last granule position, or to 0.
    pub seek: Option<IndexEntry>,
}

/// Feeds `data` to [`ogg::pages`], [`ogg::packets`] and
/// [`ogg::last_granule`], each under the budget the module asks for and the
/// default limits, and builds a [`PageIndex`] from the pages.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// more items than octets; items that do not each start where the one
/// before left off, or that run past the input; a page whose body is not
/// as long as its lacing values say; more packet octets than the input
/// holds; a last granule position no page carries; or a seek that lands
/// past its target.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = data.len();
    // A slice never holds more than u64::MAX octets.
    let len = u64::try_from(octets).unwrap_or(u64::MAX);
    let budget = || Budget::for_input(len, STEPS_PER_OCTET, 0);
    let limits = Limits::DEFAULT;
    // Every item moves the walk on by at least one octet.
    let pages: Vec<_> = ogg::pages(Cursor::new(data), &mut budget())
        .take(octets.saturating_add(1))
        .collect();
    assert!(
        pages.len() <= octets
            && pages
                .iter()
                .try_fold(0, |next, item| {
                    let (offset, advance) = match *item {
                        Ok(page) => {
                            let body: usize = page.lacing.iter().map(|&v| usize::from(v)).sum();
                            assert_eq!(page.body.len(), body, "{page:?}");
                            let page_len = 27 + page.lacing.len() + body;
                            (page.offset, u64::try_from(page_len).unwrap_or(u64::MAX))
                        }
                        Err(PageError::Skipped { offset, len }) => (offset, len),
                        Err(PageError::Version { offset, .. } | PageError::Crc { offset, .. }) => {
                            (offset, 1)
                        }
                        Err(PageError::Fault(fault)) => (fault.offset(), 1),
                    };
                    (offset == next && advance >= 1 && offset + advance <= len)
                        .then_some(offset + advance)
                })
                .is_some(),
        "the pages of {octets} octets do not tile them: {pages:?}"
    );
    let serial = pages.iter().flatten().next().map_or(0, |page| page.serial);
    let mut walk_budget = budget();
    let mut walk = ogg::packets(Cursor::new(data), serial, &limits, &mut walk_budget);
    // A packet takes at least one lacing octet and an error at least one
    // octet, apart from one last unfinished packet.
    let packets: Vec<_> = walk
        .by_ref()
        .take(octets.saturating_add(2))
        .map(|item| item.map(|packet| (packet.offset, packet.granule, packet.data.len())))
        .collect();
    let others = walk.other_streams().to_vec();
    assert!(
        packets.len() <= octets + 1
            && packets
                .iter()
                .flatten()
                .map(|packet| packet.2)
                .sum::<usize>()
                <= octets,
        "{packets:?} from {octets} octets"
    );
    let last = ogg::last_granule(Cursor::new(data), serial, &mut budget());
    // A last granule position is one a page of the stream carries.
    assert!(
        matches!(last, Ok(None) | Err(ParseFault::BudgetExceeded { .. }))
            || pages
                .iter()
                .flatten()
                .any(|page| page.serial == serial && Ok(page.granule) == last),
        "last granule {last:?} of stream {serial}"
    );
    let mut index = PageIndex::new(serial, &limits);
    for page in pages.iter().flatten() {
        index.push(page);
    }
    let target = last.ok().flatten().unwrap_or_default();
    let seek = index.seek(target);
    assert!(
        seek.is_none_or(|entry| entry.granule <= target),
        "seeking {target} landed on {seek:?}"
    );
    Outcome {
        pages,
        serial,
        packets,
        others,
        last,
        index: (index.entries().to_vec(), index.stride()),
        seek,
    }
}
