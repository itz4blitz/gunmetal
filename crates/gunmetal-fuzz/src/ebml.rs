//! The harness for the EBML parser in [`gunmetal_core::ebml`].

use gunmetal_core::ebml::{
    self, Element, ElementError, ElementHeader, HeaderError, Vint, VintError,
};

/// How many times the harness descends into element bodies: the nesting
/// limit for binary containers in the media security baseline's limits
/// table (SEC-MED-005).
pub const MAX_DEPTH: usize = 32;

/// What the EBML parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// [`ebml::decode_vint`] applied to the whole input.
    pub vint: Result<Vint, VintError>,
    /// [`ebml::decode_element_header`] applied to the whole input.
    pub header: Result<ElementHeader, HeaderError>,
    /// Every item [`ebml::elements`] yielded over the input, depth first,
    /// reading each element's body as if it held child elements.
    pub walk: Vec<Visit<'a>>,
}

/// One item the element walk yielded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit<'a> {
    /// How many element bodies enclose the buffer the item came from.
    pub depth: usize,
    /// The element, or the error that ended its buffer.
    pub item: Result<Element<'a>, ElementError>,
}

/// Feeds `data` to [`ebml::decode_vint`], [`ebml::decode_element_header`]
/// and [`ebml::elements`], descending into element bodies at most
/// [`MAX_DEPTH`] times.
///
/// # Panics
///
/// Panics when the parser breaks an invariant that holds for every input:
/// an integer or header longer than the input, an integer wider than eight
/// octets or larger than its width allows, an error offset outside its
/// buffer, more items than a buffer can hold, or an item after an error.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = data.len();
    let vint = ebml::decode_vint(data);
    if let Ok(found) = vint {
        assert!(
            (1..=8).contains(&found.width)
                && usize::from(found.width) <= octets
                && found.value >> (7 * u32::from(found.width)) == 0,
            "decode_vint returned {found:?} for {octets} octets"
        );
    }
    let header = ebml::decode_element_header(data);
    if let Ok(found) = header {
        assert!(
            usize::from(found.header_len) <= octets,
            "decode_element_header returned {found:?} for {octets} octets"
        );
    }
    let mut walk = Vec::new();
    walk_elements(data, 0, &mut walk);
    Outcome { vint, header, walk }
}

/// Records every item [`ebml::elements`] yields over `buffer`, and descends
/// into each element's body until `depth` reaches [`MAX_DEPTH`].
fn walk_elements<'a>(buffer: &'a [u8], depth: usize, walk: &mut Vec<Visit<'a>>) {
    let octets = buffer.len();
    let mut failed = false;
    for (index, item) in ebml::elements(buffer).enumerate() {
        // An element takes at least two octets, so a buffer holds at most
        // half its length in elements, then one error, then nothing.
        assert!(
            !failed && index <= octets / 2,
            "item {index} from {octets} octets, after an error: {failed}"
        );
        failed = item.is_err();
        if let Err(error) = item {
            assert!(
                error.offset() < octets,
                "{error:?} lies outside {octets} octets"
            );
        }
        walk.push(Visit { depth, item });
        if let Ok(element) = item {
            if depth < MAX_DEPTH {
                walk_elements(element.body, depth + 1, walk);
            }
        }
    }
}
