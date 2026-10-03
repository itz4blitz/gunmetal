//! The one streaming decompression helper (SEC-MED-009).
//!
//! Every piece of compressed data the core or the worker meets goes through
//! [`inflate`]: a PNG's image data before the image decoder sees it (WP-079),
//! and later `ID3v2` frames, Matroska `ContentCompression` and compressed MP4
//! headers. It inflates zlib (RFC 1950) or raw deflate (RFC 1951) data into
//! a [`BoundedBuf`] and stops as soon as the output would pass the lower of
//! the context's cap from [`Limits`] and the size the container declared, so
//! a kilobyte that expands to a gigabyte costs no more than the cap.
//!
//! The inflating itself is `miniz_oxide`'s pure-Rust decompressor, run on a
//! 32 KiB window, so the memory one call holds is the buffer's reservation
//! plus [`WORKING_OCTETS`], whatever the input says.
//!
//! **Steps** (SEC-MED-007). A call charges its [`Budget`] one step for every
//! input octet it reads and one for every octet it writes. The buffer never
//! holds more than its cap, so a call spends at most `1` × input octets +
//! the cap: a budget of `Budget::for_input(len, 1, cap)` is always enough.

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER;
use miniz_oxide::inflate::core::{DecompressorOxide, TINFL_LZ_DICT_SIZE, decompress_with_limit};

use crate::parse::{Budget, LimitKind, Limits, ParseFault, bounded_vec};

/// How a compressed stream is framed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// RFC 1950: a two-octet header, deflate data and an Adler-32 checksum
    /// of the inflated data, as PNG, `ID3v2` and Matroska use.
    Zlib,
    /// RFC 1951 deflate data alone, with no header and no checksum.
    Deflate,
}

/// What the inflated data is, which picks the cap from [`Limits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A picture, such as a PNG's image data:
    /// [`LimitKind::InflatedPicture`].
    Picture,
    /// Codec private data or a subtitle frame:
    /// [`LimitKind::InflatedCodecPrivate`].
    CodecPrivate,
    /// A compressed header: [`LimitKind::InflatedHeader`].
    Header,
}

impl Target {
    /// The limit that caps what data of this kind may inflate to.
    #[must_use]
    pub const fn limit(self) -> LimitKind {
        match self {
            Self::Picture => LimitKind::InflatedPicture,
            Self::CodecPrivate => LimitKind::InflatedCodecPrivate,
            Self::Header => LimitKind::InflatedHeader,
        }
    }
}

/// The octets of deflate history the decompressor keeps (RFC 1951 allows
/// back-references up to 32 KiB).
const WINDOW: usize = TINFL_LZ_DICT_SIZE;

/// The most octets one octet of deflate data can inflate to. The shortest
/// back-reference takes two bits, a one-bit length code and a one-bit
/// distance code, and copies at most 258 octets, so 8 bits give
/// 4 × 258 = 1,032 octets.
const MOST_PER_OCTET: u64 = 1_032;

/// The memory one [`inflate`] call holds besides its buffer's reservation:
/// the history window and the decompressor's tables. It is fixed, whatever
/// the input says (SEC-MED-009 allows 64 KiB).
pub const WORKING_OCTETS: usize = WINDOW + size_of::<DecompressorOxide>();

// The build fails if a new decompressor version's tables outgrow the 64 KiB
// SEC-MED-009 allows over the cap.
const _: () = assert!(WORKING_OCTETS <= 65_536);

/// An output buffer that holds at most the lower of the context's cap and
/// the declared size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedBuf {
    /// The inflated octets so far.
    bytes: Vec<u8>,
    /// The context's limit.
    limit: LimitKind,
    /// The limit's value in the caller's [`Limits`].
    max: u64,
    /// The inflated size the container declared, if it declares one.
    declared: Option<u64>,
}

impl BoundedBuf {
    /// An empty buffer for data of kind `target`, capped by `limits` and by
    /// `declared`, the inflated size the container states (a PNG's image
    /// size, an `ID3v2` data-length indicator), if it states one. It
    /// reserves nothing until [`inflate`] runs.
    #[must_use]
    pub const fn new(limits: &Limits, target: Target, declared: Option<u64>) -> Self {
        let limit = target.limit();
        Self {
            bytes: Vec::new(),
            limit,
            max: limits.get(limit),
            declared,
        }
    }

    /// The most octets this buffer may hold.
    #[must_use]
    pub fn cap(&self) -> u64 {
        self.declared.unwrap_or(self.max).min(self.max)
    }

    /// The octets inflated so far. After an error they are a prefix of the
    /// data, at most the cap long, and should be discarded.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// The octets reserved for the output, which never exceed the cap.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// The inflated octets, owned.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    /// The error for data that inflates past the cap: the declared size
    /// when that is what set the cap, the context's limit otherwise.
    const fn overflow(&self) -> InflateError {
        match self.declared {
            Some(declared) if declared <= self.max => InflateError::LongerThanDeclared {
                declared,
                offset: 0,
            },
            _ => InflateError::Fault(ParseFault::LimitExceeded {
                limit: self.limit,
                value: self.max.saturating_add(1),
                max: self.max,
                offset: 0,
            }),
        }
    }
}

/// Why compressed data could not be inflated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InflateError {
    /// A failure every parser shares: the input ends before the stream does
    /// ([`ParseFault::Truncated`]), the step budget ran out
    /// ([`ParseFault::BudgetExceeded`]), or the data inflates past the
    /// context's limit ([`ParseFault::LimitExceeded`], whose value is one
    /// more than the limit and whose offset is the stream's first octet).
    Fault(ParseFault),
    /// The data inflates past the size the container declared.
    LongerThanDeclared {
        /// The declared size.
        declared: u64,
        /// The stream's first octet: the whole stream is too long.
        offset: u64,
    },
    /// The data is not valid zlib or deflate: a bad zlib header, a preset
    /// dictionary, a reserved block type, a stored length that disagrees
    /// with its complement or a bad code table. A back-reference to before
    /// the start of the data is not refused: it reads zeros from the fresh
    /// window, never octets from an earlier call.
    Corrupt {
        /// The input octets read when the fault was found.
        offset: u64,
    },
    /// The zlib checksum does not match the inflated data.
    ChecksumMismatch {
        /// The input octets read, up to the end of the checksum.
        offset: u64,
    },
}

/// Inflates `input`, framed as `framing`, into `out`, charging `budget`.
///
/// Returns the input octets the stream took. Octets after the end of the
/// stream are left unread; the caller decides whether they matter.
///
/// # Errors
///
/// Returns an [`InflateError`] when the data would inflate past `out`'s cap,
/// is corrupt or truncated, fails its checksum, or costs more steps than
/// `budget` holds.
pub fn inflate(
    input: &[u8],
    framing: Framing,
    budget: &mut Budget,
    out: &mut BoundedBuf,
) -> Result<u64, InflateError> {
    let cap = out.cap();
    // Give back an earlier call's octets before reserving, so the old and
    // the new reservation are never held at once.
    out.bytes = Vec::new();
    // The reservation follows the input's octets as well as the cap
    // (SEC-MED-003): no stream can inflate to more than this.
    let most = widen(input.len()).saturating_mul(MOST_PER_OCTET);
    out.bytes = bounded_vec(cap, 1, most, cap);
    let flags = match framing {
        // Parsing the zlib header also has the decompressor compute the
        // Adler-32 and compare it with the stream's.
        Framing::Zlib => TINFL_FLAG_PARSE_ZLIB_HEADER,
        Framing::Deflate => 0,
    };
    let mut state = DecompressorOxide::new();
    // A fresh, zeroed window for every call, so a back-reference to before
    // the start of the data reads zeros and never an earlier call's octets.
    // It lives on the heap: 32 KiB is too large for a local array, and only
    // the bounded capacity helper may pre-size a vector.
    let size = widen(WINDOW);
    let mut window = bounded_vec(size, 1, size, size);
    window.extend(core::iter::repeat_n(0_u8, WINDOW));
    let mut rest = input;
    let mut read: u64 = 0;
    let mut at = 0;
    loop {
        // Ask for one octet more than the cap leaves room for, so a stream
        // that passes the cap is caught after exactly one octet too many.
        let room = cap.saturating_sub(widen(out.bytes.len()));
        let ask = usize::try_from(room.saturating_add(1)).unwrap_or(usize::MAX);
        let (status, used, made) =
            decompress_with_limit(&mut state, rest, &mut window, at, ask, flags);
        rest = rest.get(used..).unwrap_or_default();
        read = read.saturating_add(widen(used));
        let keep = made.min(usize::try_from(room).unwrap_or(usize::MAX));
        budget
            .charge(widen(used).saturating_add(widen(keep)), read)
            .map_err(InflateError::Fault)?;
        out.bytes.extend(window.iter().skip(at).take(keep));
        if made > keep {
            return Err(out.overflow());
        }
        // The decompressor writes up to the window's end and no further, so
        // the next call starts after this one's output, wrapping to 0.
        at = at
            .saturating_add(made)
            .checked_rem(WINDOW)
            .unwrap_or_default();
        match status {
            TINFLStatus::HasMoreOutput => {}
            TINFLStatus::Done => return Ok(read),
            TINFLStatus::FailedCannotMakeProgress | TINFLStatus::NeedsMoreInput => {
                return Err(InflateError::Fault(ParseFault::Truncated {
                    offset: widen(input.len()),
                    needed: 1,
                    available: 0,
                }));
            }
            TINFLStatus::Adler32Mismatch => {
                return Err(InflateError::ChecksumMismatch { offset: read });
            }
            _ => return Err(InflateError::Corrupt { offset: read }),
        }
    }
}

/// An octet count as a `u64`, saturating on a target where `usize` is
/// wider.
fn widen(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
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

    // -----------------------------------------------------------------------
    // A reference encoder, written for these tests from RFC 1950 and RFC
    // 1951 and sharing no code with the inflater.
    // -----------------------------------------------------------------------

    /// Deflate's bit packer: values go in least significant bit first,
    /// Huffman codes most significant bit first (RFC 1951, section 3.1.1).
    #[derive(Default)]
    struct Bits {
        bytes: Vec<u8>,
        /// Bits used in the last octet, 0 to 7; 0 means a new octet starts.
        used: u32,
    }

    impl Bits {
        fn bit(&mut self, bit: u32) {
            if self.used == 0 {
                self.bytes.push(0);
            }
            if bit == 1 {
                *self.bytes.last_mut().unwrap() |= 1 << self.used;
            }
            self.used = (self.used + 1) % 8;
        }

        /// `count` bits of `value`, least significant first.
        fn value(&mut self, value: u32, count: u32) {
            for i in 0..count {
                self.bit((value >> i) & 1);
            }
        }

        /// A Huffman code of `len` bits, most significant first.
        fn code(&mut self, code: u32, len: u32) {
            for i in (0..len).rev() {
                self.bit((code >> i) & 1);
            }
        }

        /// `count` zero bits, quickly.
        fn zeros(&mut self, count: u64) {
            let mut left = count;
            while left > 0 && self.used != 0 {
                self.bit(0);
                left -= 1;
            }
            let whole = usize::try_from(left / 8).unwrap();
            self.bytes.extend((0..whole).map(|_| 0));
            for _ in 0..left % 8 {
                self.bit(0);
            }
        }

        /// Pads to the next octet boundary with zero bits.
        fn align(&mut self) {
            self.used = 0;
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }

    /// Adler-32 (RFC 1950, section 8.2), written out byte by byte.
    fn adler32(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1_u32, 0_u32);
        for &byte in data {
            a = (a + u32::from(byte)) % 65_521;
            b = (b + a) % 65_521;
        }
        (b << 16) | a
    }

    /// Wraps deflate data in a zlib header (32 KiB window, no dictionary,
    /// fastest level: 0x78 0x01) and the checksum `check`.
    fn zlib(deflate: &[u8], check: u32) -> Vec<u8> {
        let mut out = vec![0x78, 0x01];
        out.extend_from_slice(deflate);
        out.extend_from_slice(&check.to_be_bytes());
        out
    }

    /// `data` as stored blocks of at most `block` octets (RFC 1951,
    /// section 3.2.4), at least one block even for no data.
    fn stored(data: &[u8], block: usize) -> Vec<u8> {
        let mut bits = Bits::default();
        let chunks: Vec<&[u8]> = if data.is_empty() {
            vec![&[]]
        } else {
            data.chunks(block).collect()
        };
        for (i, chunk) in chunks.iter().enumerate() {
            bits.value(u32::from(i + 1 == chunks.len()), 1);
            bits.value(0b00, 2);
            bits.align();
            let len = u16::try_from(chunk.len()).unwrap();
            bits.bytes.extend_from_slice(&len.to_le_bytes());
            bits.bytes.extend_from_slice(&(!len).to_le_bytes());
            bits.bytes.extend_from_slice(chunk);
        }
        bits.finish()
    }

    /// A literal or length symbol in the fixed Huffman code (RFC 1951,
    /// section 3.2.6).
    fn fixed_symbol(bits: &mut Bits, symbol: u32) {
        match symbol {
            0..=143 => bits.code(0x30 + symbol, 8),
            144..=255 => bits.code(0x190 + symbol - 144, 9),
            256..=279 => bits.code(symbol - 256, 7),
            _ => bits.code(0xC0 + symbol - 280, 8),
        }
    }

    /// `data` in one fixed-Huffman block. Each run of one octet value is
    /// written as the octet followed by back-references of distance one:
    /// length 258 (symbol 285) while at least 258 remain, then one of
    /// length 3 to 10 (symbols 257 to 264), then literals.
    fn fixed(data: &[u8]) -> Vec<u8> {
        let mut bits = Bits::default();
        bits.value(1, 1);
        bits.value(0b01, 2);
        let mut rest = data;
        while let Some((&first, _)) = rest.split_first() {
            let run = rest.iter().take_while(|&&b| b == first).count();
            fixed_symbol(&mut bits, u32::from(first));
            let mut left = run - 1;
            while left >= 258 {
                fixed_symbol(&mut bits, 285);
                bits.code(0, 5);
                left -= 258;
            }
            if (3..=10).contains(&left) {
                fixed_symbol(&mut bits, 254 + u32::try_from(left).unwrap());
                bits.code(0, 5);
                left = 0;
            }
            for _ in 0..left {
                fixed_symbol(&mut bits, u32::from(first));
            }
            rest = &rest[run..];
        }
        fixed_symbol(&mut bits, 256);
        bits.finish()
    }

    /// A deflate bomb: one dynamic-Huffman block whose codes make each
    /// back-reference of 258 octets two zero bits (RFC 1951, section
    /// 3.2.7). It inflates to one zero octet followed by `matches` × 258
    /// more.
    ///
    /// Code lengths: literal 0 and end-of-block take two bits, length
    /// symbol 285 one bit, and distance codes 0 and 1 one bit each. They
    /// are sent with the code-length code 18 (a run of zeros) in one bit
    /// and the lengths 1 and 2 in two bits each.
    fn bomb(matches: u64) -> Vec<u8> {
        let mut bits = Bits::default();
        bits.value(1, 1); // BFINAL
        bits.value(0b10, 2); // dynamic Huffman
        bits.value(286 - 257, 5); // HLIT
        bits.value(2 - 1, 5); // HDIST
        bits.value(18 - 4, 4); // HCLEN
        // Code-length code lengths, in the order 16, 17, 18, 0, 8, 7, 9, 6,
        // 10, 5, 11, 4, 12, 3, 13, 2, 14, 1.
        for len in [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2] {
            bits.value(len, 3);
        }
        // Canonical codes: 18 is `0`, length 1 is `10`, length 2 is `11`.
        let one = |bits: &mut Bits| bits.code(0b10, 2);
        let two = |bits: &mut Bits| bits.code(0b11, 2);
        let zeros = |bits: &mut Bits, count: u32| {
            bits.code(0, 1);
            bits.value(count - 11, 7);
        };
        two(&mut bits); // literal 0
        zeros(&mut bits, 138); // literals 1 to 138
        zeros(&mut bits, 117); // literals 139 to 255
        two(&mut bits); // end of block
        zeros(&mut bits, 28); // lengths 257 to 284
        one(&mut bits); // length 285: 258 octets
        one(&mut bits); // distance code 0: one octet back
        one(&mut bits); // distance code 1
        // Canonical codes: 285 is `0`, literal 0 is `10`, end of block
        // `11`; distance code 0 is `0`.
        bits.code(0b10, 2);
        bits.zeros(2 * matches);
        bits.code(0b11, 2);
        bits.finish()
    }

    /// The Adler-32 of `len` zero octets: `a` stays 1 and `b` adds 1 per
    /// octet.
    fn adler32_of_zeros(len: u64) -> u32 {
        (u32::try_from(len % 65_521).unwrap() << 16) | 1
    }

    /// `len` copies of `byte`, built without the reservation methods the
    /// core's Clippy configuration bans (SEC-MED-003).
    fn filled(byte: u8, len: usize) -> Vec<u8> {
        (0..len).map(|_| byte).collect()
    }

    // -----------------------------------------------------------------------
    // Helpers for calling the helper.
    // -----------------------------------------------------------------------

    /// Limits with `target`'s cap lowered to `max`.
    fn capped(target: Target, max: u64) -> Limits {
        Limits::DEFAULT.with_override(target.limit(), max).unwrap()
    }

    /// Enough steps for any call on `input` with cap `cap`.
    fn ample(input: &[u8], cap: u64) -> Budget {
        Budget::for_input(u64::try_from(input.len()).unwrap(), 1, cap)
    }

    /// Inflates `input` with `target`'s cap at `max` and nothing declared,
    /// returning the result and the buffer.
    fn run(
        input: &[u8],
        framing: Framing,
        max: u64,
        declared: Option<u64>,
    ) -> (Result<u64, InflateError>, BoundedBuf) {
        let limits = capped(Target::CodecPrivate, max);
        let mut out = BoundedBuf::new(&limits, Target::CodecPrivate, declared);
        let result = inflate(input, framing, &mut ample(input, max), &mut out);
        (result, out)
    }

    /// The octets inflated from `input`, or the error.
    fn inflated(input: &[u8], framing: Framing) -> Result<Vec<u8>, InflateError> {
        let (result, out) = run(input, framing, 1 << 20, None);
        result.map(|_| out.into_vec())
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap()
    }

    const KIB: u64 = 1 << 10;
    const MIB: u64 = 1 << 20;
    const GIB: u64 = 1 << 30;

    // -----------------------------------------------------------------------
    // The reference encoder against the specifications' own bytes.
    // -----------------------------------------------------------------------

    #[test]
    fn the_reference_adler32_matches_rfc_1950s_example_values() {
        // Adler-32 of "Wikipedia", the widely quoted worked example.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(&[0; 10]), adler32_of_zeros(10));
        assert_eq!(adler32(&filled(0, 70_000)), adler32_of_zeros(70_000));
    }

    #[test]
    fn the_reference_encoder_writes_the_octets_rfc_1951_specifies() {
        // An empty final stored block: 0b001, padding, LEN 0, NLEN 0xFFFF.
        assert_eq!(stored(b"", 10), [0x01, 0x00, 0x00, 0xFF, 0xFF]);
        assert_eq!(
            stored(b"abc", 2),
            [
                0x00, 0x02, 0x00, 0xFD, 0xFF, b'a', b'b', 0x01, 0x01, 0x00, 0xFE, 0xFF, b'c'
            ]
        );
        // "a" in the fixed code: header 1, 01; 0x91 as 10010001; 256 as 0000000.
        assert_eq!(fixed(b"a"), [0x4B, 0x04, 0x00]);
        // The zlib framing of an empty stored block, as zlib writes it at
        // level 0 apart from the header's level bits.
        assert_eq!(
            zlib(&stored(b"", 1), adler32(b"")),
            [
                0x78, 0x01, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01
            ]
        );
    }

    #[test]
    fn the_bomb_is_header_codes_then_two_bits_per_match() {
        // 71 header bits, 34 bits of code lengths, a 2-bit literal, 2 bits
        // per match and a 2-bit end of block.
        assert_eq!(bomb(0).len(), (71 + 34 + 2 + 2_usize).div_ceil(8));
        assert_eq!(
            bomb(4_000).len(),
            (71 + 34 + 2 + 8_000 + 2_usize).div_ceil(8)
        );
    }

    // -----------------------------------------------------------------------
    // Valid streams round-trip.
    // -----------------------------------------------------------------------

    /// `zlib.compress(b"hello")` and its raw deflate body.
    const HELLO_ZLIB: [u8; 13] = [
        0x78, 0x9C, 0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0x07, 0x00, 0x06, 0x2C, 0x02, 0x15,
    ];

    #[test]
    fn inflates_what_zlib_itself_wrote() {
        assert_eq!(inflated(&HELLO_ZLIB, Framing::Zlib), Ok(b"hello".to_vec()));
        assert_eq!(
            inflated(&HELLO_ZLIB[2..9], Framing::Deflate),
            Ok(b"hello".to_vec())
        );
    }

    #[test]
    fn inflates_stored_fixed_and_dynamic_blocks_from_the_reference_encoder() {
        let text: Vec<u8> = b"Gunmetal, gunmetal, gunmetal!"
            .iter()
            .copied()
            .cycle()
            .take(29 * 40)
            .collect();
        for deflate in [stored(&text, 100), fixed(&text)] {
            assert_eq!(inflated(&deflate, Framing::Deflate), Ok(text.clone()));
            let wrapped = zlib(&deflate, adler32(&text));
            assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(text.clone()));
        }
        // A run long enough for several 258-octet back-references.
        let run = [filled(7, 1_000), filled(9, 5)].concat();
        assert_eq!(inflated(&fixed(&run), Framing::Deflate), Ok(run.clone()));
        // Three matches: 1 + 3 × 258 zero octets.
        let zeros = filled(0, 775);
        assert_eq!(inflated(&bomb(3), Framing::Deflate), Ok(zeros.clone()));
        let wrapped = zlib(&bomb(3), adler32_of_zeros(775));
        assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(zeros));
    }

    #[test]
    fn inflates_an_empty_stream_to_nothing() {
        assert_eq!(inflated(&stored(b"", 1), Framing::Deflate), Ok(vec![]));
        let wrapped = zlib(&stored(b"", 1), 1);
        assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(vec![]));
    }

    #[test]
    fn crosses_the_32_kib_window_many_times() {
        // 100,000 octets in stored blocks of 7,000 and in one fixed block:
        // the output wraps the window three times.
        let data: Vec<u8> = (0..100_000_u32).map(|i| (i % 251) as u8).collect();
        assert_eq!(
            inflated(&stored(&data, 7_000), Framing::Deflate),
            Ok(data.clone())
        );
        let wrapped = zlib(&fixed(&data), adler32(&data));
        assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(data));
    }

    /// A back-reference just after the window wraps reads the octet written
    /// just before it, at the window's far end.
    #[test]
    fn back_references_reach_across_the_windows_wrap() {
        // Runs written as distance-one matches: the wraps at 32,768 and
        // 65,536 fall inside the runs of 7 and of 9.
        let data = [filled(3, 20_000), filled(7, 20_000), filled(9, 30_000)].concat();
        assert_eq!(inflated(&fixed(&data), Framing::Deflate), Ok(data.clone()));
        let wrapped = zlib(&fixed(&data), adler32(&data));
        assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(data));
    }

    #[test]
    fn reports_the_octets_the_stream_took_and_leaves_the_rest() {
        let data = b"payload";
        let deflate = stored(data, 100);
        let trailing = [deflate.clone(), b"after".to_vec()].concat();
        let (result, out) = run(&trailing, Framing::Deflate, KIB, None);
        assert_eq!((result, out.as_slice()), (Ok(12), &data[..]));
        let wrapped = [zlib(&deflate, adler32(data)), filled(0xEE, 3)].concat();
        let (result, out) = run(&wrapped, Framing::Zlib, KIB, None);
        assert_eq!((result, out.as_slice()), (Ok(18), &data[..]));
    }

    #[test]
    fn a_buffer_can_be_used_again() {
        let limits = Limits::DEFAULT;
        let mut out = BoundedBuf::new(&limits, Target::Header, None);
        let first = stored(b"first", 10);
        let second = stored(b"second", 10);
        assert_eq!(
            inflate(&first, Framing::Deflate, &mut ample(&first, MIB), &mut out),
            Ok(10)
        );
        assert_eq!(out.as_slice(), b"first");
        assert_eq!(
            inflate(
                &second,
                Framing::Deflate,
                &mut ample(&second, MIB),
                &mut out
            ),
            Ok(11)
        );
        assert_eq!(out.into_vec(), b"second");
    }

    // -----------------------------------------------------------------------
    // The cap: the lower of the context's limit and the declared size.
    // -----------------------------------------------------------------------

    #[test]
    fn each_target_takes_its_own_limit() {
        assert_eq!(Target::Picture.limit(), LimitKind::InflatedPicture);
        assert_eq!(
            Target::CodecPrivate.limit(),
            LimitKind::InflatedCodecPrivate
        );
        assert_eq!(Target::Header.limit(), LimitKind::InflatedHeader);
        let limits = Limits::DEFAULT;
        let caps = [Target::Picture, Target::CodecPrivate, Target::Header]
            .map(|target| BoundedBuf::new(&limits, target, None).cap());
        assert_eq!(caps, [33_554_432, 1_048_576, 4_194_304]);
    }

    #[test]
    fn the_cap_is_the_lower_of_the_limit_and_the_declared_size() {
        let limits = capped(Target::Header, 1_000);
        let cap = |declared| BoundedBuf::new(&limits, Target::Header, declared).cap();
        assert_eq!(cap(None), 1_000);
        assert_eq!(cap(Some(999)), 999);
        assert_eq!(cap(Some(1_000)), 1_000);
        assert_eq!(cap(Some(1_001)), 1_000);
        assert_eq!(cap(Some(u64::MAX)), 1_000);
        assert_eq!(cap(Some(0)), 0);
    }

    #[test]
    fn a_new_buffer_is_empty_and_reserves_nothing() {
        let out = BoundedBuf::new(&Limits::DEFAULT, Target::Picture, Some(5));
        assert_eq!((out.as_slice(), out.capacity()), (&[][..], 0));
    }

    /// Verifies: SEC-MED-009
    #[test]
    fn output_of_exactly_the_cap_is_accepted() {
        let data = filled(5, 300);
        let deflate = fixed(&data);
        let (result, out) = run(&deflate, Framing::Deflate, 300, None);
        assert_eq!(
            (result, out.as_slice()),
            (Ok(u64::try_from(deflate.len()).unwrap()), &data[..])
        );
        let (result, out) = run(&deflate, Framing::Deflate, KIB, Some(300));
        assert_eq!(
            (result, out.as_slice()),
            (Ok(u64::try_from(deflate.len()).unwrap()), &data[..])
        );
    }

    /// Verifies: SEC-MED-009
    #[test]
    fn one_octet_past_the_limit_is_refused_and_the_buffer_stops_at_the_limit() {
        let data = filled(5, 301);
        let (result, out) = run(&fixed(&data), Framing::Deflate, 300, None);
        assert_eq!(
            result,
            Err(InflateError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::InflatedCodecPrivate,
                value: 301,
                max: 300,
                offset: 0,
            }))
        );
        assert_eq!((out.as_slice(), out.capacity()), (&data[..300], 300));
    }

    /// Verifies: SEC-MED-009
    #[test]
    fn a_stream_that_declares_less_than_it_inflates_to_stops_at_the_declared_size() {
        let data: Vec<u8> = (0..=255).collect();
        let wrapped = zlib(&stored(&data, 1_000), adler32(&data));
        let (result, out) = run(&wrapped, Framing::Zlib, KIB, Some(200));
        assert_eq!(
            result,
            Err(InflateError::LongerThanDeclared {
                declared: 200,
                offset: 0,
            })
        );
        assert_eq!((out.as_slice(), out.capacity()), (&data[..200], 200));
        // Declared equal to the limit: the declared size is what is passed.
        let (result, _) = run(&wrapped, Framing::Zlib, 200, Some(200));
        assert_eq!(
            result,
            Err(InflateError::LongerThanDeclared {
                declared: 200,
                offset: 0,
            })
        );
        // Declared above the limit: the limit is what is passed.
        let (result, _) = run(&wrapped, Framing::Zlib, 200, Some(201));
        assert_eq!(
            result,
            Err(InflateError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::InflatedCodecPrivate,
                value: 201,
                max: 200,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-009
    #[test]
    fn a_cap_of_nothing_refuses_the_first_octet() {
        let (result, out) = run(&stored(b"x", 1), Framing::Deflate, KIB, Some(0));
        assert_eq!(
            result,
            Err(InflateError::LongerThanDeclared {
                declared: 0,
                offset: 0,
            })
        );
        assert_eq!((out.as_slice(), out.capacity()), (&[][..], 0));
        let (result, _) = run(&stored(b"", 1), Framing::Deflate, KIB, Some(0));
        assert_eq!(result, Ok(5));
    }

    #[test]
    fn the_window_is_deflates_32_kib() {
        assert_eq!(WINDOW, 32_768);
        // The working memory is the window and the decompressor's state.
        assert_eq!(WORKING_OCTETS - size_of::<DecompressorOxide>(), 32_768);
    }

    /// A 1 KiB zlib bomb inflates to about a mebibyte; with the picture
    /// limit at 64 KiB it stops there, holding no more than the limit.
    ///
    /// Verifies: SEC-MED-009
    #[test]
    fn a_one_kib_bomb_stops_at_the_cap() {
        let matches = 4_016;
        let input = zlib(&bomb(matches), adler32_of_zeros(1 + 258 * matches));
        assert_eq!(input.len(), 1_024);
        let limits = capped(Target::Picture, 64 * KIB);
        let mut out = BoundedBuf::new(&limits, Target::Picture, None);
        let result = inflate(
            &input,
            Framing::Zlib,
            &mut ample(&input, 64 * KIB),
            &mut out,
        );
        assert_eq!(
            result,
            Err(InflateError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::InflatedPicture,
                value: 64 * KIB + 1,
                max: 64 * KIB,
                offset: 0,
            }))
        );
        assert_eq!(out.as_slice(), filled(0, 65_536));
        assert_eq!(out.capacity(), 65_536);
        assert!(out.capacity() + WORKING_OCTETS <= 65_536 + 65_536);
    }

    /// A bomb that expands to a gibibyte, refused with the picture limit at
    /// its default of 32 MiB and at 1 MiB, holding no more than the cap
    /// plus the fixed working memory.
    ///
    /// Verifies: SEC-MED-009
    #[test]
    fn a_one_gib_bomb_stops_at_the_cap() {
        let matches = GIB / 258 + 1;
        let input = zlib(&bomb(matches), adler32_of_zeros(1 + 258 * matches));
        // 6 zlib octets and (109 + 2 × 4,161,791) bits of deflate data.
        assert_eq!(input.len(), 1_040_468);
        let limits = capped(Target::Picture, MIB);
        let mut out = BoundedBuf::new(&limits, Target::Picture, None);
        let result = inflate(&input, Framing::Zlib, &mut ample(&input, MIB), &mut out);
        assert_eq!(
            result,
            Err(InflateError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::InflatedPicture,
                value: MIB + 1,
                max: MIB,
                offset: 0,
            }))
        );
        assert_eq!(out.as_slice().len(), 1_048_576);
        assert!(out.as_slice().iter().all(|&b| b == 0));
        assert_eq!(out.capacity(), 1_048_576);
        assert!(out.capacity() + WORKING_OCTETS <= 1_048_576 + 65_536);
    }

    /// The reservation follows the input's octets, not the cap: a stream
    /// of a few octets cannot need the whole 32 MiB a picture may have.
    ///
    /// Verifies: SEC-MED-003
    #[test]
    fn reserves_no_more_than_the_input_could_inflate_to() {
        let input = bomb(3);
        assert_eq!(input.len(), 15);
        let mut out = BoundedBuf::new(&Limits::DEFAULT, Target::Picture, Some(u64::MAX));
        let result = inflate(&input, Framing::Deflate, &mut ample(&input, MIB), &mut out);
        assert_eq!(result, Ok(15));
        assert_eq!(out.as_slice(), filled(0, 775));
        assert_eq!(out.capacity(), 15 * 1_032);
    }

    // -----------------------------------------------------------------------
    // Malformed streams give typed errors.
    // -----------------------------------------------------------------------

    #[test]
    fn truncation_at_every_octet_gives_the_typed_error() {
        let text = b"truncate me, truncate me".to_vec();
        let streams = [
            (Framing::Deflate, stored(&text, 10)),
            (Framing::Deflate, fixed(&text)),
            (Framing::Deflate, bomb(2)),
            (Framing::Zlib, zlib(&fixed(&text), adler32(&text))),
            (Framing::Zlib, zlib(&bomb(2), adler32_of_zeros(517))),
        ];
        for (framing, stream) in streams {
            for len in 0..stream.len() {
                let (result, _) = run(&stream[..len], framing, KIB, None);
                let offset = u64::try_from(len).unwrap();
                assert_eq!(
                    result,
                    Err(InflateError::Fault(ParseFault::Truncated {
                        offset,
                        needed: 1,
                        available: 0,
                    }))
                );
            }
        }
    }

    #[test]
    fn a_reserved_block_type_is_corrupt() {
        // BFINAL 1, BTYPE 11.
        assert_eq!(
            inflated(&[0b111], Framing::Deflate),
            Err(InflateError::Corrupt { offset: 1 })
        );
    }

    #[test]
    fn a_stored_length_that_disagrees_with_its_complement_is_corrupt() {
        assert_eq!(
            inflated(&[0x01, 0x01, 0x00, 0xFF, 0xFF, b'x'], Framing::Deflate),
            Err(InflateError::Corrupt { offset: 5 })
        );
    }

    /// A back-reference to before the start of the data reads the fresh,
    /// zeroed window, never octets from an earlier call.
    #[test]
    fn a_back_reference_before_the_start_reads_zeros_and_nothing_earlier() {
        let limits = Limits::DEFAULT;
        let mut out = BoundedBuf::new(&limits, Target::Header, None);
        let secret = stored(b"secret", 10);
        assert_eq!(
            inflate(
                &secret,
                Framing::Deflate,
                &mut ample(&secret, MIB),
                &mut out
            ),
            Ok(11)
        );
        // Fixed block: length 3 (symbol 257) at distance 1 with no output yet.
        let mut bits = Bits::default();
        bits.value(1, 1);
        bits.value(0b01, 2);
        fixed_symbol(&mut bits, 257);
        bits.code(0, 5);
        fixed_symbol(&mut bits, 256);
        let reach_back = bits.finish();
        assert_eq!(
            inflate(
                &reach_back,
                Framing::Deflate,
                &mut ample(&reach_back, MIB),
                &mut out
            ),
            Ok(3)
        );
        assert_eq!(out.as_slice(), [0, 0, 0]);
    }

    #[test]
    fn a_bad_zlib_header_is_corrupt() {
        // Each header below is a multiple of 31 unless the check is what is
        // wrong (RFC 1950, section 2.2).
        let with_header = |cmf: u8, flg: u8| {
            let mut stream = zlib(&stored(b"", 1), 1);
            stream[..2].copy_from_slice(&[cmf, flg]);
            stream
        };
        // 0x7802 is 31 × 991 + 1.
        let bad_check = with_header(0x78, 0x02);
        // Method 7 instead of 8: 0x7709 is 31 × 983.
        let bad_method = with_header(0x77, 0x09);
        // A preset dictionary (FDICT): 0x7820 is 31 × 992.
        let dictionary = with_header(0x78, 0x20);
        // A 64 KiB window (CINFO 8), which zlib does not allow: 0x881C is
        // 31 × 1,124.
        let window = with_header(0x88, 0x1C);
        assert_eq!(
            inflated(&with_header(0x78, 0x01), Framing::Zlib),
            Ok(vec![])
        );
        for header in [bad_check, bad_method, dictionary, window] {
            assert_eq!(
                inflated(&header, Framing::Zlib),
                Err(InflateError::Corrupt { offset: 2 })
            );
        }
    }

    #[test]
    fn a_wrong_checksum_is_refused() {
        let wrapped = zlib(&stored(b"abc", 10), adler32(b"abd"));
        assert_eq!(wrapped.len(), 14);
        assert_eq!(
            inflated(&wrapped, Framing::Zlib),
            Err(InflateError::ChecksumMismatch { offset: 14 })
        );
    }

    // -----------------------------------------------------------------------
    // Steps (SEC-MED-007).
    // -----------------------------------------------------------------------

    /// One stored block of ten octets: 5 header octets and 10 data octets
    /// read, 10 octets written, 25 steps.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_steps_at_exactly_the_step_it_should() {
        let input = stored(&[3; 10], 100);
        let limits = Limits::DEFAULT;
        let mut out = BoundedBuf::new(&limits, Target::Header, None);
        let mut budget = Budget::for_input(0, 0, 25);
        assert_eq!(
            inflate(&input, Framing::Deflate, &mut budget, &mut out),
            Ok(15)
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 24);
        assert_eq!(
            inflate(&input, Framing::Deflate, &mut budget, &mut out),
            Err(InflateError::Fault(ParseFault::BudgetExceeded {
                offset: 15
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_bomb_spends_no_more_than_its_input_and_its_cap() {
        let input = zlib(&bomb(10_000), adler32_of_zeros(2_580_001));
        let len = u64::try_from(input.len()).unwrap();
        let limits = capped(Target::Picture, 100_000);
        let mut out = BoundedBuf::new(&limits, Target::Picture, None);
        let mut budget = Budget::for_input(len, 1, 100_000);
        let result = inflate(&input, Framing::Zlib, &mut budget, &mut out);
        assert_eq!(
            result,
            Err(InflateError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::InflatedPicture,
                value: 100_001,
                max: 100_000,
                offset: 0,
            }))
        );
        assert!(budget.remaining() >= len - 400);
    }

    // -----------------------------------------------------------------------
    // Properties.
    // -----------------------------------------------------------------------

    /// Inputs that look like streams: a valid stream from the reference
    /// encoder with some octets changed, cut short or extended.
    fn mangled() -> impl Strategy<Value = (Framing, Vec<u8>)> {
        (
            prop_oneof![Just(Framing::Zlib), Just(Framing::Deflate)],
            vec(any::<u8>(), 0..300),
            0_u8..3,
            vec((any::<usize>(), any::<u8>()), 0..4),
            any::<usize>(),
        )
            .prop_map(|(framing, data, kind, edits, cut)| {
                let deflate = match kind {
                    0 => stored(&data, 50),
                    1 => fixed(&data),
                    _ => bomb(u64::try_from(data.len()).unwrap()),
                };
                let mut stream = match framing {
                    Framing::Zlib => zlib(&deflate, adler32(&data)),
                    Framing::Deflate => deflate,
                };
                // Every encoder writes at least one octet.
                for (at, byte) in edits {
                    let at = at % stream.len();
                    stream[at] ^= byte;
                }
                stream.truncate(cut % (stream.len() + 1));
                (framing, stream)
            })
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn returns_within_its_step_bound_for_any_input(
            framing in prop_oneof![Just(Framing::Zlib), Just(Framing::Deflate)],
            input in vec(any::<u8>(), 0..512),
            max in 0_u64..4_096,
        ) {
            let len = u64::try_from(input.len()).unwrap();
            let (result, out) = on_small_stack(move || {
                let limits = capped(Target::Header, max);
                let mut out = BoundedBuf::new(&limits, Target::Header, None);
                let mut budget = Budget::for_input(len, 1, max);
                let result = inflate(&input, framing, &mut budget, &mut out);
                (result, out)
            });
            let out_of_steps = matches!(result, Err(InflateError::Fault(ParseFault::BudgetExceeded { .. })));
            prop_assert!(!out_of_steps);
            prop_assert!(u64::try_from(out.as_slice().len()).unwrap() <= max);
            prop_assert!(u64::try_from(out.capacity()).unwrap() <= max);
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn returns_within_its_step_bound_for_mangled_streams(
            (framing, input) in mangled(),
            max in 0_u64..200_000,
        ) {
            let len = u64::try_from(input.len()).unwrap();
            let (result, out) = on_small_stack(move || {
                let limits = capped(Target::Header, max);
                let mut out = BoundedBuf::new(&limits, Target::Header, None);
                let mut budget = Budget::for_input(len, 1, max);
                let result = inflate(&input, framing, &mut budget, &mut out);
                (result, out)
            });
            let out_of_steps = matches!(result, Err(InflateError::Fault(ParseFault::BudgetExceeded { .. })));
            prop_assert!(!out_of_steps);
            prop_assert!(u64::try_from(out.as_slice().len()).unwrap() <= max);
            if let Ok(read) = result {
                prop_assert!(read <= len);
            }
        }

        #[test]
        fn round_trips_the_reference_encoders_output(
            data in vec(any::<u8>(), 0..2_000),
            runs in vec((any::<u8>(), 1_usize..600), 0..4),
            block in 1_usize..3_000,
        ) {
            let data: Vec<u8> = runs
                .iter()
                .flat_map(|&(byte, len)| filled(byte, len))
                .chain(data)
                .collect();
            for deflate in [stored(&data, block), fixed(&data)] {
                prop_assert_eq!(inflated(&deflate, Framing::Deflate), Ok(data.clone()));
                let wrapped = zlib(&deflate, adler32(&data));
                prop_assert_eq!(inflated(&wrapped, Framing::Zlib), Ok(data.clone()));
            }
        }
    }
}
