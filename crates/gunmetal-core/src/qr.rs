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
//! The encoder is not written yet: this is the failing-test step, and
//! [`encode`] refuses every payload.

/// The longest payload a symbol holds, in bytes. Not set yet.
pub const MAX_PAYLOAD: usize = 0;

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

/// Encodes `payload` as the smallest symbol that holds it.
///
/// # Errors
///
/// [`QrError::TooLong`] when the payload is longer than [`MAX_PAYLOAD`]
/// bytes.
pub fn encode(payload: &[u8]) -> Result<Matrix, QrError> {
    Err(QrError::TooLong {
        len: payload.len(),
        max: MAX_PAYLOAD,
    })
}

// What follows stands in for the encoder's stages so that the tests of them
// compile. Each exists only under test and gives an empty answer; the
// encoder replaces them all.

#[cfg(test)]
struct Version;

#[cfg(test)]
static VERSIONS: [Version; 15] = [const { Version }; 15];

#[cfg(test)]
struct Mask {
    format: u16,
    flips: fn(usize, usize) -> bool,
}

#[cfg(test)]
static MASKS: [Mask; 8] = [const {
    Mask {
        format: 0,
        flips: |_, _| false,
    }
}; 8];

#[cfg(test)]
fn multiply(_left: u8, _right: u8) -> u8 {
    0
}

#[cfg(test)]
fn generator(_degree: usize) -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
fn correction_codewords(_data: &[u8], _generator: &[u8]) -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
fn interleave(_data: &[u8], _blocks: &[(usize, usize)], _correction_len: usize) -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
fn data_codewords(_payload: &[u8], _version: &Version) -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
fn candidates(_version: &Version, _data: &[u8]) -> Vec<Matrix> {
    Vec::new()
}

#[cfg(test)]
fn symbol(_version: &Version, _data: &[u8]) -> Matrix {
    Matrix { rows: Vec::new() }
}

#[cfg(test)]
fn penalty(_matrix: &Matrix) -> usize {
    0
}

#[cfg(test)]
fn run_penalty(_line: &[bool]) -> usize {
    0
}

#[cfg(test)]
fn block_penalty(_rows: &[Vec<bool>]) -> usize {
    0
}

#[cfg(test)]
fn finder_penalty(_line: &[bool]) -> usize {
    0
}

#[cfg(test)]
fn balance_penalty(_rows: &[Vec<bool>]) -> usize {
    0
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
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
            let mut logarithms = vec![0; 256];
            for (exponent, &power) in powers.iter().enumerate() {
                logarithms[usize::from(power)] = exponent;
            }
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
        let mut cells = vec![vec![Cell::Data; size]; size];
        for along in 0..size {
            paint(&mut cells, 6, along, Cell::Pattern(along % 2 == 0));
            paint(&mut cells, along, 6, Cell::Pattern(along % 2 == 0));
        }
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
        let degree = 31 - divisor.leading_zeros();
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
        let mut blocks = vec![Vec::new(); count];
        let mut rest = codewords.iter().copied();
        // The last `longer` blocks hold one data codeword more than the
        // others.
        for index in 0..=short {
            for (number, block) in blocks.iter_mut().enumerate() {
                if index < short || number >= count - longer {
                    block.push(rest.next().expect("a data codeword"));
                }
            }
        }
        for _ in 0..correction {
            for block in &mut blocks {
                block.push(rest.next().expect("an error correction codeword"));
            }
        }
        assert_eq!(rest.next(), None, "codewords are left over");
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
        assert_eq!(
            encode(&[0x61; 413]),
            Err(QrError::TooLong { len: 413, max: 412 })
        );
        assert_eq!(
            encode(&vec![0; 100_000]),
            Err(QrError::TooLong {
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
                encode(&vec![0x61; len]),
                Err(QrError::TooLong { len, max: 412 })
            );
        }
    }
}
