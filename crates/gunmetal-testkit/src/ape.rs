//! A builder for APE tags, versions 1 and 2, as the `APEv2` specification
//! in the Hydrogenaudio Knowledgebase lays them out: an optional 32-octet
//! header, the items, then a 32-octet footer.
//!
//! The header and the footer are the same block: `APETAGEX`, then the
//! version (1000 or 2000), the tag size (the items and the footer, without
//! the header), the item count and the tag flags, each a little-endian
//! 32-bit integer, then eight zero octets. Each item is its value's length
//! and its flags, little-endian 32-bit integers, then its key, a zero
//! octet, and its value.
//!
//! The builder writes what it is given, so a test can declare a size or a
//! count the items do not match, a key the specification forbids, or an
//! item with no key terminator.

/// The tag-flags bit that says the tag has a header.
pub const HAS_HEADER: u32 = 1 << 31;
/// The tag-flags bit that marks a block as the header, not the footer.
pub const IS_HEADER: u32 = 1 << 29;
/// The tag flags of a header block: it has a header (bit 31), and it is
/// the header (bit 29).
const HEADER_FLAGS: u32 = 0xA000_0000;
/// The item flags of a binary item.
pub const BINARY: u32 = 1 << 1;
/// The item flags of an external locator: UTF-8 text naming a URL or a
/// path.
pub const LOCATOR: u32 = 2 << 1;
/// The item flags of the item type the specification reserves.
pub const RESERVED: u32 = 3 << 1;
/// The item-flags bit that marks an item read-only.
pub const READ_ONLY: u32 = 1;

/// An APE tag to write. It starts as an `APEv2` tag with a header and no
/// items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ape {
    version: u32,
    header: bool,
    items: Vec<u8>,
    count: u32,
    declared_count: Option<u32>,
    declared_size: Option<u32>,
}

impl Default for Ape {
    fn default() -> Self {
        Self::new()
    }
}

impl Ape {
    /// An `APEv2` tag with a header and no items.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: 2000,
            header: true,
            items: Vec::new(),
            count: 0,
            declared_count: None,
            declared_size: None,
        }
    }

    /// Writes `version` in the header and the footer.
    #[must_use]
    pub const fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Leaves the header out, as `APEv1` tags and some `APEv2` writers do.
    #[must_use]
    pub const fn without_header(mut self) -> Self {
        self.header = false;
        self
    }

    /// Adds an item with `flags`.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not fit a 32-bit length.
    #[must_use]
    pub fn item(self, key: &[u8], flags: u32, value: &[u8]) -> Self {
        let len = u32::try_from(value.len()).expect("an item value fits a 32-bit length");
        let mut item = Vec::new();
        item.extend(len.to_le_bytes());
        item.extend(flags.to_le_bytes());
        item.extend(key);
        item.push(0);
        item.extend(value);
        self.raw(&item)
    }

    /// Adds a UTF-8 text item.
    #[must_use]
    pub fn text(self, key: &str, value: &str) -> Self {
        self.item(key.as_bytes(), 0, value.as_bytes())
    }

    /// Adds `bytes` as they are, counted as one item: for an item the
    /// specification does not allow.
    #[must_use]
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.items.extend(bytes);
        self.count += 1;
        self
    }

    /// Declares `count` items, whatever was added.
    #[must_use]
    pub const fn declared_count(mut self, count: u32) -> Self {
        self.declared_count = Some(count);
        self
    }

    /// Declares a tag size of `size` octets, whatever was added.
    #[must_use]
    pub const fn declared_size(mut self, size: u32) -> Self {
        self.declared_size = Some(size);
        self
    }

    /// The header block, flagged as the header, whether or not the tag
    /// has one.
    ///
    /// # Panics
    ///
    /// Panics when the items do not fit a 32-bit size.
    #[must_use]
    pub fn header(&self) -> [u8; 32] {
        self.block(HEADER_FLAGS)
    }

    /// The footer block.
    ///
    /// # Panics
    ///
    /// Panics when the items do not fit a 32-bit size.
    #[must_use]
    pub fn footer(&self) -> [u8; 32] {
        self.block(if self.header { HAS_HEADER } else { 0 })
    }

    /// The whole tag: the header if it has one, the items, the footer.
    ///
    /// # Panics
    ///
    /// Panics when the items do not fit a 32-bit size.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut tag = Vec::new();
        if self.header {
            tag.extend(self.header());
        }
        tag.extend(&self.items);
        tag.extend(self.footer());
        tag
    }

    /// A header or footer block with `flags`.
    fn block(&self, flags: u32) -> [u8; 32] {
        let size = self.declared_size.unwrap_or_else(|| {
            u32::try_from(self.items.len() + 32).expect("the items fit a 32-bit size")
        });
        let count = self.declared_count.unwrap_or(self.count);
        let mut block = [0; 32];
        block[..8].copy_from_slice(b"APETAGEX");
        for (at, value) in [(8, self.version), (12, size), (16, count), (20, flags)] {
            block[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `APEv2` tag with a header and one text item, written out from the
    /// specification's tables.
    #[test]
    fn writes_the_layout_the_specification_gives() {
        let tag = Ape::new().text("Title", "Gunmetal").build();
        let expected = [
            // Header: preamble, version 2000, size 54, one item, flags
            // "has a header" and "is the header", reserved.
            &b"APETAGEX"[..],
            &[0xD0, 0x07, 0x00, 0x00],
            &[0x36, 0x00, 0x00, 0x00],
            &[0x01, 0x00, 0x00, 0x00],
            &[0x00, 0x00, 0x00, 0xA0],
            &[0; 8],
            // Item: value length 8, flags 0 (UTF-8 text), key, terminator,
            // value.
            &[0x08, 0x00, 0x00, 0x00],
            &[0x00, 0x00, 0x00, 0x00],
            b"Title\0",
            b"Gunmetal",
            // Footer: as the header, flagged only "has a header".
            b"APETAGEX",
            &[0xD0, 0x07, 0x00, 0x00],
            &[0x36, 0x00, 0x00, 0x00],
            &[0x01, 0x00, 0x00, 0x00],
            &[0x00, 0x00, 0x00, 0x80],
            &[0; 8],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    #[test]
    fn writes_a_tag_without_a_header_as_items_and_footer() {
        let tag = Ape::new()
            .without_header()
            .version(1000)
            .text("Artist", "Band")
            .build();
        let expected = [
            &[0x04, 0x00, 0x00, 0x00][..],
            &[0x00, 0x00, 0x00, 0x00],
            b"Artist\0Band",
            b"APETAGEX",
            &[0xE8, 0x03, 0x00, 0x00],
            &[0x33, 0x00, 0x00, 0x00],
            &[0x01, 0x00, 0x00, 0x00],
            &[0x00; 4],
            &[0; 8],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    #[test]
    fn writes_each_item_with_its_flags_key_and_value_in_order() {
        let tag = Ape::new()
            .without_header()
            .item(b"Cover Art (Front)", BINARY | READ_ONLY, b"a.jpg\0\xFF\xD8")
            .item(b"Link", LOCATOR, b"http://example.invalid/")
            .item(b"Odd", RESERVED, b"")
            .build();
        let expected = [
            &[0x08, 0x00, 0x00, 0x00][..],
            &[0x03, 0x00, 0x00, 0x00],
            b"Cover Art (Front)\0a.jpg\0\xFF\xD8",
            &[0x17, 0x00, 0x00, 0x00],
            &[0x04, 0x00, 0x00, 0x00],
            b"Link\0http://example.invalid/",
            &[0x00, 0x00, 0x00, 0x00],
            &[0x06, 0x00, 0x00, 0x00],
            b"Odd\0",
            b"APETAGEX",
            &[0xD0, 0x07, 0x00, 0x00],
            &[0x72, 0x00, 0x00, 0x00],
            &[0x03, 0x00, 0x00, 0x00],
            &[0x00; 4],
            &[0; 8],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    #[test]
    fn writes_raw_items_as_they_are_and_counts_them() {
        let tag = Ape::new()
            .without_header()
            .raw(b"\x01\x00\x00\x00\x00\x00\x00\x00NoEnd")
            .build();
        let expected = [
            &b"\x01\x00\x00\x00\x00\x00\x00\x00NoEnd"[..],
            b"APETAGEX",
            &[0xD0, 0x07, 0x00, 0x00],
            &[0x2D, 0x00, 0x00, 0x00],
            &[0x01, 0x00, 0x00, 0x00],
            &[0x00; 4],
            &[0; 8],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    #[test]
    fn declares_the_size_and_count_it_is_told_in_both_blocks() {
        let ape = Ape::new()
            .text("A1", "x")
            .declared_count(u32::MAX)
            .declared_size(0x0102_0304);
        let block = |flags: [u8; 4]| {
            [
                &b"APETAGEX"[..],
                &[0xD0, 0x07, 0x00, 0x00],
                &[0x04, 0x03, 0x02, 0x01],
                &[0xFF; 4],
                &flags,
                &[0; 8],
            ]
            .concat()
        };
        assert_eq!(ape.header().to_vec(), block([0x00, 0x00, 0x00, 0xA0]));
        assert_eq!(ape.footer().to_vec(), block([0x00, 0x00, 0x00, 0x80]));
        let item = b"\x01\x00\x00\x00\x00\x00\x00\x00A1\0x";
        assert_eq!(
            ape.build(),
            [&ape.header()[..], item, &ape.footer()].concat()
        );
    }

    /// A header block says the tag has a header (bit 31), since it is one,
    /// and that it is the header (bit 29).
    #[test]
    fn writes_a_header_block_even_for_a_tag_without_one() {
        let ape = Ape::new().without_header();
        assert_eq!(
            ape.header().to_vec(),
            [
                &b"APETAGEX"[..],
                &[0xD0, 0x07, 0x00, 0x00],
                &[0x20, 0x00, 0x00, 0x00],
                &[0x00; 4],
                &[0x00, 0x00, 0x00, 0xA0],
                &[0; 8],
            ]
            .concat()
        );
        assert_eq!(ape.build(), ape.footer().to_vec());
    }

    #[test]
    fn starts_as_an_empty_apev2_tag_with_a_header() {
        assert_eq!(Ape::default(), Ape::new());
        let empty = Ape::new();
        assert_eq!(
            empty.build(),
            [&empty.header()[..], &empty.footer()].concat()
        );
        assert_eq!(
            empty.footer().to_vec(),
            [
                &b"APETAGEX"[..],
                &[0xD0, 0x07, 0x00, 0x00],
                &[0x20, 0x00, 0x00, 0x00],
                &[0x00; 4],
                &[0x00, 0x00, 0x00, 0x80],
                &[0; 8],
            ]
            .concat()
        );
    }

    #[test]
    fn flags_and_type_bits_are_where_the_specification_puts_them() {
        assert_eq!(
            [HAS_HEADER, IS_HEADER, BINARY, LOCATOR, RESERVED, READ_ONLY],
            [0x8000_0000, 0x2000_0000, 0b010, 0b100, 0b110, 0b001]
        );
        assert_eq!(HEADER_FLAGS, HAS_HEADER | IS_HEADER);
    }
}
