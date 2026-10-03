//! The structure-aware harness for FLAC (SEC-MED-031).
//!
//! Random bytes rarely get past the `fLaC` marker, so this harness reads
//! its input as a recipe for a stream instead. The stream it writes always
//! has valid framing: the marker, a STREAMINFO block first, block headers
//! whose lengths match their bodies, seek tables of whole seek points,
//! pictures whose inner lengths match their fields, the last-block flag on
//! the final block and the start of a frame after it. The fields inside
//! that framing come from the recipe, so mutations reach the parser's
//! handling of each value. The stream then goes through the plain harness,
//! [`crate::flac_metadata::run`], and its checks.
//!
//! A recipe is read one octet at a time, with zeros once it runs out:
//!
//! - 34 octets of STREAMINFO;
//! - then, until the recipe ends or [`MAX_BLOCKS`] more blocks are
//!   written, one octet whose value modulo 8 is the block type, 7 meaning a
//!   reserved type given by the next octet (7 plus its value modulo 120),
//!   followed by the block's fields:
//!   - STREAMINFO: 34 octets;
//!   - a seek table: a count (modulo 4), then that many 18-octet points;
//!   - a picture: 4 octets of type, a media type length (modulo 16) and
//!     the media type, a description length (modulo 32) and the
//!     description, 16 octets of width, height, depth and colours, and a
//!     data length and the data;
//!   - any other type: a length and that many octets.

use crate::flac_metadata;

/// The most blocks a recipe writes after STREAMINFO.
pub const MAX_BLOCKS: usize = 8;

/// What the harness wrote and what the parser reported for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The stream the recipe describes.
    pub stream: Vec<u8>,
    /// What the plain harness reported for the stream.
    pub parsed: flac_metadata::Outcome,
}

/// Writes the stream `recipe` describes and runs the plain harness on it.
///
/// # Panics
///
/// Panics when [`flac_metadata::run`] does.
#[must_use]
pub fn run(recipe: &[u8]) -> Outcome {
    let stream = stream(recipe);
    let parsed = flac_metadata::run(&stream);
    Outcome { stream, parsed }
}

/// The stream `recipe` describes, laid out as the module says.
#[must_use]
pub fn stream(recipe: &[u8]) -> Vec<u8> {
    let mut recipe = Recipe(recipe.iter());
    let mut blocks = vec![(0, recipe.octets(34))];
    while !recipe.is_empty() && blocks.len() <= MAX_BLOCKS {
        let code = match recipe.octet() % 8 {
            7 => 7 + recipe.octet() % 120,
            code => code,
        };
        let body = match code {
            0 => recipe.octets(34),
            3 => {
                let points = recipe.octet() % 4;
                recipe.octets(usize::from(points) * 18)
            }
            6 => picture(&mut recipe),
            _ => {
                let len = recipe.octet();
                recipe.octets(len.into())
            }
        };
        blocks.push((code, body));
    }
    let last = blocks.len() - 1;
    let mut stream = b"fLaC".to_vec();
    for (index, (code, body)) in blocks.into_iter().enumerate() {
        let flagged = if index == last { code + 0x80 } else { code };
        let [_, high, middle, low] = length(&body);
        stream.extend([flagged, high, middle, low]);
        stream.extend(body);
    }
    stream.extend([0xFF, 0xF8]);
    stream
}

/// A picture block's body, its lengths matching its fields.
fn picture(recipe: &mut Recipe<'_>) -> Vec<u8> {
    let mut body = recipe.octets(4);
    let media_type = recipe.octet() % 16;
    let media_type = recipe.octets(media_type.into());
    body.extend(length(&media_type));
    body.extend(media_type);
    let description = recipe.octet() % 32;
    let description = recipe.octets(description.into());
    body.extend(length(&description));
    body.extend(description);
    body.extend(recipe.octets(16));
    let data = recipe.octet();
    let data = recipe.octets(data.into());
    body.extend(length(&data));
    body.extend(data);
    body
}

/// The big-endian length of `field`, which a recipe keeps far below 2^24.
fn length(field: &[u8]) -> [u8; 4] {
    u32::try_from(field.len()).unwrap_or(0).to_be_bytes()
}

/// A recipe, read one octet at a time.
struct Recipe<'a>(std::slice::Iter<'a, u8>);

impl Recipe<'_> {
    /// The next octet, or zero once the recipe has run out.
    fn octet(&mut self) -> u8 {
        self.0.next().copied().unwrap_or(0)
    }

    /// The next `count` octets, with zeros once the recipe has run out.
    fn octets(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.octet()).collect()
    }

    /// Whether every octet has been read.
    fn is_empty(&self) -> bool {
        self.0.as_slice().is_empty()
    }
}
