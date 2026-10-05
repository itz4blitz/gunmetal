//! iTunes-style item lists: the `ilst` box inside `moov/udta/meta`.
//!
//! Each child of `ilst` is one item, named by its box type (`©nam`, `trkn`,
//! `covr` and so on) or, for a freeform `----` item, by the `mean` and
//! `name` boxes inside it. An item holds one or more `data` boxes, each a
//! well-known type, a locale and the value (Apple's `QuickTime` metadata
//! documents). Text is decoded and capped; integers are read; artwork in
//! `covr` becomes a reference to its octets, never decoded here; anything
//! else is kept as it is for the tag mapping.
//!
//! An item that cannot be read is skipped with a recorded problem, and the
//! rest of the list is kept (SEC-MED-017).

use super::boxes::{Flow, FourCc, Mp4Box};
use super::probe::{Mp4Error, Mp4Problem};
use crate::parse::{LimitKind, Limits};
use crate::text::{self, Encoding, Text};
use crate::untrusted::Untrusted;

/// One item of an item list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlstItem {
    /// What the item is.
    pub key: ItemKey,
    /// Its values, in the order of its `data` boxes.
    pub values: Vec<ItemValue>,
}

/// What an item is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKey {
    /// An item named by its box type, such as `©nam` or `trkn`.
    Atom(FourCc),
    /// A freeform `----` item, such as `com.apple.iTunes` `iTunSMPB`.
    Freeform {
        /// The namespace, when the item has a `mean` box.
        mean: Option<Text>,
        /// The name.
        name: Text,
    },
}

/// One value of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemValue {
    /// Text: well-known types 1 (UTF-8) and 2 (UTF-16, big-endian).
    Text(Text),
    /// A big-endian signed integer of one to eight octets: type 21.
    Signed(i64),
    /// A big-endian unsigned integer of one to eight octets: type 22.
    Unsigned(u64),
    /// A picture in a `covr` item.
    Picture(PictureRef),
    /// Any other value, as it is: for example the implicit type 0 of
    /// `trkn` and `disk`.
    Binary {
        /// The well-known type.
        type_code: u32,
        /// The octets.
        bytes: Vec<u8>,
    },
}

/// Where a picture's octets are in the file. The picture itself is never
/// decoded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureRef {
    /// The well-known type: 13 for JPEG, 14 for PNG, 27 for BMP, 0 when
    /// the writer did not say.
    pub type_code: u32,
    /// Where the octets start.
    pub offset: u64,
    /// How many there are.
    pub len: u64,
}

/// An item while its boxes are read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Item {
    /// The item's box type.
    kind: FourCc,
    /// Where its box starts.
    offset: u64,
    /// The text of its last `mean` box.
    mean: Option<Text>,
    /// The text of its last `name` box.
    name: Option<Text>,
    /// The values read so far.
    values: Vec<ItemValue>,
}

/// The items of every item list in a movie, gathered as the walk goes
/// through them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ItemList {
    /// The items kept so far.
    items: Vec<IlstItem>,
    /// The item being read.
    current: Item,
    /// The pictures kept so far.
    pictures: u64,
}

/// A freeform item.
const FREEFORM: FourCc = FourCc(*b"----");
/// The namespace of a freeform item.
pub(crate) const MEAN: FourCc = FourCc(*b"mean");
/// The name of a freeform item.
pub(crate) const NAME: FourCc = FourCc(*b"name");
/// A value.
pub(crate) const DATA: FourCc = FourCc(*b"data");
/// The artwork item.
pub(crate) const COVR: FourCc = FourCc(*b"covr");

/// The well-known type of UTF-8 text.
const UTF8: u32 = 1;
/// The well-known type of big-endian UTF-16 text.
const UTF16: u32 = 2;
/// The well-known type of a big-endian signed integer.
const SIGNED: u32 = 21;
/// The well-known type of a big-endian unsigned integer.
const UNSIGNED: u32 = 22;

impl ItemList {
    /// Starts the item whose box is `item`, unless the list already holds
    /// as many items as the tag-field limit allows; then the rest of the
    /// list is skipped, with one problem recorded.
    pub(crate) fn enter(
        &mut self,
        item: Mp4Box<'_>,
        limits: &Limits,
        problems: &mut Vec<Mp4Problem>,
    ) -> Flow {
        let count = u64::try_from(self.items.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        if let Err(fault) = limits.check(LimitKind::TagFields, count, item.offset) {
            problems.push(Mp4Problem::Metadata(fault.into()));
            return Flow::SkipRest;
        }
        self.current = Item {
            kind: item.kind,
            offset: item.offset,
            ..Item::default()
        };
        Flow::Continue
    }

    /// The type of the item being read.
    #[must_use]
    pub(crate) const fn current_kind(&self) -> FourCc {
        self.current.kind
    }

    /// Records a `covr` picture from its `data` header, without the
    /// payload. A picture past the picture-count limit ends the item; one
    /// past the size limit is skipped.
    pub(crate) fn picture(
        &mut self,
        type_code: u32,
        offset: u64,
        len: u64,
        limits: &Limits,
        problems: &mut Vec<Mp4Problem>,
    ) -> Flow {
        match self.push_picture(type_code, offset, len, limits) {
            Ok(()) => Flow::Continue,
            Err(Stop::Skip(error)) => {
                problems.push(Mp4Problem::Metadata(error));
                Flow::Continue
            }
            Err(Stop::Item(error)) => {
                problems.push(Mp4Problem::Metadata(error));
                Flow::SkipRest
            }
        }
    }

    /// Reads a box inside the current item: its `mean`, its `name` or one
    /// of its values. Other boxes are ignored.
    pub(crate) fn leaf(
        &mut self,
        child: Mp4Box<'_>,
        limits: &Limits,
        problems: &mut Vec<Mp4Problem>,
    ) -> Flow {
        let read = match child.kind {
            MEAN => full_text(child, limits).map(|text| self.current.mean = Some(text)),
            NAME => full_text(child, limits).map(|text| self.current.name = Some(text)),
            DATA => self.data(child, limits),
            _ => Ok(()),
        };
        match read {
            Ok(()) => Flow::Continue,
            Err(Stop::Skip(error)) => {
                problems.push(Mp4Problem::Metadata(error));
                Flow::Continue
            }
            Err(Stop::Item(error)) => {
                problems.push(Mp4Problem::Metadata(error));
                Flow::SkipRest
            }
        }
    }

    /// Reads one `data` box of the current item.
    fn data(&mut self, child: Mp4Box<'_>, limits: &Limits) -> Result<(), Stop> {
        let mut body = child.body;
        let type_code = body.u32_be().map_err(skip)?;
        body.skip(4).map_err(skip)?; // the locale
        let offset = body.offset();
        let value = if self.current.kind == COVR {
            self.push_picture(type_code, offset, body.remaining(), limits)?;
            return Ok(());
        } else {
            value(type_code, body.rest(), self.cap(limits))
        };
        self.current.values.push(value);
        Ok(())
    }

    /// Keeps a picture at `offset` of `len` octets, or skips it.
    fn push_picture(
        &mut self,
        type_code: u32,
        offset: u64,
        len: u64,
        limits: &Limits,
    ) -> Result<(), Stop> {
        let count = self.pictures.saturating_add(1);
        limits
            .check(LimitKind::Pictures, count, offset)
            .map_err(|fault| Stop::Item(fault.into()))?;
        limits
            .check(LimitKind::PictureBytes, len, offset)
            .map_err(skip)?;
        self.pictures = count;
        self.current.values.push(ItemValue::Picture(PictureRef {
            type_code,
            offset,
            len,
        }));
        Ok(())
    }

    /// The cap on the text of the current item's values: see
    /// [`text_limit`].
    fn cap(&self, limits: &Limits) -> u32 {
        u32::try_from(limits.get(text_limit(self.current.kind))).unwrap_or(u32::MAX)
    }

    /// Ends the current item and keeps it, unless it is a freeform item
    /// without a name.
    pub(crate) fn leave(&mut self, problems: &mut Vec<Mp4Problem>) {
        let item = std::mem::take(&mut self.current);
        let key = if item.kind == FREEFORM {
            let Some(name) = item.name else {
                problems.push(Mp4Problem::Metadata(Mp4Error::Missing {
                    kind: NAME,
                    offset: item.offset,
                }));
                return;
            };
            ItemKey::Freeform {
                mean: item.mean,
                name,
            }
        } else {
            ItemKey::Atom(item.kind)
        };
        self.items.push(IlstItem {
            key,
            values: item.values,
        });
    }

    /// Every item kept.
    pub(crate) fn into_items(self) -> Vec<IlstItem> {
        self.items
    }
}

/// Why reading a box of an item stopped.
enum Stop {
    /// The box is skipped, and the rest of the item read.
    Skip(Mp4Error),
    /// The rest of the item is skipped.
    Item(Mp4Error),
}

/// Skips one box for `fault`.
fn skip(fault: crate::parse::ParseFault) -> Stop {
    Stop::Skip(fault.into())
}

/// The limit the text of an item list is cut at, by the type of the box
/// that says what the text is. For the text of an item's values that is the
/// item's own type, `----` for a freeform item: the long-text limit for
/// lyrics, comments, descriptions and freeform values, and the short-text
/// limit for every other item. For the text of a `mean` or a `name` box it
/// is the type of that box, and the limit is the short-text limit.
///
/// The parser cuts at this limit, and the tag mapper
/// ([`crate::tags::mp4`]) names it when it reports the cut, so the two
/// cannot disagree.
#[must_use]
pub(crate) fn text_limit(kind: FourCc) -> LimitKind {
    match &kind.0 {
        b"\xA9lyr" | b"\xA9cmt" | b"desc" | b"ldes" | b"----" => LimitKind::LongText,
        _ => LimitKind::ShortText,
    }
}

/// The UTF-8 text of a `mean` or `name` box, after its version and flags,
/// capped as [`text_limit`] says for its type: as a short text.
fn full_text(child: Mp4Box<'_>, limits: &Limits) -> Result<Text, Stop> {
    let mut body = child.body;
    body.skip(4).map_err(skip)?; // version and flags
    let cap = u32::try_from(limits.get(text_limit(child.kind))).unwrap_or(u32::MAX);
    Ok(text::decode(
        Untrusted::new(body.rest()),
        Encoding::Utf8,
        cap,
    ))
}

/// One value of the well-known type `type_code`.
fn value(type_code: u32, octets: &[u8], cap: u32) -> ItemValue {
    let wide = matches!(octets.len(), 1..=8);
    match type_code {
        UTF8 => ItemValue::Text(text::decode(Untrusted::new(octets), Encoding::Utf8, cap)),
        UTF16 => ItemValue::Text(text::decode(Untrusted::new(octets), Encoding::Utf16Be, cap)),
        SIGNED if wide => {
            let fill = match octets.first() {
                Some(&first) if first >= 0x80 => 0xFF,
                _ => 0,
            };
            ItemValue::Signed(i64::from_be_bytes(widened(octets, fill)))
        }
        UNSIGNED if wide => ItemValue::Unsigned(u64::from_be_bytes(widened(octets, 0))),
        _ => ItemValue::Binary {
            type_code,
            bytes: octets.to_vec(),
        },
    }
}

/// `octets`, at most eight, right-aligned in eight octets of `fill`.
fn widened(octets: &[u8], fill: u8) -> [u8; 8] {
    let mut wide = [fill; 8];
    for (slot, &octet) in wide.iter_mut().rev().zip(octets.iter().rev()) {
        *slot = octet;
    }
    wide
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::mp4::boxes::Children;
    use crate::parse::{Budget, Cursor, Depth, ParseFault};
    use gunmetal_testkit::mp4::{data, freeform, full_box, mp4_box};

    /// The depth `level` levels below the root of a container.
    fn depth(level: u64) -> Depth {
        (0..level).fold(Depth::CONTAINER_ROOT, |depth, _| {
            depth
                .descend(&Limits::DEFAULT, 0)
                .expect("the test stays within the depth limit")
        })
    }

    /// Reads the items of an `ilst` body that starts at offset 0, the way
    /// the walk hands them over.
    fn read(body: &[u8], limits: &Limits) -> (Vec<IlstItem>, Vec<Mp4Problem>) {
        let mut list = ItemList::default();
        let mut problems = Vec::new();
        let mut budget = Budget::for_input(0, 0, 999);
        let mut items = Children::new(Cursor::at(body, 0), depth(4));
        while let Some(item) = items
            .next(limits, &mut budget)
            .expect("the test's items are well formed")
        {
            if list.enter(item, limits, &mut problems) == Flow::SkipRest {
                break;
            }
            let mut children = Children::new(item.body, item.depth);
            while let Some(child) = children
                .next(limits, &mut budget)
                .expect("the test's boxes are well formed")
            {
                if list.leaf(child, limits, &mut problems) == Flow::SkipRest {
                    break;
                }
            }
            list.leave(&mut problems);
        }
        (list.into_items(), problems)
    }

    fn text(value: &str, truncated: bool) -> Text {
        Text {
            value: value.to_owned(),
            truncated,
            replaced: false,
        }
    }

    fn atom(kind: [u8; 4], values: Vec<ItemValue>) -> IlstItem {
        IlstItem {
            key: ItemKey::Atom(FourCc(kind)),
            values,
        }
    }

    /// `Limits::DEFAULT` with each of `lowered` set to its value.
    fn lowered(lowered: &[(LimitKind, u64)]) -> Limits {
        lowered
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits
                    .with_override(kind, value)
                    .expect("the test lowers a limit")
            })
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> Mp4Problem {
        Mp4Problem::Metadata(Mp4Error::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        }))
    }

    #[test]
    fn reads_text_items_in_utf8_and_utf16() {
        let body = [
            mp4_box(*b"\xA9nam", &data(1, "Café\u{7}".as_bytes())),
            mp4_box(*b"\xA9ART", &data(2, b"\x00A\xD8\x34\xDD\x1E")),
            mp4_box(*b"aART", &[data(1, b"One"), data(1, b"Two")].concat()),
        ]
        .concat();
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![
                    // The bell is a control character, so it goes.
                    atom(*b"\xA9nam", vec![ItemValue::Text(text("Café", false))]),
                    atom(
                        *b"\xA9ART",
                        vec![ItemValue::Text(text("A\u{1D11E}", false))]
                    ),
                    atom(
                        *b"aART",
                        vec![
                            ItemValue::Text(text("One", false)),
                            ItemValue::Text(text("Two", false)),
                        ]
                    ),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn reads_integers_of_every_width_and_keeps_other_values_as_they_are() {
        let body = [
            mp4_box(*b"cpil", &data(21, &[1])),
            mp4_box(*b"tmpo", &data(21, &[0xFF, 0x38])),
            mp4_box(*b"plID", &data(21, &[0x80, 0, 0, 0, 0, 0, 0, 0])),
            mp4_box(*b"rtng", &data(22, &[0xFF])),
            mp4_box(*b"cnID", &data(22, &[0xFF; 8])),
            mp4_box(*b"atID", &data(22, &[1, 2, 3])),
            // trkn: the implicit type 0, track 3 of 12.
            mp4_box(*b"trkn", &data(0, &[0, 0, 0, 3, 0, 12, 0, 0])),
            mp4_box(*b"long", &data(21, &[0; 9])),
            mp4_box(*b"none", &data(22, &[])),
            mp4_box(*b"sjis", &data(3, b"x")),
        ]
        .concat();
        let binary = |type_code, bytes: &[u8]| ItemValue::Binary {
            type_code,
            bytes: bytes.to_vec(),
        };
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![
                    atom(*b"cpil", vec![ItemValue::Signed(1)]),
                    atom(*b"tmpo", vec![ItemValue::Signed(-200)]),
                    atom(*b"plID", vec![ItemValue::Signed(i64::MIN)]),
                    atom(*b"rtng", vec![ItemValue::Unsigned(255)]),
                    atom(*b"cnID", vec![ItemValue::Unsigned(u64::MAX)]),
                    atom(*b"atID", vec![ItemValue::Unsigned(0x01_0203)]),
                    atom(*b"trkn", vec![binary(0, &[0, 0, 0, 3, 0, 12, 0, 0])]),
                    atom(*b"long", vec![binary(21, &[0; 9])]),
                    atom(*b"none", vec![binary(22, &[])]),
                    atom(*b"sjis", vec![binary(3, b"x")]),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn reads_artwork_as_references_to_its_octets() {
        // covr at 0, its first data box at 8 with the picture at 24, its
        // second at 28 with the picture at 44.
        let body = mp4_box(
            *b"covr",
            &[
                data(13, &[0xFF, 0xD8, 0xFF, 0xE0]),
                data(14, &[0x89, b'P', b'N']),
            ]
            .concat(),
        );
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![atom(
                    *b"covr",
                    vec![
                        ItemValue::Picture(PictureRef {
                            type_code: 13,
                            offset: 24,
                            len: 4,
                        }),
                        ItemValue::Picture(PictureRef {
                            type_code: 14,
                            offset: 44,
                            len: 3,
                        }),
                    ]
                )],
                vec![]
            )
        );
    }

    #[test]
    fn reads_freeform_items_with_and_without_a_mean() {
        let body = [
            freeform(
                Some("com.apple.iTunes"),
                Some("iTunSMPB"),
                &[data(1, b" 00000000 00000840")],
            ),
            freeform(None, Some("MusicBrainz Track Id"), &[data(1, b"id")]),
        ]
        .concat();
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![
                    IlstItem {
                        key: ItemKey::Freeform {
                            mean: Some(text("com.apple.iTunes", false)),
                            name: text("iTunSMPB", false),
                        },
                        values: vec![ItemValue::Text(text(" 00000000 00000840", false))],
                    },
                    IlstItem {
                        key: ItemKey::Freeform {
                            mean: None,
                            name: text("MusicBrainz Track Id", false),
                        },
                        values: vec![ItemValue::Text(text("id", false))],
                    },
                ],
                vec![]
            )
        );
    }

    /// Verifies: SEC-MED-017
    #[test]
    fn drops_a_freeform_item_without_a_name() {
        // A well-known atom first, so the nameless freeform is not at
        // offset 0: the recorded problem names the freeform's own box.
        let day = mp4_box(*b"\xA9day", &data(1, b"2024"));
        let offset = u64::try_from(day.len()).expect("the day item fits");
        let body = [day, freeform(Some("m"), None, &[data(1, b"x")])].concat();
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![atom(
                    *b"\xA9day",
                    vec![ItemValue::Text(text("2024", false))]
                )],
                vec![Mp4Problem::Metadata(Mp4Error::Missing {
                    kind: FourCc(*b"name"),
                    offset,
                })]
            )
        );
    }

    #[test]
    fn keeps_the_last_mean_and_name_and_ignores_other_boxes() {
        let body = mp4_box(
            *b"----",
            &[
                full_box(*b"mean", 0, 0, b"first"),
                full_box(*b"name", 0, 0, b"one"),
                full_box(*b"mean", 0, 0, b"second"),
                full_box(*b"name", 0, 0, b"two"),
                mp4_box(*b"itif", &[0; 4]),
            ]
            .concat(),
        );
        assert_eq!(
            read(&body, &Limits::DEFAULT).0,
            vec![IlstItem {
                key: ItemKey::Freeform {
                    mean: Some(text("second", false)),
                    name: text("two", false),
                },
                values: vec![],
            }]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_short_and_long_text_at_their_limits() {
        // Titles take the short cap; lyrics, comments, descriptions and
        // freeform values take the long one; mean and name are short.
        let limits = lowered(&[(LimitKind::ShortText, 3), (LimitKind::LongText, 5)]);
        let long = |kind: &[u8; 4]| mp4_box(*kind, &data(1, b"abcdefg"));
        let body = [
            long(b"\xA9nam"),
            long(b"\xA9lyr"),
            long(b"\xA9cmt"),
            long(b"desc"),
            long(b"ldes"),
            freeform(Some("mmmm"), Some("nnnn"), &[data(1, b"abcdefg")]),
        ]
        .concat();
        let capped = |value| vec![ItemValue::Text(text(value, true))];
        assert_eq!(
            read(&body, &limits),
            (
                vec![
                    atom(*b"\xA9nam", capped("abc")),
                    atom(*b"\xA9lyr", capped("abcde")),
                    atom(*b"\xA9cmt", capped("abcde")),
                    atom(*b"desc", capped("abcde")),
                    atom(*b"ldes", capped("abcde")),
                    IlstItem {
                        key: ItemKey::Freeform {
                            mean: Some(text("mmm", true)),
                            name: text("nnn", true),
                        },
                        values: capped("abcde"),
                    },
                ],
                vec![]
            )
        );
        // At the cap exactly, nothing is cut.
        let body = mp4_box(*b"\xA9nam", &data(1, b"abc"));
        assert_eq!(
            read(&body, &limits).0,
            vec![atom(*b"\xA9nam", vec![ItemValue::Text(text("abc", false))])]
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn skips_a_value_or_a_name_too_short_to_read() {
        // The data box at 8 has three octets of body at 16; the mean box
        // at 19 has two at 27; the last data box has six at 67. The item and the next value are kept.
        let body = mp4_box(
            *b"----",
            &[
                mp4_box(*b"data", &[0, 0, 1]),
                mp4_box(*b"mean", &[0, 0]),
                full_box(*b"name", 0, 0, b"n"),
                data(1, b"v"),
                mp4_box(*b"data", &[0, 0, 0, 1, 0, 0]),
            ]
            .concat(),
        );
        assert_eq!(
            read(&body, &Limits::DEFAULT),
            (
                vec![IlstItem {
                    key: ItemKey::Freeform {
                        mean: None,
                        name: text("n", false),
                    },
                    values: vec![ItemValue::Text(text("v", false))],
                }],
                vec![
                    truncated(16, 4, 3),
                    truncated(27, 4, 2),
                    truncated(71, 4, 2)
                ]
            )
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn stops_the_list_past_the_tag_field_limit() {
        let item = |text: &[u8]| mp4_box(*b"\xA9nam", &data(1, text));
        let body = [item(b"a"), item(b"b"), item(b"c"), item(b"d")].concat();
        let limits = lowered(&[(LimitKind::TagFields, 2)]);
        assert_eq!(
            read(&body, &limits),
            (
                vec![
                    atom(*b"\xA9nam", vec![ItemValue::Text(text("a", false))]),
                    atom(*b"\xA9nam", vec![ItemValue::Text(text("b", false))]),
                ],
                vec![Mp4Problem::Metadata(Mp4Error::Fault(
                    ParseFault::LimitExceeded {
                        limit: LimitKind::TagFields,
                        value: 3,
                        max: 2,
                        offset: 50,
                    }
                ))]
            )
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn stops_an_artwork_item_past_the_picture_limit() {
        // Two covr items of two pictures each, under a limit of three. The
        // fourth picture starts at 83.
        let pictures = [data(13, &[1]), data(14, &[2])].concat();
        let body = [mp4_box(*b"covr", &pictures), mp4_box(*b"covr", &pictures)].concat();
        let limits = lowered(&[(LimitKind::Pictures, 3)]);
        let picture = |type_code, offset| {
            ItemValue::Picture(PictureRef {
                type_code,
                offset,
                len: 1,
            })
        };
        assert_eq!(
            read(&body, &limits),
            (
                vec![
                    atom(*b"covr", vec![picture(13, 24), picture(14, 41)]),
                    atom(*b"covr", vec![picture(13, 66)]),
                ],
                vec![Mp4Problem::Metadata(Mp4Error::Fault(
                    ParseFault::LimitExceeded {
                        limit: LimitKind::Pictures,
                        value: 4,
                        max: 3,
                        offset: 83,
                    }
                ))]
            )
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn skips_a_picture_larger_than_the_limit() {
        // The second picture, of four octets at 43, is one too many.
        let body = mp4_box(
            *b"covr",
            &[
                data(13, &[1, 2, 3]),
                data(13, &[1, 2, 3, 4]),
                data(14, &[5]),
            ]
            .concat(),
        );
        let limits = lowered(&[(LimitKind::PictureBytes, 3)]);
        assert_eq!(
            read(&body, &limits),
            (
                vec![atom(
                    *b"covr",
                    vec![
                        ItemValue::Picture(PictureRef {
                            type_code: 13,
                            offset: 24,
                            len: 3,
                        }),
                        ItemValue::Picture(PictureRef {
                            type_code: 14,
                            offset: 63,
                            len: 1,
                        }),
                    ]
                )],
                vec![Mp4Problem::Metadata(Mp4Error::Fault(
                    ParseFault::LimitExceeded {
                        limit: LimitKind::PictureBytes,
                        value: 4,
                        max: 3,
                        offset: 43,
                    }
                ))]
            )
        );
    }

    #[test]
    fn reads_an_empty_list_and_an_empty_item() {
        assert_eq!(read(&[], &Limits::DEFAULT), (vec![], vec![]));
        assert_eq!(
            read(&mp4_box(*b"\xA9gen", &[]), &Limits::DEFAULT),
            (vec![atom(*b"\xA9gen", vec![])], vec![])
        );
    }
}
