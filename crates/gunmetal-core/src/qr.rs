//! QR code symbols: the square of dark and light modules that carries a
//! byte string.
//!
//! [`encode`] is the one encoder the server and the clients share, so the
//! claim link on the console and an invitation, a pairing link or a
//! help-sign-in link on a screen are the same squares everywhere (ACC-062,
//! ACC-064, ACC-080). It makes the matrix and nothing else. Whoever shows a
//! symbol draws the squares and leaves the margin the standard asks for,
//! four light modules on every side. What a symbol carries is decided by
//! the package that issues it; this module holds no security control.
//!
//! A symbol is a QR Code of ISO/IEC 18004:2015 with these choices made:
//!
//! - Byte mode. The payload is written as it is, eight bits to a byte, with
//!   no character set declared.
//! - Error correction level M, which lets a reader recover a symbol with
//!   about 15 % of its codewords damaged.
//! - The smallest version that holds the payload, from version 1 (21
//!   modules a side, 14 bytes) to version 15 (77 modules, 412 bytes). A
//!   longer payload is refused, never cut. The cap is what a link needs at
//!   its longest: `https://`, a host name of 253 bytes and a port take 267
//!   bytes, which leaves 145 for the path and for the fragment that carries
//!   a code or a key.
//! - The data mask that scores the lowest penalty under the standard's four
//!   rules, and of two masks that score the same, the one the standard
//!   numbers lower.
//!
//! The standard's wording of two of those rules can be read more than one
//! way, and encoders differ. Here a finder-like pattern scores once when the
//! four modules before it or the four after it are light, the margin around
//! the symbol counts as light, and the share of dark modules is rated in
//! whole steps of 5 %, rounded down. That is how zint reads the rules. Every
//! reading gives a symbol any reader reads; they differ only in which of the
//! eight masks is picked.

use std::collections::BTreeMap;
use std::iter::once;

/// The longest payload a symbol holds, in bytes: what version 15 carries at
/// level M.
pub const MAX_PAYLOAD: usize = 412;

/// The four bits that open a run of bytes (Table 2).
const BYTE_MODE: u8 = 0b0100;
/// The four light bits that end the data (clause 7.4.9).
const TERMINATOR: u8 = 0b0000;
/// The two codewords that in turn fill the room the data leaves
/// (clause 7.4.10).
const PAD: [u8; 2] = [0xEC, 0x11];
/// What the third penalty rule looks for, because a reader could take it
/// for a finder pattern: dark, light, three dark, light, dark.
const FINDER_LIKE: [bool; 7] = [true, false, true, true, true, false, true];

/// Why a payload has no symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrError {
    /// The payload is longer than the largest symbol holds.
    TooLong {
        /// Bytes in the payload.
        len: usize,
        /// Bytes the largest symbol holds.
        max: usize,
    },
}

/// A QR code symbol, without the light margin a drawing of it needs.
///
/// Rows are counted from the top and columns from the left, both from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    rows: Vec<Vec<bool>>,
}

impl Matrix {
    /// Modules along one side: 21 for version 1 and four more for each
    /// version after it, up to 77.
    #[must_use]
    pub fn size(&self) -> usize {
        self.rows.len()
    }

    /// Whether the module in `row` and `column` is dark. A place outside
    /// the symbol is light, as the margin around it is.
    #[must_use]
    pub fn is_dark(&self, row: usize, column: usize) -> bool {
        self.rows
            .get(row)
            .and_then(|line| line.get(column))
            .copied()
            .unwrap_or_default()
    }
}

/// One version of the symbol at error correction level M, as the standard's
/// tables give it.
struct Version {
    /// The version's number, from 1 to 15.
    number: u8,
    /// The error correction blocks, as pairs of how many blocks there are
    /// and how many data codewords each holds, shorter blocks first
    /// (Table 9).
    blocks: &'static [(usize, usize)],
    /// Error correction codewords in each block (Table 9).
    correction_len: usize,
    /// The rows, which are also the columns, of the centres of the
    /// alignment patterns (Table E.1).
    alignment: &'static [usize],
    /// The version information, from version 7 (Table D.1).
    information: Option<u32>,
}

/// Versions 1 to 15.
static VERSIONS: [Version; 15] = [
    Version {
        number: 1,
        blocks: &[(1, 16)],
        correction_len: 10,
        alignment: &[],
        information: None,
    },
    Version {
        number: 2,
        blocks: &[(1, 28)],
        correction_len: 16,
        alignment: &[6, 18],
        information: None,
    },
    Version {
        number: 3,
        blocks: &[(1, 44)],
        correction_len: 26,
        alignment: &[6, 22],
        information: None,
    },
    Version {
        number: 4,
        blocks: &[(2, 32)],
        correction_len: 18,
        alignment: &[6, 26],
        information: None,
    },
    Version {
        number: 5,
        blocks: &[(2, 43)],
        correction_len: 24,
        alignment: &[6, 30],
        information: None,
    },
    Version {
        number: 6,
        blocks: &[(4, 27)],
        correction_len: 16,
        alignment: &[6, 34],
        information: None,
    },
    Version {
        number: 7,
        blocks: &[(4, 31)],
        correction_len: 18,
        alignment: &[6, 22, 38],
        information: Some(0x7C94),
    },
    Version {
        number: 8,
        blocks: &[(2, 38), (2, 39)],
        correction_len: 22,
        alignment: &[6, 24, 42],
        information: Some(0x85BC),
    },
    Version {
        number: 9,
        blocks: &[(3, 36), (2, 37)],
        correction_len: 22,
        alignment: &[6, 26, 46],
        information: Some(0x9A99),
    },
    Version {
        number: 10,
        blocks: &[(4, 43), (1, 44)],
        correction_len: 26,
        alignment: &[6, 28, 50],
        information: Some(0xA4D3),
    },
    Version {
        number: 11,
        blocks: &[(1, 50), (4, 51)],
        correction_len: 30,
        alignment: &[6, 30, 54],
        information: Some(0xBBF6),
    },
    Version {
        number: 12,
        blocks: &[(6, 36), (2, 37)],
        correction_len: 22,
        alignment: &[6, 32, 58],
        information: Some(0xC762),
    },
    Version {
        number: 13,
        blocks: &[(8, 37), (1, 38)],
        correction_len: 22,
        alignment: &[6, 34, 62],
        information: Some(0xD847),
    },
    Version {
        number: 14,
        blocks: &[(4, 40), (5, 41)],
        correction_len: 24,
        alignment: &[6, 26, 46, 66],
        information: Some(0xE60D),
    },
    Version {
        number: 15,
        blocks: &[(5, 41), (5, 42)],
        correction_len: 24,
        alignment: &[6, 26, 48, 70],
        information: Some(0xF928),
    },
];

impl Version {
    /// Modules along one side: 21 for version 1 and four more for each
    /// version after it.
    fn size(&self) -> usize {
        usize::from(self.number)
            .saturating_mul(4)
            .saturating_add(17)
    }

    /// Octets the payload's length is written in: one up to version 9 and
    /// two from version 10 (Table 3).
    fn count_len(&self) -> usize {
        if self.number < 10 { 1 } else { 2 }
    }

    /// Data codewords in all the blocks together.
    fn data_len(&self) -> usize {
        self.blocks
            .iter()
            .map(|&(count, len)| count.saturating_mul(len))
            .sum()
    }

    /// The longest payload in bytes: the data codewords, less the length
    /// and the one codeword that the mode and the terminator, four bits
    /// each, take between them.
    fn capacity(&self) -> usize {
        self.data_len()
            .saturating_sub(self.count_len())
            .saturating_sub(1)
    }
}

/// One of the eight data masks.
struct Mask {
    /// The format information for this mask at level M, as the symbol shows
    /// it (Table C.1).
    format: u16,
    /// Whether the mask turns over the module in a row and a column
    /// (Table 10).
    flips: fn(usize, usize) -> bool,
}

/// The data masks in the standard's order, from 000 to 111.
static MASKS: [Mask; 8] = [
    Mask {
        format: 0x5412,
        flips: |row: usize, column: usize| row.wrapping_add(column) % 2 == 0,
    },
    Mask {
        format: 0x5125,
        flips: |row: usize, _| row % 2 == 0,
    },
    Mask {
        format: 0x5E7C,
        flips: |_, column: usize| column % 3 == 0,
    },
    Mask {
        format: 0x5B4B,
        flips: |row: usize, column: usize| row.wrapping_add(column) % 3 == 0,
    },
    Mask {
        format: 0x45F9,
        flips: |row: usize, column: usize| (row / 2).wrapping_add(column / 3) % 2 == 0,
    },
    Mask {
        format: 0x40CE,
        flips: |row: usize, column: usize| {
            let product = row.wrapping_mul(column);
            (product % 2).wrapping_add(product % 3) == 0
        },
    },
    Mask {
        format: 0x4F97,
        flips: |row: usize, column: usize| {
            // The table takes the product modulo 2 before it adds. Whether
            // the sum is even is the same without that step.
            let product = row.wrapping_mul(column);
            product.wrapping_add(product % 3) % 2 == 0
        },
    },
    Mask {
        format: 0x4AA0,
        flips: |row: usize, column: usize| {
            // As in mask 110: the table takes the row plus the column
            // modulo 2 before it adds.
            let product = row.wrapping_mul(column);
            row.wrapping_add(column).wrapping_add(product % 3) % 2 == 0
        },
    },
];

/// What one module of a symbol holds.
enum Module {
    /// Part of a function pattern or of the version information, with its
    /// colour.
    Fixed(bool),
    /// A bit of the format information, counted from the least significant.
    Format(usize),
    /// A module of the encoding region, with the bit placed in it.
    Data(bool),
}

/// Encodes `payload` as the smallest symbol that holds it.
///
/// # Errors
///
/// [`QrError::TooLong`] when the payload is longer than [`MAX_PAYLOAD`]
/// bytes.
pub fn encode(payload: &[u8]) -> Result<Matrix, QrError> {
    let version = VERSIONS
        .iter()
        .find(|version| payload.len() <= version.capacity())
        .ok_or(QrError::TooLong {
            len: payload.len(),
            max: MAX_PAYLOAD,
        })?;
    Ok(symbol(version, &data_codewords(payload, version)))
}

/// The data codewords of a payload in byte mode: the mode, the length, the
/// bytes and the terminator, then pad codewords to fill the version
/// (clauses 7.4.5, 7.4.9 and 7.4.10).
fn data_codewords(payload: &[u8], version: &Version) -> Vec<u8> {
    // A payload that fits a version is never longer than sixteen bits can
    // say.
    let length = u16::try_from(payload.len())
        .unwrap_or(u16::MAX)
        .to_be_bytes()
        .into_iter()
        .skip(2_usize.saturating_sub(version.count_len()));
    // The mode is four bits and so is the terminator. Written as the low
    // half of a first octet and the high half of a last one, they make
    // every codeword the low half of one octet and the high half of the
    // next.
    let octets: Vec<u8> = once(BYTE_MODE)
        .chain(length)
        .chain(payload.iter().copied())
        .chain(once(TERMINATOR))
        .collect();
    octets
        .iter()
        .zip(octets.iter().skip(1))
        .map(|(&left, &right)| {
            let [_, codeword] = (u16::from_be_bytes([left, right]) >> 4).to_be_bytes();
            codeword
        })
        .chain(PAD.into_iter().cycle())
        .take(version.data_len())
        .collect()
}

/// The final sequence of codewords: the data cut into blocks, the error
/// correction codewords worked out for each block, and then the data blocks
/// and the error correction blocks each read one codeword from every block
/// in turn (clauses 7.5.2 and 7.6).
fn interleave(data: &[u8], blocks: &[(usize, usize)], correction_len: usize) -> Vec<u8> {
    let divisor = generator(correction_len);
    let mut rest = data.iter().copied();
    let data_blocks: Vec<Vec<u8>> = blocks
        .iter()
        .flat_map(|&(count, len)| (0..count).map(move |_| len))
        .map(|len| rest.by_ref().take(len).collect())
        .collect();
    let correction_blocks: Vec<Vec<u8>> = data_blocks
        .iter()
        .map(|block| correction_codewords(block, &divisor))
        .collect();
    in_turn(&data_blocks)
        .chain(in_turn(&correction_blocks))
        .collect()
}

/// One codeword from each block in turn until every block is used up; a
/// shorter block is passed over once it has run out.
fn in_turn(blocks: &[Vec<u8>]) -> impl Iterator<Item = u8> {
    let longest = blocks.iter().map(Vec::len).max().unwrap_or_default();
    (0..longest).flat_map(move |index| {
        blocks
            .iter()
            .filter_map(move |block| block.get(index).copied())
    })
}

/// The generator polynomial for `degree` error correction codewords: the
/// product of x - 2^n for every n from 0 below `degree`, as its
/// coefficients from the highest power down, the leading 1 included
/// (Annex A).
fn generator(degree: usize) -> Vec<u8> {
    let mut product = vec![1];
    let mut root = 1;
    for _ in 0..degree {
        // The product times x, plus the product times the root. Adding and
        // subtracting are the same in this field: both are exclusive or.
        let scaled: Vec<u8> = product
            .iter()
            .map(|&coefficient| multiply(coefficient, root))
            .collect();
        product = product
            .iter()
            .copied()
            .chain(once(0))
            .zip(once(0).chain(scaled))
            .map(|(raised, added)| raised ^ added)
            .collect();
        root = multiply(root, 2);
    }
    product
}

/// The error correction codewords of one block: what is left of the block's
/// polynomial, raised by the generator's degree, after dividing it by the
/// generator (clause 7.5.2).
fn correction_codewords(data: &[u8], generator: &[u8]) -> Vec<u8> {
    // The generator without its leading 1: one coefficient for each
    // codeword of the remainder.
    let divisor = generator.iter().skip(1);
    let mut remainder: Vec<u8> = divisor.clone().map(|_| 0).collect();
    for &codeword in data {
        let factor = codeword ^ remainder.first().copied().unwrap_or_default();
        remainder = remainder
            .iter()
            .skip(1)
            .copied()
            .chain(once(0))
            .zip(divisor.clone())
            .map(|(carried, &coefficient)| carried ^ multiply(coefficient, factor))
            .collect();
    }
    remainder
}

/// The product of two elements of the field the codewords are in: GF(2^8)
/// with the polynomial x^8 + x^4 + x^3 + x^2 + 1 (clause 7.5.2).
fn multiply(left: u8, right: u8) -> u8 {
    (0..8).rev().fold(0, |product: u8, bit| {
        // Double what the higher bits of `right` gave, bringing x^8 back
        // into the field, then add `left` if this bit of `right` is set.
        let doubled = (product << 1) ^ (product >> 7).wrapping_mul(0x1D);
        doubled ^ ((right >> bit) & 1).wrapping_mul(left)
    })
}

/// The symbol for a version's data codewords, under the mask that scores
/// the lowest penalty.
fn symbol(version: &Version, data: &[u8]) -> Matrix {
    // `min_by_key` keeps the first of equals, so of two masks that score the
    // same the lower-numbered one is picked. There are always eight
    // candidates.
    candidates(version, data)
        .into_iter()
        .min_by_key(penalty)
        .unwrap_or(Matrix { rows: Vec::new() })
}

/// The symbol for a version's data codewords under each of the eight masks,
/// in the masks' order.
fn candidates(version: &Version, data: &[u8]) -> Vec<Matrix> {
    let mut grid = plan(version);
    place(
        &mut grid,
        &interleave(data, version.blocks, version.correction_len),
    );
    MASKS.iter().map(|mask| render(&grid, mask)).collect()
}

/// Every module of a version's symbol, with the encoding region still
/// light.
fn plan(version: &Version) -> Vec<Vec<Module>> {
    let size = version.size();
    (0..size)
        .map(|row| {
            (0..size)
                .map(|column| module(version, row, column))
                .collect()
        })
        .collect()
}

/// What the module in a row and a column of a version's symbol holds.
fn module(version: &Version, row: usize, column: usize) -> Module {
    let size = version.size();
    let fixed = finder(size, row, column)
        .or_else(|| alignment(version, row, column))
        .or_else(|| timing(row, column))
        .or_else(|| dark_module(size, row, column))
        .or_else(|| version_information(version, row, column));
    match (fixed, format_bit(size, row, column)) {
        (Some(dark), _) => Module::Fixed(dark),
        (None, Some(bit)) => Module::Format(bit),
        (None, None) => Module::Data(false),
    }
}

/// How many modules a place lies from a centre, along the row or along the
/// column, whichever is further: the square ring around the centre that the
/// place is on.
fn ring(row: usize, column: usize, centre_row: usize, centre_column: usize) -> usize {
    row.abs_diff(centre_row).max(column.abs_diff(centre_column))
}

/// The finder patterns and their separators, eight modules square in three
/// corners (clauses 6.3.3 and 6.3.4).
fn finder(size: usize, row: usize, column: usize) -> Option<bool> {
    // A finder pattern's centre is the fourth module in from two edges.
    // Around it the rings are dark, dark, light and dark, and the fourth
    // ring out is the light separator.
    let far = size.saturating_sub(4);
    [(3, 3), (3, far), (far, 3)]
        .into_iter()
        .map(|(centre_row, centre_column)| ring(row, column, centre_row, centre_column))
        .find(|&distance| distance <= 4)
        .map(|distance| matches!(distance, 0 | 1 | 3))
}

/// The alignment patterns, five modules square around every crossing of the
/// version's coordinates that keeps clear of the finder patterns
/// (clause 6.3.6 and Annex E).
fn alignment(version: &Version, row: usize, column: usize) -> Option<bool> {
    // The coordinates are at least twelve apart, so no more than one is
    // within two modules of a place.
    let near = |place: usize| {
        version
            .alignment
            .iter()
            .copied()
            .find(|centre| centre.abs_diff(place) <= 2)
    };
    let (centre_row, centre_column) = (near(row)?, near(column)?);
    finder(version.size(), centre_row, centre_column)
        .is_none()
        .then_some(ring(row, column, centre_row, centre_column) != 1)
}

/// The timing patterns: row 6 and column 6, dark on every even module
/// (clause 6.3.5).
fn timing(row: usize, column: usize) -> Option<bool> {
    match (row, column) {
        (6, along) | (along, 6) => Some(along % 2 == 0),
        _ => None,
    }
}

/// The one module that is always dark, in column 8 just above the
/// bottom-left finder pattern's separator (clause 7.9).
fn dark_module(size: usize, row: usize, column: usize) -> Option<bool> {
    (column == 8 && size.saturating_sub(row) == 8).then_some(true)
}

/// The version information, from version 7: eighteen bits in a block six
/// modules by three beside the top-right finder pattern, and the same bits
/// again, turned about the diagonal, beside the bottom-left one
/// (clause 7.10).
fn version_information(version: &Version, row: usize, column: usize) -> Option<bool> {
    let word = version.information?;
    // The blocks' short side starts eleven modules from the far edge. A
    // place is in a block when it is one of the six modules along and one
    // of the three across.
    let start = version.size().saturating_sub(11);
    [(row, column), (column, row)]
        .into_iter()
        .find_map(|(long, short)| {
            let across = short.checked_sub(start)?;
            matches!((long, across), (0..=5, 0..=2))
                .then_some(long.saturating_mul(3).saturating_add(across))
        })
        .map(|bit| (word >> bit) & 1 == 1)
}

/// Which bit of the format information the module in a row and a column
/// shows, counted from the least significant. The fifteen bits are written
/// twice: once around the top-left finder pattern, and once split between
/// the other two (clause 7.9).
fn format_bit(size: usize, row: usize, column: usize) -> Option<usize> {
    // How far the place is from the bottom edge and from the right edge,
    // with the module on the edge counted as 1.
    let below = size.saturating_sub(row);
    let beside = size.saturating_sub(column);
    match (row, column) {
        (0..=5, 8) => Some(row),
        (7, 8) => Some(6),
        (8, 8) => Some(7),
        (8, 7) => Some(8),
        (8, 0..=5) => Some(14_usize.saturating_sub(column)),
        (8, _) if beside <= 8 => Some(beside.saturating_sub(1)),
        (_, 8) if below <= 7 => Some(15_usize.saturating_sub(below)),
        _ => None,
    }
}

/// The modules of the encoding region in the order the codewords' bits fill
/// them: two columns at a time from the right edge, stepping over the
/// timing column, the first pair upwards and the next downwards in turn,
/// and in each row the right module before the left (clause 7.7.3).
fn data_modules(grid: &[Vec<Module>]) -> Vec<(usize, usize)> {
    let size = grid.len();
    let last = size.saturating_sub(1);
    let columns: Vec<usize> = (0..size).rev().filter(|&column| column != 6).collect();
    columns
        .chunks(2)
        .zip([true, false].into_iter().cycle())
        .flat_map(|(pair, upwards)| {
            (0..size)
                .map(move |step| {
                    if upwards {
                        last.saturating_sub(step)
                    } else {
                        step
                    }
                })
                .flat_map(move |row| pair.iter().map(move |&column| (row, column)))
        })
        .filter(|&(row, column)| {
            matches!(
                grid.get(row).and_then(|line| line.get(column)),
                Some(Module::Data(_))
            )
        })
        .collect()
}

/// Writes the codewords' bits, most significant first, into the encoding
/// region. The few modules the codewords do not reach stay light: they are
/// the standard's remainder bits.
fn place(grid: &mut [Vec<Module>], codewords: &[u8]) {
    let bits = codewords
        .iter()
        .flat_map(|&codeword| (0..8).rev().map(move |bit| (codeword >> bit) & 1 == 1));
    let placed: BTreeMap<(usize, usize), bool> = data_modules(grid).into_iter().zip(bits).collect();
    for (row, line) in grid.iter_mut().enumerate() {
        for (column, module) in line.iter_mut().enumerate() {
            if let Some(&bit) = placed.get(&(row, column)) {
                *module = Module::Data(bit);
            }
        }
    }
}

/// The finished symbol under one mask: the function patterns as they are,
/// the format information that names the mask, and every module of the
/// encoding region turned over where the mask says so (clause 7.8).
fn render(grid: &[Vec<Module>], mask: &Mask) -> Matrix {
    let rows = grid
        .iter()
        .enumerate()
        .map(|(row, line)| {
            line.iter()
                .enumerate()
                .map(|(column, module)| match *module {
                    Module::Fixed(dark) => dark,
                    Module::Format(bit) => (mask.format >> bit) & 1 == 1,
                    Module::Data(dark) => dark != (mask.flips)(row, column),
                })
                .collect()
        })
        .collect();
    Matrix { rows }
}

/// The penalty of a finished symbol under the standard's four rules: the
/// lower it is, the easier the symbol is to read (clause 7.8.3).
fn penalty(matrix: &Matrix) -> usize {
    let rows = &matrix.rows;
    let columns: Vec<Vec<bool>> = (0..rows.len())
        .map(|column| {
            rows.iter()
                .map(|row| row.get(column).copied().unwrap_or_default())
                .collect()
        })
        .collect();
    let along_lines: usize = rows
        .iter()
        .chain(&columns)
        .map(|line| run_penalty(line).saturating_add(finder_penalty(line)))
        .sum();
    along_lines
        .saturating_add(block_penalty(rows))
        .saturating_add(balance_penalty(rows))
}

/// The first rule, for one row or column: every run of five or more modules
/// of one colour scores 3, and 1 more for each module over five.
fn run_penalty(line: &[bool]) -> usize {
    line.chunk_by(|left, right| left == right)
        .map(<[bool]>::len)
        .filter(|&run| run >= 5)
        .map(|run| run.saturating_sub(2))
        .sum()
}

/// The second rule: every square of two modules by two in one colour scores
/// 3, squares that overlap each counted.
fn block_penalty(rows: &[Vec<bool>]) -> usize {
    rows.iter()
        .zip(rows.iter().skip(1))
        .map(|(upper, lower)| {
            upper
                .windows(2)
                .zip(lower.windows(2))
                .filter(|(top, bottom)| top == bottom && top.first() == top.last())
                .count()
        })
        .sum::<usize>()
        .saturating_mul(3)
}

/// The third rule, for one row or column: every finder-like pattern with
/// four light modules before it or after it scores 40, once. The margin
/// around the symbol is light, so a pattern at an end of the line counts.
fn finder_penalty(line: &[bool]) -> usize {
    let margin = [false; 4];
    let padded: Vec<bool> = margin.iter().chain(line).chain(&margin).copied().collect();
    padded
        .windows(15)
        .filter(|window| {
            window.iter().skip(4).take(7).eq(&FINDER_LIKE)
                && (window.iter().take(4).all(|&dark| !dark)
                    || window.iter().skip(11).all(|&dark| !dark))
        })
        .count()
        .saturating_mul(40)
}

/// The fourth rule: 10 for every whole 5 % by which the share of dark
/// modules differs from one half.
fn balance_penalty(rows: &[Vec<bool>]) -> usize {
    let total: usize = rows.iter().map(Vec::len).sum();
    let dark = rows.iter().flatten().filter(|&&dark| dark).count();
    // Twice the dark modules less all the modules, over all the modules, is
    // twice the share's distance from one half: 0.1 for each step of 5 %.
    dark.saturating_mul(2)
        .abs_diff(total)
        .saturating_mul(10)
        .checked_div(total)
        .unwrap_or_default()
        .saturating_mul(10)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use core::marker::PhantomData;
    use proptest::collection::vec;
    use proptest::prelude::*;

    // Published symbols, each with where it comes from. Dark is 1.

    /// The standard's worked example, ISO/IEC 18004:2015 Annex I,
    /// Figure I.2: "01234567" in numeric mode at version 1 and level M,
    /// under mask 010. Transcribed in zint's `backend/tests/test_qr.c`
    /// (`test_qr_encode`, item 7, "same") and in `test_annex_i_qr` of the
    /// `qrcode` crate; the two agree module for module.
    const ISO_ANNEX_I: [&str; 21] = [
        "111111100101101111111",
        "100000100111101000001",
        "101110101000001011101",
        "101110101100001011101",
        "101110101011101011101",
        "100000101000101000001",
        "111111101010101111111",
        "000000001001100000000",
        "101111100100101111100",
        "000101011010100101100",
        "001000110101010011111",
        "000010000100000111100",
        "000111111001010010000",
        "000000001011111001100",
        "111111100110101100000",
        "100000101011111000101",
        "101110101000100101100",
        "101110101100100100000",
        "101110101011010010100",
        "100000100000000110110",
        "111111101111010010100",
    ];

    /// The data codewords of the worked example, as Annex I prints them.
    const ANNEX_I_DATA: [u8; 16] = [
        0x10, 0x20, 0x0C, 0x56, 0x61, 0x80, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC,
        0x11,
    ];

    /// The error correction codewords of the worked example, as Annex I
    /// prints them.
    const ANNEX_I_CORRECTION: [u8; 10] =
        [0xA5, 0x24, 0xD4, 0xC1, 0xED, 0x36, 0xC7, 0x87, 0x2C, 0x55];

    /// The text of the standard's first figure.
    const FIGURE_1_TEXT: &[u8] = b"QR Code Symbol";

    /// ISO/IEC 18004:2015 Figure 1: "QR Code Symbol" in byte mode at
    /// version 1 and level M, under mask 101. From zint's `test_qr_encode`,
    /// item 1, which forces that mask and records the result as the same
    /// as the figure.
    const ISO_FIGURE_1: [&str; 21] = [
        "111111100001101111111",
        "100000101001101000001",
        "101110101110101011101",
        "101110101010001011101",
        "101110100000101011101",
        "100000100010101000001",
        "111111101010101111111",
        "000000001100100000000",
        "100000101111011001110",
        "100010001110001000111",
        "011101111001100100010",
        "110100001011010100110",
        "011111111110001011011",
        "000000001000000010110",
        "111111100111111000110",
        "100000100010011011100",
        "101110100000111000111",
        "101110100100001010100",
        "101110100100101010011",
        "100000100001110111100",
        "111111101011001010010",
    ];

    /// The symbol zint draws for "QR Code Symbol" when it picks the mask
    /// itself: mask 110, where the standard's figure has 101. From zint's
    /// `test_qr_encode`, item 0.
    const FIGURE_1_TEXT_BY_ZINT: [&str; 21] = [
        "111111101001101111111",
        "100000101001101000001",
        "101110101100101011101",
        "101110100010001011101",
        "101110101001101011101",
        "100000100001101000001",
        "111111101010101111111",
        "000000000100100000000",
        "100111111101010010111",
        "100010001110001000111",
        "010100110000101101011",
        "110111001000010111110",
        "011111111110001011011",
        "000000001000011010101",
        "111111101101101010100",
        "100000101010011011100",
        "101110101001110001110",
        "101110101111001001100",
        "101110100100101010011",
        "100000100001101111111",
        "111111101001011000000",
    ];

    /// The text of the symbol at the top of the standard's Figure 29.
    const FIGURE_29_TEXT: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// ISO/IEC 18004:2015 Figure 29, the symbol at the top: 62 characters
    /// in alphanumeric mode at version 4 and level M, under mask 100. From
    /// zint's `test_qr_encode`, item 2, which picks that mask itself and
    /// records the result as the same as the figure.
    const ISO_FIGURE_29: [&str; 33] = [
        "111111101100110010010010101111111",
        "100000100010111010111000101000001",
        "101110100000001101101100001011101",
        "101110101010000111000110001011101",
        "101110101101100011010010001011101",
        "100000101100010100001101101000001",
        "111111101010101010101010101111111",
        "000000001010000000011100100000000",
        "100010111100001100100011011111001",
        "100101000111001001000110000101100",
        "010001100011111010101000011011001",
        "101101011101010010000010010000000",
        "001111110011010110010011101001100",
        "011001000101001000111100110101001",
        "101001111001111101001111000110111",
        "100100010001000111100101111100000",
        "110010111101110000011110111111100",
        "010000010111100010001000010000111",
        "111111111111010101000110010001111",
        "001100010000000111100101010101110",
        "111101111011101000111001010010001",
        "100110000101001010010111000100001",
        "000110111110111010010001011001000",
        "001011010011101000011111011101111",
        "111011111000010111001001111110000",
        "000000001110110011111100100010100",
        "111111101000110100101000101010011",
        "100000100001010010001011100010000",
        "101110101111011010000010111111100",
        "101110100000111000111100000000101",
        "101110100101010100001000010110100",
        "100000100010110111000110101001001",
        "111111101101101011010000111100011",
    ];

    /// The digits of zint's version 2 symbol.
    const VERSION_2_DIGITS: &str = "12345678901234567890123456789012345678901";

    /// The symbol zint draws for 41 digits in numeric mode at level M:
    /// version 2, under mask 001, which it picks itself. From zint's
    /// `test_qr_encode`, item 21.
    const VERSION_2_BY_ZINT: [&str; 25] = [
        "1111111011001110101111111",
        "1000001001000000001000001",
        "1011101011001111101011101",
        "1011101001100000101011101",
        "1011101001101011001011101",
        "1000001010111110101000001",
        "1111111010101010101111111",
        "0000000001111100000000000",
        "1010001100000101000100101",
        "0111010101111001011000001",
        "0010011101000111110010011",
        "1001010100011011001100011",
        "1000101101100111010110101",
        "0001010000100111101011011",
        "1111101100010001011000110",
        "0000100001000101011010011",
        "1101001101011100111111101",
        "0000000011001001100010000",
        "1111111010101110101010001",
        "1000001000111011100010001",
        "1011101000100111111110110",
        "1011101001100110010101000",
        "1011101011110001000100111",
        "1000001000000100111010110",
        "1111111010011100001100111",
    ];

    /// The symbol zint draws for "1234567890" in numeric mode at version 1
    /// and level M: mask 001, which its test data says scores the same as
    /// mask 010. From zint's `test_qr_encode`, item 28.
    const TIE_BY_ZINT: [&str; 21] = [
        "111111101001101111111",
        "100000100100101000001",
        "101110101001001011101",
        "101110100101101011101",
        "101110100001101011101",
        "100000101101101000001",
        "111111101010101111111",
        "000000000000100000000",
        "101000110010000100101",
        "101010001111011101011",
        "111010101101110110010",
        "110111010101011100011",
        "110111110101111111001",
        "000000001010000000000",
        "111111101110001000010",
        "100000100000100010001",
        "101110100110001000111",
        "101110100111011001000",
        "101110101101110110111",
        "100000100001011000010",
        "111111101011111111111",
    ];

    /// The symbol the `qrcode` crate scores in its `penalty_tests`
    /// (`check_penalty_canvas`), written there as `#` and `.`.
    const PENALTY_GRID: [&str; 21] = [
        "111111101100001111111",
        "100000101001001000001",
        "101110101001101011101",
        "101110101000001011101",
        "101110101010001011101",
        "100000100010001000001",
        "111111101010101111111",
        "000000001000000000000",
        "011010110000101011111",
        "010000001111000010001",
        "001101110110001011000",
        "011011010011010101110",
        "100010101011101110101",
        "000000001101001000101",
        "111111101010000101100",
        "100000100101101101000",
        "101110101010001111111",
        "101110100101010100010",
        "101110101000111101001",
        "100000101011010001011",
        "111111100000111100001",
    ];

    /// Which masks turn over the module in a row, counted modulo 12, and a
    /// column, counted modulo 6: bit n is set when mask n does. zint's
    /// `qr_masks` in `backend/qr.h`, which it takes from BWIPP.
    const MASK_TABLE: [[u8; 6]; 12] = [
        [0xFF, 0x72, 0xF3, 0x6E, 0xE3, 0x62],
        [0x74, 0x51, 0x58, 0x85, 0x80, 0x89],
        [0xE7, 0x4A, 0x03, 0x76, 0xDB, 0x92],
        [0x6C, 0x81, 0x60, 0x9D, 0x70, 0x91],
        [0xF7, 0x92, 0xDB, 0x66, 0x03, 0x4A],
        [0x74, 0x99, 0x90, 0x85, 0x48, 0x41],
        [0xEF, 0x62, 0xE3, 0x7E, 0xF3, 0x72],
        [0x64, 0x41, 0x48, 0x95, 0x90, 0x99],
        [0xF7, 0x5A, 0x13, 0x66, 0xCB, 0x82],
        [0x7C, 0x91, 0x70, 0x8D, 0x60, 0x81],
        [0xE7, 0x82, 0xCB, 0x76, 0x13, 0x5A],
        [0x64, 0x89, 0x80, 0x95, 0x58, 0x51],
    ];

    /// Table 7 of the standard at level M: the bytes each version from 1
    /// to 15 holds in byte mode.
    const CAPACITIES: [usize; 15] = [
        14, 26, 42, 62, 84, 106, 122, 152, 180, 213, 251, 287, 331, 362, 412,
    ];

    /// Table 7 of the standard at level M: the data codewords of each
    /// version from 1 to 15.
    const DATA_CODEWORDS: [usize; 15] = [
        16, 28, 44, 64, 86, 108, 124, 154, 182, 216, 254, 290, 334, 365, 415,
    ];

    /// Table 9 of the standard at level M as the reader holds it: for each
    /// version from 1 to 15, how many error correction blocks there are and
    /// the error correction codewords in each.
    const BLOCKS: [(usize, usize); 15] = [
        (1, 10),
        (1, 16),
        (1, 26),
        (2, 18),
        (2, 24),
        (4, 16),
        (4, 18),
        (4, 22),
        (5, 22),
        (5, 26),
        (5, 30),
        (8, 22),
        (9, 22),
        (9, 24),
        (10, 24),
    ];

    // Small tools.

    /// A version of the table by its number.
    fn version(number: usize) -> &'static Version {
        &VERSIONS[number - 1]
    }

    /// A symbol from rows of 1 for dark and 0 for light.
    fn grid(rows: &[&str]) -> Matrix {
        Matrix {
            rows: rows.iter().copied().map(line).collect(),
        }
    }

    /// One row or column from 1 for dark and 0 for light.
    fn line(text: &str) -> Vec<bool> {
        text.bytes().map(|module| module == b'1').collect()
    }

    /// A symbol as rows of 1 for dark and 0 for light, so that a failed
    /// comparison prints two pictures.
    fn picture(symbol: &Matrix) -> Vec<String> {
        symbol
            .rows
            .iter()
            .map(|modules| {
                modules
                    .iter()
                    .map(|&dark| if dark { '1' } else { '0' })
                    .collect()
            })
            .collect()
    }

    /// Rows written as literals, in the form `picture` gives.
    fn rows_of(rows: &[&str]) -> Vec<String> {
        rows.iter().copied().map(str::to_owned).collect()
    }

    /// The part of a symbol's picture that starts in row `top` and column
    /// `left`.
    fn crop(symbol: &Matrix, top: usize, left: usize, height: usize, width: usize) -> Vec<String> {
        picture(symbol)[top..top + height]
            .iter()
            .map(|modules| modules[left..left + width].to_owned())
            .collect()
    }

    /// A symbol's columns, each from the top down.
    fn columns_of(symbol: &Matrix) -> Vec<Vec<bool>> {
        (0..symbol.rows.len())
            .map(|column| symbol.rows.iter().map(|modules| modules[column]).collect())
            .collect()
    }

    /// A payload of `len` bytes that differ from their neighbours.
    fn sample(len: usize) -> Vec<u8> {
        (0..len)
            .map(|index| u8::try_from((index * 7 + len) % 251).unwrap())
            .collect()
    }

    /// Every place of a square of `side` modules, row by row.
    fn square(side: usize) -> impl Iterator<Item = (usize, usize)> {
        (0..side).flat_map(move |down| (0..side).map(move |across| (down, across)))
    }

    /// The penalty of a symbol under each mask, for a failed comparison to
    /// print.
    fn penalties(version: &Version, data: &[u8]) -> Vec<usize> {
        candidates(version, data).iter().map(penalty).collect()
    }

    /// What one of the rules for a row or a column scores over all of
    /// `lines`.
    fn along(score: fn(&[bool]) -> usize, lines: &[Vec<bool>]) -> usize {
        lines.iter().map(|modules| score(modules.as_slice())).sum()
    }

    // A second writer of bit streams, for the modes the encoder does not
    // have and as a check on the one it has. It shares no code with the
    // encoder.

    /// Packs bit fields, each a value and its width, most significant bit
    /// first, fills the last octet with light bits, and adds the pad
    /// codewords in turn up to `len` octets (clauses 7.4.9 and 7.4.10).
    fn stream(fields: &[(u32, u32)], len: usize) -> Vec<u8> {
        let bits: Vec<bool> = fields
            .iter()
            .flat_map(|&(value, width)| (0..width).rev().map(move |bit| (value >> bit) & 1 == 1))
            .collect();
        let mut octets: Vec<u8> = bits
            .chunks(8)
            .map(|octet| {
                let value = octet
                    .iter()
                    .fold(0_u8, |value, &bit| (value << 1) | u8::from(bit));
                value << (8 - octet.len())
            })
            .collect();
        let pads = len - octets.len();
        octets.extend([0xEC, 0x11].into_iter().cycle().take(pads));
        octets
    }

    /// The fields of digits in numeric mode, with the terminator, for
    /// versions 1 to 9: three digits in ten bits, and a last two in seven
    /// or a last one in four (clause 7.4.3).
    fn numeric(digits: &str) -> Vec<(u32, u32)> {
        let groups = digits.as_bytes().chunks(3).map(|group| {
            let value = group
                .iter()
                .fold(0, |value, &digit| value * 10 + u32::from(digit - b'0'));
            (value, [4, 7, 10][group.len() - 1])
        });
        let count = u32::try_from(digits.len()).unwrap();
        [(0b0001, 4), (count, 10)]
            .into_iter()
            .chain(groups)
            .chain([(0, 4)])
            .collect()
    }

    /// The fields of an even number of characters in alphanumeric mode,
    /// with the terminator, for versions 1 to 9: two characters in eleven
    /// bits (clause 7.4.4).
    fn alphanumeric(text: &str) -> Vec<(u32, u32)> {
        const SYMBOLS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
        let value = |symbol: u8| {
            let place = SYMBOLS.iter().position(|&known| known == symbol);
            u32::try_from(place.unwrap()).unwrap()
        };
        let pairs = text
            .as_bytes()
            .chunks(2)
            .map(|pair| (45 * value(pair[0]) + value(pair[1]), 11));
        let count = u32::try_from(text.len()).unwrap();
        [(0b0010, 4), (count, 9)]
            .into_iter()
            .chain(pairs)
            .chain([(0, 4)])
            .collect()
    }

    /// The fields of a payload in byte mode, its length in `count_bits`
    /// bits, with the terminator (clause 7.4.5).
    fn bytes(payload: &[u8], count_bits: u32) -> Vec<(u32, u32)> {
        let count = u32::try_from(payload.len()).unwrap();
        let octets = payload.iter().map(|&octet| (u32::from(octet), 8));
        [(0b0100, 4), (count, count_bits)]
            .into_iter()
            .chain(octets)
            .chain([(0, 4)])
            .collect()
    }

    // A reader, written from the standard for these tests. It shares no
    // code with the encoder: it paints the function patterns where the
    // encoder asks about one module at a time, works out the alignment
    // patterns' places by rule where the encoder has a table, takes the
    // masks from zint's table where the encoder has formulas, checks the
    // format and version information by dividing where the encoder has
    // tables, and checks the error correction codewords by evaluating
    // where the encoder divides.

    /// The field the codewords are in, as Annex A of the standard prints
    /// it: by the powers of 2 and their logarithms.
    struct Field {
        /// 2 to the power of each number from 0 to 254.
        powers: Vec<u8>,
        /// The power of 2 that each element but 0 is.
        logarithms: Vec<usize>,
    }

    impl Field {
        /// Builds the tables by doubling.
        fn new() -> Self {
            let mut powers = vec![1_u8];
            for _ in 1..255 {
                let last = powers[powers.len() - 1];
                powers.push(if last < 0x80 {
                    last << 1
                } else {
                    (last << 1) ^ 0x1D
                });
            }
            // 0 is no power of 2; its entry is never read.
            let logarithms = (0..=u8::MAX)
                .map(|element| {
                    let exponent = powers.iter().position(|&power| power == element);
                    exponent.unwrap_or(0)
                })
                .collect();
            Self { powers, logarithms }
        }

        /// The product of two elements, by adding their logarithms.
        fn times(&self, left: u8, right: u8) -> u8 {
            if left == 0 || right == 0 {
                0
            } else {
                let exponent =
                    self.logarithms[usize::from(left)] + self.logarithms[usize::from(right)];
                self.powers[exponent % 255]
            }
        }

        /// The value, at 2 to the power of `exponent`, of the polynomial
        /// whose coefficients are `coefficients`, highest power first.
        fn value_at(&self, coefficients: &[u8], exponent: usize) -> u8 {
            coefficients.iter().fold(0, |sum, &coefficient| {
                self.times(sum, self.powers[exponent]) ^ coefficient
            })
        }
    }

    /// What the reader expects of a module before it looks at a symbol.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Cell {
        /// Part of a function pattern, with the colour it must have.
        Pattern(bool),
        /// Format or version information.
        Information,
        /// A module of the encoding region.
        Data,
    }

    /// Sets one module of the reader's expectation.
    fn paint(cells: &mut [Vec<Cell>], row: usize, column: usize, cell: Cell) {
        cells[row][column] = cell;
    }

    /// The rows, which are also the columns, of the centres of a version's
    /// alignment patterns, by the rule behind the standard's table: the
    /// last is seven modules from the far edge, the first is 6, and the
    /// others are spaced evenly, an even number of modules apart.
    fn alignment_centres(version: usize) -> Vec<usize> {
        if version == 1 {
            return Vec::new();
        }
        let count = version / 7 + 2;
        let step = (version * 4 + count * 2 + 1) / (count * 2 - 2) * 2;
        let far = 4 * version + 10;
        let mut centres: Vec<usize> = (0..count - 1).map(|index| far - index * step).collect();
        centres.push(6);
        centres
    }

    /// Where each bit of the format information is written, least
    /// significant first: its place around the top-left finder pattern
    /// and its place beside one of the other two.
    fn format_places(size: usize) -> Vec<[(usize, usize); 2]> {
        (0..15)
            .map(|bit| {
                let near = match bit {
                    0..=5 => (bit, 8),
                    6 => (7, 8),
                    7 => (8, 8),
                    8 => (8, 7),
                    _ => (8, 14 - bit),
                };
                let apart = if bit < 8 {
                    (8, size - 1 - bit)
                } else {
                    (size - 15 + bit, 8)
                };
                [near, apart]
            })
            .collect()
    }

    /// Where each bit of the version information is written, least
    /// significant first: its place beside the top-right finder pattern
    /// and its place beside the bottom-left one.
    fn version_places(size: usize) -> Vec<[(usize, usize); 2]> {
        (0..18)
            .map(|bit| {
                let (long, short) = (bit / 3, size - 11 + bit % 3);
                [(long, short), (short, long)]
            })
            .collect()
    }

    /// The reader's drawing of a version's symbol before any data: the
    /// function patterns with their colours, and the places kept for the
    /// format and version information.
    fn layout(version: usize) -> Vec<Vec<Cell>> {
        const FINDER: [&str; 7] = [
            "1111111", "1000001", "1011101", "1011101", "1011101", "1000001", "1111111",
        ];
        const ALIGNMENT: [&str; 5] = ["11111", "10001", "10101", "10001", "11111"];
        let size = 17 + 4 * version;
        // The timing patterns run the length of row 6 and of column 6; what
        // is painted afterwards covers their ends.
        let mut cells: Vec<Vec<Cell>> = (0..size)
            .map(|row| {
                (0..size)
                    .map(|column| {
                        if row == 6 {
                            Cell::Pattern(column % 2 == 0)
                        } else if column == 6 {
                            Cell::Pattern(row % 2 == 0)
                        } else {
                            Cell::Data
                        }
                    })
                    .collect()
            })
            .collect();
        // A finder pattern sits in a corner square of eight modules. The
        // row and the column of that square that face inwards are its
        // light separator.
        for (top, left, inset_down, inset_across) in
            [(0, 0, 0, 0), (0, size - 8, 0, 1), (size - 8, 0, 1, 0)]
        {
            for (down, across) in square(8) {
                paint(&mut cells, top + down, left + across, Cell::Pattern(false));
            }
            for (down, across) in square(7) {
                let dark = FINDER[down].as_bytes()[across] == b'1';
                let (row, column) = (top + inset_down + down, left + inset_across + across);
                paint(&mut cells, row, column, Cell::Pattern(dark));
            }
        }
        let centres = alignment_centres(version);
        for &centre_row in &centres {
            for &centre_column in &centres {
                // Three crossings fall on finder patterns and hold no
                // alignment pattern.
                let on_a_finder =
                    [(6, 6), (6, size - 7), (size - 7, 6)].contains(&(centre_row, centre_column));
                if !on_a_finder {
                    for (down, across) in square(5) {
                        let dark = ALIGNMENT[down].as_bytes()[across] == b'1';
                        let (row, column) = (centre_row - 2 + down, centre_column - 2 + across);
                        paint(&mut cells, row, column, Cell::Pattern(dark));
                    }
                }
            }
        }
        paint(&mut cells, size - 8, 8, Cell::Pattern(true));
        for (row, column) in format_places(size).into_iter().flatten() {
            paint(&mut cells, row, column, Cell::Information);
        }
        if version >= 7 {
            for (row, column) in version_places(size).into_iter().flatten() {
                paint(&mut cells, row, column, Cell::Information);
            }
        }
        cells
    }

    /// The word a symbol shows in one of the two places of its format or
    /// version information.
    fn shown(symbol: &Matrix, places: &[[(usize, usize); 2]], copy: usize) -> u32 {
        places.iter().enumerate().fold(0, |word, (bit, place)| {
            let (row, column) = place[copy];
            word | (u32::from(symbol.rows[row][column]) << bit)
        })
    }

    /// What is left of one polynomial with coefficients 0 and 1 after
    /// dividing it by another, each written as the bits of a number: the
    /// check on the format and the version information (Annexes C and D).
    fn remainder(dividend: u32, divisor: u32) -> u32 {
        let degree = divisor.ilog2();
        (degree..32).rev().fold(dividend, |rest, bit| {
            if (rest >> bit) & 1 == 1 {
                rest ^ (divisor << (bit - degree))
            } else {
                rest
            }
        })
    }

    /// The mask a symbol's format information names, once both places
    /// agree, the word is one its code allows and it names level M.
    fn read_format(symbol: &Matrix) -> usize {
        let places = format_places(symbol.rows.len());
        let word = shown(symbol, &places, 0);
        assert_eq!(
            word,
            shown(symbol, &places, 1),
            "the two copies of the format information differ"
        );
        let plain = word ^ 0x5412;
        assert_eq!(
            remainder(plain, 0x537),
            0,
            "the format information is not a word of its code"
        );
        assert_eq!(plain >> 13, 0b00, "the symbol is not at level M");
        usize::try_from((plain >> 10) & 0b111).unwrap()
    }

    /// Checks that both places of a symbol's version information agree,
    /// hold a word its code allows and name the version.
    fn check_version(symbol: &Matrix, version: usize) {
        let places = version_places(symbol.rows.len());
        let word = shown(symbol, &places, 0);
        assert_eq!(
            word,
            shown(symbol, &places, 1),
            "the two copies of the version information differ"
        );
        assert_eq!(
            remainder(word, 0x1F25),
            0,
            "the version information is not a word of its code"
        );
        assert_eq!(
            usize::try_from(word >> 12).unwrap(),
            version,
            "the version information names another version"
        );
    }

    /// Every codeword a symbol carries, in the order they were placed and
    /// with the mask taken off.
    fn read_codewords(symbol: &Matrix, cells: &[Vec<Cell>], mask: usize) -> Vec<u8> {
        let size = cells.len();
        // The right column of each pair of columns, from the right edge,
        // stepping over the timing column.
        let mut pairs: Vec<usize> = (7..size).rev().step_by(2).collect();
        pairs.extend([5, 3, 1]);
        let mut bits = Vec::new();
        for (turn, &right) in pairs.iter().enumerate() {
            for step in 0..size {
                let row = if turn % 2 == 0 { size - 1 - step } else { step };
                for column in [right, right - 1] {
                    if cells[row][column] == Cell::Data {
                        let turned = (MASK_TABLE[row % 12][column % 6] >> mask) & 1 == 1;
                        bits.push(symbol.rows[row][column] != turned);
                    }
                }
            }
        }
        let (whole, spare) = bits.split_at(bits.len() / 8 * 8);
        assert!(
            spare.iter().all(|&bit| !bit),
            "the remainder bits are not light"
        );
        whole
            .chunks(8)
            .map(|octet| {
                octet
                    .iter()
                    .fold(0, |value, &bit| (value << 1) | u8::from(bit))
            })
            .collect()
    }

    /// The data codewords among a symbol's codewords, once the blocks are
    /// taken apart and each has passed its error correction check: its
    /// polynomial is zero at the first powers of 2, one for each error
    /// correction codeword.
    fn data_of(version: usize, codewords: &[u8]) -> Vec<u8> {
        let (count, correction) = BLOCKS[version - 1];
        let data = codewords.len() - count * correction;
        let (short, longer) = (data / count, data % count);
        // The codewords come in rounds, one from each block: `short` rounds
        // of data, a round in which only the last `longer` blocks give one
        // more, and then the rounds of error correction codewords.
        let first_longer = count - longer;
        let blocks: Vec<Vec<u8>> = (0..count)
            .map(|number| {
                let whole = (0..short).map(|round| round * count + number);
                let extra = (short..short + usize::from(number >= first_longer))
                    .map(|round| round * count + number - first_longer);
                let checks = (0..correction).map(|round| data + round * count + number);
                whole
                    .chain(extra)
                    .chain(checks)
                    .map(|place| codewords[place])
                    .collect()
            })
            .collect();
        let field = Field::new();
        for block in &blocks {
            assert!(
                (0..correction).all(|exponent| field.value_at(block, exponent) == 0),
                "a block fails its error correction check"
            );
        }
        blocks
            .iter()
            .flat_map(|block| block.iter().copied().take(block.len() - correction))
            .collect()
    }

    /// The payload in a version's data codewords: one run of bytes, the
    /// terminator, and pad codewords to the end.
    fn payload_of(version: usize, data: &[u8]) -> Vec<u8> {
        let bits: Vec<bool> = data
            .iter()
            .flat_map(|&codeword| (0..8).rev().map(move |bit| (codeword >> bit) & 1 == 1))
            .collect();
        let read = |from: usize, width: usize| {
            bits[from..from + width]
                .iter()
                .fold(0_usize, |value, &bit| (value << 1) | usize::from(bit))
        };
        let count_bits = if version < 10 { 8 } else { 16 };
        assert_eq!(read(0, 4), 0b0100, "the data is not in byte mode");
        let len = read(4, count_bits);
        let start = 4 + count_bits;
        let payload: Vec<u8> = (0..len)
            .map(|index| u8::try_from(read(start + 8 * index, 8)).unwrap())
            .collect();
        let end = start + 8 * len;
        assert_eq!(read(end, 4), 0, "the terminator is not four light bits");
        let pads: Vec<usize> = (end + 4..bits.len())
            .step_by(8)
            .map(|from| read(from, 8))
            .collect();
        let expected: Vec<usize> = [0xEC, 0x11].into_iter().cycle().take(pads.len()).collect();
        assert_eq!(pads, expected, "the pad codewords are not 0xEC and 0x11");
        payload
    }

    /// A symbol's version and data codewords, read the way a scanner
    /// reads them, with every part of the symbol checked on the way.
    fn read_data(symbol: &Matrix) -> (usize, Vec<u8>) {
        let size = symbol.rows.len();
        assert!(
            size >= 21 && size % 4 == 1,
            "no version is {size} modules a side"
        );
        assert!(
            symbol.rows.iter().all(|modules| modules.len() == size),
            "the symbol is not square"
        );
        let version = (size - 17) / 4;
        let cells = layout(version);
        for (row, column) in square(size) {
            if let Cell::Pattern(dark) = cells[row][column] {
                assert_eq!(
                    symbol.rows[row][column], dark,
                    "the function pattern at row {row}, column {column}"
                );
            }
        }
        let mask = read_format(symbol);
        if version >= 7 {
            check_version(symbol, version);
        }
        let codewords = read_codewords(symbol, &cells, mask);
        (version, data_of(version, &codewords))
    }

    /// The payload of a symbol.
    fn decode(symbol: &Matrix) -> Vec<u8> {
        let (version, data) = read_data(symbol);
        payload_of(version, &data)
    }

    /// The standard's first figure with one module turned over.
    fn damaged(row: usize, column: usize) -> Matrix {
        let mut symbol = grid(&ISO_FIGURE_1);
        symbol.rows[row][column] ^= true;
        symbol
    }

    // The tools, tested against the standard.

    #[test]
    fn the_reader_s_field_is_the_one_annex_a_prints() {
        let field = Field::new();
        assert_eq!(
            field.powers[..12],
            [1, 2, 4, 8, 16, 32, 64, 128, 29, 58, 116, 232]
        );
        assert_eq!((field.powers[25], field.powers[254]), (3, 142));
        let mut elements = field.powers.clone();
        elements.sort_unstable();
        elements.dedup();
        assert_eq!(elements.len(), 255);
        assert_eq!(field.times(142, 2), 1);
        assert_eq!(field.times(0, 2), 0);
    }

    #[test]
    fn the_reader_divides_as_annexes_c_and_d_do() {
        // Annex C: level M with mask 101 is 00101, and its ten check bits
        // are 0011011100.
        assert_eq!(remainder(0b00101 << 10, 0x537), 0b00_1101_1100);
        // Annex D: version 7 is 000111, and its twelve check bits are
        // 110010010100.
        assert_eq!(remainder(7 << 12, 0x1F25), 0b1100_1001_0100);
    }

    #[test]
    fn the_reader_places_alignment_patterns_as_annex_e_does() {
        // Table E.1, versions 1 to 15, each row from the far edge inwards.
        let table: [&[usize]; 15] = [
            &[],
            &[18, 6],
            &[22, 6],
            &[26, 6],
            &[30, 6],
            &[34, 6],
            &[38, 22, 6],
            &[42, 24, 6],
            &[46, 26, 6],
            &[50, 28, 6],
            &[54, 30, 6],
            &[58, 32, 6],
            &[62, 34, 6],
            &[66, 46, 26, 6],
            &[70, 48, 26, 6],
        ];
        for (number, centres) in (1..).zip(table) {
            assert_eq!(alignment_centres(number), centres, "version {number}");
        }
    }

    #[test]
    fn the_second_writer_writes_the_standard_s_examples() {
        // Annex I: "01234567" in numeric mode at version 1-M.
        assert_eq!(stream(&numeric("01234567"), 16), ANNEX_I_DATA);
        // Clause 7.4.4: "AC-42" is 00111001110, 11100111001 and a last
        // character on its own; the first four characters are the pairs.
        assert_eq!(
            alphanumeric("AC-4"),
            [
                (0b0010, 4),
                (4, 9),
                (0b001_1100_1110, 11),
                (0b111_0011_1001, 11),
                (0, 4)
            ]
        );
        assert_eq!(
            bytes(b"\x00\xFF", 8),
            [(0b0100, 4), (2, 8), (0x00, 8), (0xFF, 8), (0, 4)]
        );
        // Twelve bits are an octet and a half; the pad codewords follow.
        assert_eq!(
            stream(&[(0b1010_0101_1111, 12)], 4),
            [0xA5, 0xF0, 0xEC, 0x11]
        );
    }

    #[test]
    fn the_reader_reads_the_standard_s_figures() {
        assert_eq!(decode(&grid(&ISO_FIGURE_1)), FIGURE_1_TEXT);
        assert_eq!(decode(&grid(&FIGURE_1_TEXT_BY_ZINT)), FIGURE_1_TEXT);
        assert_eq!(read_data(&grid(&ISO_ANNEX_I)), (1, ANNEX_I_DATA.to_vec()));
        assert_eq!(
            read_data(&grid(&ISO_FIGURE_29)),
            (4, stream(&alphanumeric(FIGURE_29_TEXT), 64))
        );
        assert_eq!(
            read_data(&grid(&VERSION_2_BY_ZINT)),
            (2, stream(&numeric(VERSION_2_DIGITS), 28))
        );
        assert_eq!(
            read_data(&grid(&TIE_BY_ZINT)),
            (1, stream(&numeric("1234567890"), 16))
        );
    }

    #[test]
    #[should_panic(expected = "a block fails its error correction check")]
    fn the_reader_refuses_a_changed_data_module() {
        let _ = decode(&damaged(20, 20));
    }

    #[test]
    #[should_panic(expected = "the function pattern at row 3, column 3")]
    fn the_reader_refuses_a_changed_function_pattern() {
        let _ = decode(&damaged(3, 3));
    }

    #[test]
    #[should_panic(expected = "the two copies of the format information differ")]
    fn the_reader_refuses_changed_format_information() {
        let _ = decode(&damaged(8, 0));
    }

    // The encoder, a stage at a time.

    #[test]
    fn multiplies_in_the_standard_s_field() {
        // x^8 is x^4 + x^3 + x^2 + 1.
        assert_eq!(multiply(0x80, 2), 0x1D);
        assert_eq!(multiply(2, 0x80), 0x1D);
        // x + 1, squared, is x^2 + 1.
        assert_eq!(multiply(3, 3), 5);
        // Annex A: 2 to the 254th is 142, and the 255th is 1 again.
        assert_eq!(multiply(142, 2), 1);
        assert_eq!(multiply(0xFF, 1), 0xFF);
        let field = Field::new();
        for (left, right) in square(256) {
            let (left, right) = (u8::try_from(left).unwrap(), u8::try_from(right).unwrap());
            assert_eq!(
                multiply(left, right),
                field.times(left, right),
                "{left} times {right}"
            );
        }
    }

    #[test]
    fn the_generator_for_ten_codewords_is_the_one_annex_a_prints() {
        // Annex A gives the powers of 2: 0, 251, 67, 46, 61, 118, 70, 64,
        // 94, 32 and 45. These are the elements they stand for.
        assert_eq!(
            generator(10),
            [1, 216, 194, 159, 111, 199, 94, 95, 113, 157, 193]
        );
    }

    #[test]
    fn a_generator_has_the_first_powers_of_two_as_its_roots() {
        let field = Field::new();
        for degree in [7, 10, 13, 16, 18, 22, 24, 26, 30] {
            let divisor = generator(degree);
            assert_eq!(divisor.len(), degree + 1, "degree {degree}");
            assert_eq!(divisor.first(), Some(&1), "degree {degree}");
            let roots: Vec<usize> = (0..=degree)
                .filter(|&exponent| field.value_at(&divisor, exponent) == 0)
                .collect();
            assert_eq!(roots, (0..degree).collect::<Vec<_>>(), "degree {degree}");
        }
    }

    #[test]
    fn works_out_the_error_correction_codewords_of_published_blocks() {
        // The standard's worked example, Annex I.
        assert_eq!(
            correction_codewords(&ANNEX_I_DATA, &generator(10)),
            ANNEX_I_CORRECTION
        );
        // "HELLO WORLD" at version 1-M and 1-Q: the worked example of
        // thonky.com's tutorial, as the `qrcode` crate's `test_poly_mod_1`
        // and `test_poly_mod_2` record it.
        let hello = [
            0x20, 0x5B, 0x0B, 0x78, 0xD1, 0x72, 0xDC, 0x4D, 0x43, 0x40, 0xEC, 0x11, 0xEC, 0x11,
            0xEC, 0x11,
        ];
        assert_eq!(
            correction_codewords(&hello, &generator(10)),
            [0xC4, 0x23, 0x27, 0x77, 0xEB, 0xD7, 0xE7, 0xE2, 0x5D, 0x17]
        );
        assert_eq!(
            correction_codewords(&hello[..13], &generator(13)),
            [
                0xA8, 0x48, 0x16, 0x52, 0xD9, 0x36, 0x9C, 0x00, 0x2E, 0x0F, 0xB4, 0x7A, 0x10
            ]
        );
    }

    #[test]
    fn cuts_the_data_into_blocks_and_reads_them_in_turn() {
        // The version 5-Q example of thonky.com's tutorial, as the `qrcode`
        // crate's `test_add_ec_complex` and `test_poly_mod_3` record it:
        // two blocks of 15 data codewords, then two of 16, each with 18
        // error correction codewords.
        const DATA: [&[u8]; 4] = [
            &[67, 85, 70, 134, 87, 38, 85, 194, 119, 50, 6, 18, 6, 103, 38],
            &[
                246, 246, 66, 7, 118, 134, 242, 7, 38, 86, 22, 198, 199, 146, 6,
            ],
            &[
                182, 230, 247, 119, 50, 7, 118, 134, 87, 38, 82, 6, 134, 151, 50, 7,
            ],
            &[
                70, 247, 118, 86, 194, 6, 151, 50, 16, 236, 17, 236, 17, 236, 17, 236,
            ],
        ];
        const CORRECTION: [&[u8]; 4] = [
            &[
                213, 199, 11, 45, 115, 247, 241, 223, 229, 248, 154, 117, 154, 111, 86, 161, 111,
                39,
            ],
            &[
                87, 204, 96, 60, 202, 182, 124, 157, 200, 134, 27, 129, 209, 17, 163, 163, 120, 133,
            ],
            &[
                148, 116, 177, 212, 76, 133, 75, 242, 238, 76, 195, 230, 189, 10, 108, 240, 192,
                141,
            ],
            &[
                235, 159, 5, 173, 24, 147, 59, 33, 106, 40, 255, 172, 82, 2, 131, 32, 178, 236,
            ],
        ];
        const IN_TURN: [u8; 134] = [
            67, 246, 182, 70, 85, 246, 230, 247, 70, 66, 247, 118, 134, 7, 119, 86, 87, 118, 50,
            194, 38, 134, 7, 6, 85, 242, 118, 151, 194, 7, 134, 50, 119, 38, 87, 16, 50, 86, 38,
            236, 6, 22, 82, 17, 18, 198, 6, 236, 6, 199, 134, 17, 103, 146, 151, 236, 38, 6, 50,
            17, 7, 236, 213, 87, 148, 235, 199, 204, 116, 159, 11, 96, 177, 5, 45, 60, 212, 173,
            115, 202, 76, 24, 247, 182, 133, 147, 241, 124, 75, 59, 223, 157, 242, 33, 229, 200,
            238, 106, 248, 134, 76, 40, 154, 27, 195, 255, 117, 129, 230, 172, 154, 209, 189, 82,
            111, 17, 10, 2, 86, 163, 108, 131, 161, 163, 240, 32, 111, 120, 192, 178, 39, 133, 141,
            236,
        ];
        for (block, correction) in DATA.into_iter().zip(CORRECTION) {
            assert_eq!(correction_codewords(block, &generator(18)), correction);
        }
        assert_eq!(interleave(&DATA.concat(), &[(2, 15), (2, 16)], 18), IN_TURN);
    }

    #[test]
    fn writes_the_mode_the_length_the_bytes_and_the_terminator() {
        // 0100, then 14 as 00001110, then the bytes: every codeword holds
        // the low half of one octet and the high half of the next, and the
        // terminator fills the last. Version 1 holds exactly these 16.
        assert_eq!(
            data_codewords(FIGURE_1_TEXT, version(1)),
            [
                0x40, 0xE5, 0x15, 0x22, 0x04, 0x36, 0xF6, 0x46, 0x52, 0x05, 0x37, 0x96, 0xD6, 0x26,
                0xF6, 0xC0
            ]
        );
        // The standard's figure carries them.
        assert_eq!(
            read_data(&grid(&ISO_FIGURE_1)).1,
            data_codewords(FIGURE_1_TEXT, version(1))
        );
        // A shorter payload is followed by the pad codewords in turn.
        assert_eq!(
            data_codewords(b"A", version(1)),
            [
                0x40, 0x14, 0x10, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC,
                0x11, 0xEC
            ]
        );
        assert_eq!(
            data_codewords(b"", version(1)),
            [
                0x40, 0x00, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
                0xEC, 0x11
            ]
        );
    }

    #[test]
    fn writes_the_length_in_eight_bits_up_to_version_9_and_in_sixteen_after() {
        for (number, (most, len)) in (1..).zip(CAPACITIES.into_iter().zip(DATA_CODEWORDS)) {
            let count_bits = if number < 10 { 8 } else { 16 };
            for held in [most / 2, most] {
                let payload = sample(held);
                assert_eq!(
                    data_codewords(&payload, version(number)),
                    stream(&bytes(&payload, count_bits), len),
                    "version {number}, {held} bytes"
                );
            }
        }
    }

    #[test]
    fn each_mask_turns_over_the_modules_the_standard_says() {
        for (number, mask) in MASKS.iter().enumerate() {
            for (row, column) in square(77) {
                assert_eq!(
                    (mask.flips)(row, column),
                    (MASK_TABLE[row % 12][column % 6] >> number) & 1 == 1,
                    "mask {number} at row {row}, column {column}"
                );
            }
        }
    }

    #[test]
    fn each_mask_carries_the_format_information_of_level_m() {
        let words: Vec<u16> = MASKS.iter().map(|mask| mask.format).collect();
        // Table C.1: level M is 00, then the mask from 000 to 111.
        assert_eq!(
            words,
            [
                0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0
            ]
        );
        // Annex I: level M with mask 010 is 101111001111100.
        assert_eq!(words[2], 0b101_1110_0111_1100);
        for (number, word) in (0_u32..).zip(words) {
            let plain = u32::from(word) ^ 0x5412;
            assert_eq!(plain >> 10, number, "mask {number}");
            assert_eq!(remainder(plain, 0x537), 0, "mask {number}");
        }
    }

    #[test]
    fn scores_runs_of_five_or_more_modules_in_one_colour() {
        let cases = [
            ("", 0),
            ("1111", 0),
            ("0000", 0),
            ("0101010101", 0),
            ("11110000", 0),
            ("11111", 3),
            ("00000", 3),
            ("111111", 4),
            ("0000000", 5),
            ("011111", 3),
            ("1111100000", 6),
            ("111110111110", 6),
            ("111111111111111111111", 19),
        ];
        for (text, score) in cases {
            assert_eq!(run_penalty(&line(text)), score, "{text}");
        }
    }

    #[test]
    fn scores_each_square_of_four_modules_in_one_colour() {
        let cases: [(&[&str], usize); 12] = [
            (&[], 0),
            (&["0"], 0),
            (&["01", "10"], 0),
            (&["00", "11"], 0),
            (&["01", "01"], 0),
            (&["00", "01"], 0),
            (&["10", "00"], 0),
            (&["00", "00"], 3),
            (&["11", "11"], 3),
            (&["000", "000"], 6),
            (&["111", "111", "111"], 12),
            (&["0000", "0110", "0110", "0000"], 3),
        ];
        for (rows, score) in cases {
            assert_eq!(block_penalty(&grid(rows).rows), score, "{rows:?}");
        }
    }

    #[test]
    fn scores_a_finder_like_pattern_with_four_light_modules_on_one_side() {
        let cases = [
            ("", 0),
            ("10111", 0),
            ("000000000000000", 0),
            ("111111111111111", 0),
            // Dark on both sides.
            ("110111011", 0),
            // Three light modules and then a dark one, on both sides.
            ("100010111010001", 0),
            // The margin is light: alone, at the start and at the end.
            ("1011101", 40),
            ("10111011", 40),
            ("11011101", 40),
            // Three light modules and then the margin.
            ("0001011101", 40),
            // Four light modules after, then before, with a dark module on
            // the other side.
            ("110111010000", 40),
            ("000010111011", 40),
            // Light on both sides scores once.
            ("000010111010000", 40),
            // Two patterns that share three modules.
            ("10111011101", 80),
        ];
        for (text, score) in cases {
            assert_eq!(finder_penalty(&line(text)), score, "{text}");
        }
    }

    #[test]
    fn scores_each_whole_five_per_cent_away_from_half_dark() {
        let cases = [
            (50, 0),
            (46, 0),
            (54, 0),
            (45, 10),
            (55, 10),
            (41, 10),
            (59, 10),
            (40, 20),
            (60, 20),
            (36, 20),
            (35, 30),
            (0, 100),
            (100, 100),
        ];
        for (dark, score) in cases {
            // Ten rows of ten modules, the first `dark` of them dark.
            let rows: Vec<Vec<bool>> = (0..10)
                .map(|row| (0..10).map(|column| row * 10 + column < dark).collect())
                .collect();
            assert_eq!(balance_penalty(&rows), score, "{dark} dark of 100");
        }
        assert_eq!(balance_penalty(&[]), 0);
    }

    #[test]
    fn scores_a_symbol_as_the_qrcode_crate_does() {
        // The crate's `penalty_tests` give 88 and 92 for runs along rows
        // and along columns, 90 for squares, and 0 and 40 for finder-like
        // patterns, after taking off the 360 that the three finder
        // patterns score in each direction. Its fourth rule is not the
        // standard's; 217 dark modules of 441 are within 5 % of half.
        let symbol = grid(&PENALTY_GRID);
        let columns = columns_of(&symbol);
        assert_eq!(along(run_penalty, &symbol.rows), 88);
        assert_eq!(along(run_penalty, &columns), 92);
        assert_eq!(block_penalty(&symbol.rows), 90);
        assert_eq!(along(finder_penalty, &symbol.rows), 360);
        assert_eq!(along(finder_penalty, &columns), 400);
        assert_eq!(balance_penalty(&symbol.rows), 0);
        assert_eq!(penalty(&symbol), 88 + 92 + 90 + 360 + 400);
    }

    #[test]
    fn adds_the_four_rules_over_rows_and_columns() {
        // Four dark modules: one square, and all dark is ten steps of 5 %
        // from half.
        assert_eq!(penalty(&grid(&["11", "11"])), 3 + 100);
        // A finder-like pattern down the first column and nothing else.
        // Runs: the rows' light runs of six and seven score 30, and six
        // light columns score 30. Squares: the 30 that keep clear of the
        // first column, 90. The pattern, 40, is seen only down the column.
        // Five dark modules of 49 are seven steps from half, 70.
        let column = [
            "1000000", "0000000", "1000000", "1000000", "1000000", "0000000", "1000000",
        ];
        assert_eq!(penalty(&grid(&column)), 60 + 90 + 40 + 70);
        // The same pattern along the first row scores the same.
        let row = [
            "1011101", "0000000", "0000000", "0000000", "0000000", "0000000", "0000000",
        ];
        assert_eq!(penalty(&grid(&row)), 60 + 90 + 40 + 70);
    }

    // The encoder, whole.

    #[test]
    fn draws_the_worked_example_of_annex_i() {
        let scores = penalties(version(1), &ANNEX_I_DATA);
        assert_eq!(
            picture(&symbol(version(1), &ANNEX_I_DATA)),
            ISO_ANNEX_I,
            "penalties by mask: {scores:?}"
        );
    }

    #[test]
    fn draws_the_worked_example_under_its_mask() {
        assert_eq!(
            candidates(version(1), &ANNEX_I_DATA).get(2).map(picture),
            Some(rows_of(&ISO_ANNEX_I))
        );
    }

    #[test]
    fn draws_the_standard_s_first_figure_under_its_mask() {
        let data = data_codewords(FIGURE_1_TEXT, version(1));
        assert_eq!(
            candidates(version(1), &data).get(5).map(picture),
            Some(rows_of(&ISO_FIGURE_1))
        );
    }

    #[test]
    fn encodes_the_text_of_the_standard_s_first_figure_as_zint_does() {
        let scores = penalties(version(1), &data_codewords(FIGURE_1_TEXT, version(1)));
        assert_eq!(
            encode(FIGURE_1_TEXT).map(|symbol| picture(&symbol)),
            Ok(rows_of(&FIGURE_1_TEXT_BY_ZINT)),
            "penalties by mask: {scores:?}"
        );
    }

    #[test]
    fn draws_the_version_4_symbol_of_the_standard_s_figure_29() {
        let data = stream(&alphanumeric(FIGURE_29_TEXT), 64);
        let scores = penalties(version(4), &data);
        assert_eq!(
            picture(&symbol(version(4), &data)),
            ISO_FIGURE_29,
            "penalties by mask: {scores:?}"
        );
    }

    #[test]
    fn draws_a_version_2_symbol_as_zint_does() {
        let data = stream(&numeric(VERSION_2_DIGITS), 28);
        let scores = penalties(version(2), &data);
        assert_eq!(
            picture(&symbol(version(2), &data)),
            VERSION_2_BY_ZINT,
            "penalties by mask: {scores:?}"
        );
    }

    #[test]
    fn picks_the_lower_numbered_of_two_masks_that_score_the_same() {
        let data = stream(&numeric("1234567890"), 16);
        let scores = penalties(version(1), &data);
        assert_eq!(
            picture(&symbol(version(1), &data)),
            TIE_BY_ZINT,
            "penalties by mask: {scores:?}"
        );
        // zint's test data: masks 001 and 010 score the same here, and no
        // mask scores lower.
        assert_eq!(scores[1], scores[2]);
        assert_eq!(scores.iter().min(), Some(&scores[1]));
    }

    #[test]
    fn draws_a_symbol_the_reader_reads_under_every_mask() {
        let payload = sample(100);
        let sixth = version(6);
        let symbols = candidates(sixth, &data_codewords(&payload, sixth));
        let masks: Vec<usize> = symbols.iter().map(read_format).collect();
        assert_eq!(masks, [0, 1, 2, 3, 4, 5, 6, 7]);
        for symbol in &symbols {
            assert_eq!(decode(symbol), payload);
        }
    }

    #[test]
    fn writes_the_version_information_where_zint_does() {
        // zint's version 10 symbol (`test_qr_encode`, item 22) has these
        // modules beside its top-right finder pattern, in rows 0 to 5 of
        // columns 46 to 48, and these beside its bottom-left one, in rows
        // 46 to 48 of columns 0 to 5. They do not depend on what a symbol
        // carries.
        let symbol = encode(&sample(181));
        assert_eq!(symbol.as_ref().map(Matrix::size), Ok(57));
        assert_eq!(
            symbol.as_ref().map(|symbol| crop(symbol, 0, 46, 6, 3)),
            Ok(rows_of(&["110", "010", "110", "010", "010", "100"]))
        );
        assert_eq!(
            symbol.as_ref().map(|symbol| crop(symbol, 46, 0, 3, 6)),
            Ok(rows_of(&["101001", "111110", "000000"]))
        );
    }

    #[test]
    fn a_symbol_tells_its_size_and_the_colour_of_each_module() {
        let symbol = encode(FIGURE_1_TEXT);
        assert_eq!(symbol.as_ref().map(Matrix::size), Ok(21));
        let seen = symbol.as_ref().map(|symbol| {
            (0..21)
                .map(|row| {
                    (0..21)
                        .map(|column| {
                            if symbol.is_dark(row, column) {
                                '1'
                            } else {
                                '0'
                            }
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        });
        assert_eq!(seen, Ok(rows_of(&FIGURE_1_TEXT_BY_ZINT)));
        // A place outside the symbol is light, as the margin is.
        let outside = symbol.map(|symbol| {
            [
                symbol.is_dark(21, 0),
                symbol.is_dark(0, 21),
                symbol.is_dark(20, 21),
                symbol.is_dark(usize::MAX, usize::MAX),
            ]
        });
        assert_eq!(outside, Ok([false; 4]));
    }

    /// A symbol carries its payload, and the payload of an invitation or a
    /// pairing link holds a secret, so the `Debug` form of a symbol shows
    /// its size and nothing of its modules.
    ///
    /// Verifies: SEC-OPS-013
    #[test]
    fn shows_only_its_size_when_formatted_for_debugging() {
        let shown: Vec<_> = [FIGURE_1_TEXT.to_vec(), sample(412)]
            .iter()
            .map(|payload| {
                encode(payload).map(|symbol| (format!("{symbol:?}"), format!("{symbol:#?}")))
            })
            .collect();
        assert_eq!(
            shown,
            [
                Ok((
                    "Matrix { size: 21, .. }".to_owned(),
                    "Matrix {\n    size: 21,\n    ..\n}".to_owned()
                )),
                Ok((
                    "Matrix { size: 77, .. }".to_owned(),
                    "Matrix {\n    size: 77,\n    ..\n}".to_owned()
                )),
            ]
        );
    }

    /// A question about a type `T`. Each answer is `false`, from [`Lacks`],
    /// unless `T` has the trait asked about: then the inherent constant of
    /// the same name applies, is found first, and answers `true`.
    struct Probe<T>(PhantomData<T>);

    /// The answers for a type that has none of the three traits.
    trait Lacks {
        const COMPARES: bool = false;
        const DISPLAYS: bool = false;
        const SERIALISES: bool = false;
    }

    impl<T> Lacks for Probe<T> {}

    impl<T: PartialEq> Probe<T> {
        const COMPARES: bool = true;
    }

    impl<T: core::fmt::Display> Probe<T> {
        const DISPLAYS: bool = true;
    }

    impl<T: serde::Serialize> Probe<T> {
        const SERIALISES: bool = true;
    }

    /// A symbol has no `==`, no `Display` and no serialised form: what it
    /// carries is read one module at a time, through `is_dark`. The first
    /// two rows show that the probe tells each of the three apart on types
    /// that have them.
    ///
    /// Verifies: SEC-OPS-013
    #[test]
    fn a_symbol_cannot_be_compared_displayed_or_serialised() {
        let answers = [
            (
                Probe::<Vec<u8>>::COMPARES,
                Probe::<Vec<u8>>::DISPLAYS,
                Probe::<Vec<u8>>::SERIALISES,
            ),
            (
                Probe::<std::io::Error>::COMPARES,
                Probe::<std::io::Error>::DISPLAYS,
                Probe::<std::io::Error>::SERIALISES,
            ),
            (
                Probe::<Matrix>::COMPARES,
                Probe::<Matrix>::DISPLAYS,
                Probe::<Matrix>::SERIALISES,
            ),
        ];
        assert_eq!(
            answers,
            [
                (true, false, true),
                (false, true, false),
                (false, false, false)
            ]
        );
    }

    #[test]
    fn chooses_the_smallest_version_that_holds_the_payload() {
        let mut least = 0;
        for (index, most) in CAPACITIES.into_iter().enumerate() {
            for len in [least, most] {
                let payload = sample(len);
                let symbol = encode(&payload);
                assert_eq!(
                    symbol.as_ref().map(Matrix::size),
                    Ok(21 + 4 * index),
                    "{len} bytes"
                );
                assert_eq!(
                    symbol.map(|symbol| decode(&symbol)),
                    Ok(payload),
                    "{len} bytes"
                );
            }
            least = most + 1;
        }
    }

    #[test]
    fn refuses_a_payload_longer_than_the_largest_symbol_holds() {
        // A symbol has no `==` (SEC-OPS-013), so a refusal is compared as
        // its error alone: `err` gives `None` for a symbol.
        assert_eq!(
            encode(&[0x61; 413]).err(),
            Some(QrError::TooLong { len: 413, max: 412 })
        );
        assert_eq!(
            encode(&sample(100_000)).err(),
            Some(QrError::TooLong {
                len: 100_000,
                max: 412
            })
        );
        assert_eq!(encode(&[0x61; 412]).as_ref().map(Matrix::size), Ok(77));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn a_symbol_reads_back_as_its_payload(payload in vec(any::<u8>(), 0..=412)) {
            prop_assert_eq!(encode(&payload).map(|symbol| decode(&symbol)), Ok(payload));
        }

        #[test]
        fn refuses_every_payload_over_the_cap(len in 413_usize..2_000) {
            prop_assert_eq!(
                encode(&sample(len)).err(),
                Some(QrError::TooLong { len, max: 412 })
            );
        }
    }
}
