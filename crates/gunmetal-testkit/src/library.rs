//! A synthetic music library: a tree of tiny files built from a seed and a
//! shape, with a manifest of what each file should yield.
//!
//! [`generate`] lays out albums, each in its own folder, one audio format
//! to an album and the formats taken in turn, so seven albums cover every
//! format. A [`Shape`] also asks for sets of two discs, compilations,
//! albums that share one title, a cover beside each album, a lyrics
//! sidecar, an album of odd and conflicting tags, and one cut-short file
//! of each format. The seed chooses the names, the years and the covers'
//! shade, through `SplitMix64` (Steele, Lea and Flood, "Fast splittable
//! pseudorandom number generators", 2014), so the same seed and shape give
//! the same octets on every machine. [`Library::write_to`] writes the tree
//! into a directory, usually a [`TempDir`](crate::tempdir::TempDir).
//!
//! Every file is written with the builders of this crate, and its
//! [`Yield`] is written here from the same inputs, never by running a
//! parser: the manifest is an oracle for Gunmetal's parsers, so it shares
//! nothing with them. No file holds real audio. A FLAC track is one frame
//! of silence; the others hold frames, packets or samples of zeros, enough
//! for a parser that reads headers and tags and never decodes.

use crate::bytes::Bytes;
use crate::checksum::{adler32, crc32_ieee};
use crate::flac::{self, Block};
use crate::flac_frames;
use crate::id3v1::Id3v1;
use crate::id3v2::{self, Encoding, Tag, Version};
use crate::mp4;
use crate::mp4_samples;
use crate::mpa;
use crate::ogg;
use crate::opus;
use crate::riff;
use std::fs;
use std::io;
use std::path::Path;

/// The vendor string of every comment block the generator writes.
const VENDOR: &[u8] = b"gunmetal-testkit";

/// The first word of every generated name.
const FIRST_WORDS: [&str; 8] = [
    "Amber", "Brass", "Cobalt", "Dusty", "Electric", "Faded", "Golden", "Hollow",
];

/// The second word of every generated name.
const SECOND_WORDS: [&str; 8] = [
    "Anthem", "Bridge", "Canyon", "Daybreak", "Echo", "Fable", "Garden", "Harbour",
];

/// The title that every same-titled album has.
const SHARED_TITLE: &str = "Greatest Hits";

/// The album artist of every compilation.
const VARIOUS_ARTISTS: &str = "Various Artists";

/// How many octets of a file its cut-short copy keeps: enough to name the
/// container, too few to hold what the format's header declares. The Opus
/// and Vorbis copies keep only the start of an Ogg page, so they are the
/// same octets and only their extensions tell them apart.
const CUT: usize = 12;

/// What a library holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag switches one independent kind of file on"
)]
pub struct Shape {
    /// Ordinary albums: one disc, one artist.
    pub albums: usize,
    /// Sets of two discs.
    pub multi_disc: usize,
    /// Compilations: the album artist is "Various Artists" and every track
    /// has an artist of its own.
    pub compilations: usize,
    /// Albums that are all called "Greatest Hits", each by its own artist.
    pub same_titled: usize,
    /// Tracks on each disc.
    pub tracks: u16,
    /// A cover beside each album: a PNG and a JPEG file by turns.
    pub artwork: bool,
    /// An `.lrc` sidecar beside the first track of each album.
    pub lyrics: bool,
    /// One more album, "Oddities", with one track for each [`Oddity`].
    pub odd_tags: bool,
    /// A folder, "Damaged", with one cut-short file of each [`Format`].
    pub damaged: bool,
}

/// An audio file format the generator writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Native FLAC, tagged with a Vorbis comment block.
    Flac,
    /// MPEG-1 Layer III, tagged with `ID3v2.4`.
    Mp3,
    /// Opus in Ogg.
    Opus,
    /// Vorbis in Ogg.
    Vorbis,
    /// AAC in MP4, tagged with an item list.
    M4a,
    /// PCM in RIFF WAVE, tagged with `ID3v2.4` in an `id3 ` chunk.
    Wav,
    /// PCM in AIFF, tagged with `ID3v2.4` in an `ID3 ` chunk.
    Aiff,
}

/// The audio a track of some format holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audio {
    /// The sample rate in hertz.
    pub sample_rate: u32,
    /// The channel count.
    pub channels: u8,
    /// The samples in each channel.
    pub samples: u64,
}

impl Format {
    /// Every format, in the order albums take them.
    pub const ALL: [Self; 7] = [
        Self::Flac,
        Self::Mp3,
        Self::Opus,
        Self::Vorbis,
        Self::M4a,
        Self::Wav,
        Self::Aiff,
    ];

    /// The file name extension, without its dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Vorbis => "ogg",
            Self::M4a => "m4a",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
        }
    }

    /// The audio that every generated track of this format declares.
    #[must_use]
    pub const fn audio(self) -> Audio {
        let (sample_rate, samples) = match self {
            // One frame of 4,096 samples.
            Self::Flac => (44_100, 4_096),
            // Two frames of 1,152 samples.
            Self::Mp3 => (44_100, 2_304),
            // One 20 ms packet at Opus's 48 kHz, after the pre-skip.
            Self::Opus => (48_000, 960),
            // The granule position of the last page.
            Self::Vorbis | Self::M4a => (44_100, 1_024),
            // Four sample frames.
            Self::Wav | Self::Aiff => (44_100, 4),
        };
        Audio {
            sample_rate,
            channels: 2,
            samples,
        }
    }
}

/// What is odd about a track's tags.
///
/// Each oddity takes effect only in the formats its variant names;
/// [`Track::encode`] ignores it in any other format, so the file then
/// carries the track's tags as an ordinary track of that format would.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Oddity {
    /// MP3 only: an `ID3v1` tag that names the title "Agreeable" and the
    /// artist "Someone Else", against the `ID3v2` tag. The [`Track`] holds
    /// what the `ID3v2` tag says.
    Id3v1Disagrees,
    /// FLAC, Opus and Vorbis only, the formats tagged with Vorbis comments:
    /// the comment names are in lower case, and the track and disc numbers
    /// carry their totals after a slash (`2/3`) in place of `TRACKTOTAL`
    /// and `DISCTOTAL` comments. No `COMPILATION` comment is written, so
    /// [`Track::compilation`] is lost; the generator gives this oddity only
    /// to a track that is not part of a compilation.
    SlashedNumbers,
    /// WAV only: no tag at all. Every text of its [`Track`] is empty and
    /// every number zero.
    Untagged,
}

/// What a track's tags say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The file's format, which also gives its [`Audio`].
    pub format: Format,
    /// The track's title.
    pub title: String,
    /// The track's artist.
    pub artist: String,
    /// The album's title.
    pub album: String,
    /// The album's artist.
    pub album_artist: String,
    /// The track's number on its disc, from 1.
    pub track: u16,
    /// The tracks on its disc.
    pub track_total: u16,
    /// The disc's number in its set, from 1.
    pub disc: u16,
    /// The discs in the set.
    pub disc_total: u16,
    /// The year of release.
    pub year: u16,
    /// Whether the album is marked as a compilation.
    pub compilation: bool,
    /// What is odd about the tags, if anything.
    pub oddity: Option<Oddity>,
}

/// What one file of the library should yield when Gunmetal reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yield {
    /// A track with these tags and its format's [`Audio`].
    Track(Track),
    /// A picture.
    Artwork {
        /// The media type: `image/png` or `image/jpeg`.
        media_type: &'static str,
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
    },
    /// Synchronised lyrics: each line's start in milliseconds and its text.
    Lyrics(Vec<(u32, String)>),
    /// Nothing but a problem: the file starts as this format's container
    /// does and is cut short inside its header. The format is the one the
    /// file's extension names: the cut Opus and Vorbis files hold the same
    /// octets, the start of an Ogg page, and nothing in them names the
    /// codec.
    Damaged(Format),
}

/// One file of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The folder the file is in, from the library's root, with `/` between
    /// its parts.
    pub folder: String,
    /// The file's name.
    pub name: String,
    /// The file's octets.
    pub bytes: Vec<u8>,
    /// What the file should yield.
    pub yields: Yield,
}

impl File {
    /// The file's path from the library's root, with `/` between its parts.
    #[must_use]
    pub fn path(&self) -> String {
        format!("{}/{}", self.folder, self.name)
    }
}

/// A generated library: its files, in the order they were made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// Every file, album by album, then the oddities, then the damaged.
    pub files: Vec<File>,
}

impl Library {
    /// The manifest: every file's path and what it should yield.
    #[must_use]
    pub fn manifest(&self) -> Vec<(String, &Yield)> {
        self.files
            .iter()
            .map(|file| (file.path(), &file.yields))
            .collect()
    }

    /// Writes every file beneath `root`, creating the folders it needs.
    ///
    /// # Errors
    ///
    /// Returns the error of the first folder or file that cannot be
    /// written, as it is. The files before it stay written.
    #[expect(
        clippy::disallowed_methods,
        reason = "a test's scratch directory has no root handle to write beneath (owner decision 33)"
    )]
    pub fn write_to(&self, root: &Path) -> io::Result<()> {
        for file in &self.files {
            let folder = root.join(&file.folder);
            fs::create_dir_all(&folder)?;
            fs::write(folder.join(&file.name), &file.bytes)?;
        }
        Ok(())
    }
}

impl Track {
    /// The file that carries these tags: the format's audio, tagged the
    /// way the format is usually tagged.
    ///
    /// An [`Oddity`] changes the file only in the formats its variant
    /// names and is ignored in any other, so a caller who pairs one with
    /// another format gets an ordinary file of that format.
    ///
    /// # Panics
    ///
    /// Panics when a text is too long for the tag that holds it, which
    /// takes megabytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self.format {
            Format::Flac => self.flac(),
            Format::Mp3 => self.mp3(),
            Format::Opus => self.opus(),
            Format::Vorbis => self.vorbis(),
            Format::M4a => self.m4a(),
            Format::Wav => self.wav(),
            Format::Aiff => self.aiff(),
        }
    }

    /// The tags as a Vorbis comment block.
    fn comments(&self) -> Vec<u8> {
        let fields = if self.oddity == Some(Oddity::SlashedNumbers) {
            vec![
                format!("title={}", self.title),
                format!("artist={}", self.artist),
                format!("album={}", self.album),
                format!("albumartist={}", self.album_artist),
                format!("tracknumber={}/{}", self.track, self.track_total),
                format!("discnumber={}/{}", self.disc, self.disc_total),
                format!("date={}", self.year),
            ]
        } else {
            vec![
                format!("TITLE={}", self.title),
                format!("ARTIST={}", self.artist),
                format!("ALBUM={}", self.album),
                format!("ALBUMARTIST={}", self.album_artist),
                format!("TRACKNUMBER={}", self.track),
                format!("TRACKTOTAL={}", self.track_total),
                format!("DISCNUMBER={}", self.disc),
                format!("DISCTOTAL={}", self.disc_total),
                format!("DATE={}", self.year),
                format!("COMPILATION={}", u8::from(self.compilation)),
            ]
        };
        let fields: Vec<&[u8]> = fields.iter().map(String::as_bytes).collect();
        opus::comment_block(VENDOR, &fields)
    }

    /// The tags as an `ID3v2.4` tag of UTF-8 text frames.
    fn id3(&self) -> Vec<u8> {
        let frames = [
            (b"TIT2", self.title.clone()),
            (b"TPE1", self.artist.clone()),
            (b"TALB", self.album.clone()),
            (b"TPE2", self.album_artist.clone()),
            (b"TRCK", format!("{}/{}", self.track, self.track_total)),
            (b"TPOS", format!("{}/{}", self.disc, self.disc_total)),
            (b"TDRC", self.year.to_string()),
            (b"TCMP", u8::from(self.compilation).to_string()),
        ];
        let mut tag = Tag::new(Version::V24);
        for (id, value) in frames {
            tag = tag.frame(id, 0, &id3v2::text(Encoding::Utf8, &[&value]));
        }
        tag.build()
    }

    /// The tags as the items of an MP4 item list.
    fn items(&self) -> Vec<u8> {
        let [track, track_total, disc, disc_total] =
            [self.track, self.track_total, self.disc, self.disc_total].map(u16::to_be_bytes);
        let year = self.year.to_string();
        let numbers = [[0, 0], disc, disc_total].concat();
        [
            item(*b"\xA9nam", 1, self.title.as_bytes()),
            item(*b"\xA9ART", 1, self.artist.as_bytes()),
            item(*b"\xA9alb", 1, self.album.as_bytes()),
            item(*b"aART", 1, self.album_artist.as_bytes()),
            item(*b"trkn", 0, &[[0, 0], track, track_total, [0, 0]].concat()),
            item(*b"disk", 0, &numbers),
            item(*b"\xA9day", 1, year.as_bytes()),
            item(*b"cpil", 21, &[u8::from(self.compilation)]),
        ]
        .concat()
    }

    /// A FLAC stream: STREAMINFO, the comments and one frame of silence.
    fn flac(&self) -> Vec<u8> {
        let info = flac::StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 44_100,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 4_096,
            md5: [0; 16],
        };
        let blocks = [
            Block::StreamInfo(info),
            Block::VorbisComment(self.comments()),
        ];
        let frame = flac_frames::frame(&flac_frames::Header::cd(false, 0), &[16, 16]);
        [flac::stream(&blocks), frame].concat()
    }

    /// An MP3 file: the `ID3v2` tag, then two frames at 32 kbit/s.
    fn mp3(&self) -> Vec<u8> {
        let frame = mpa::Frame::layer3(mpa::Version::Mpeg1, 1, 0, mpa::Mode::Stereo);
        let mut file = self.id3();
        file.extend(mpa::stream(&[frame, frame]));
        if self.oddity == Some(Oddity::Id3v1Disagrees) {
            let rival = Id3v1::new().title(b"Agreeable").artist(b"Someone Else");
            file.extend(rival.build());
        }
        file
    }

    /// An Ogg Opus stream: the identification header, the comment header,
    /// and one packet of 20 ms that holds no audio.
    fn opus(&self) -> Vec<u8> {
        let head = opus::OpusHead::stereo();
        let tags = [b"OpusTags".as_slice(), &self.comments()].concat();
        let end = u64::from(head.pre_skip) + self.format.audio().samples;
        stream(&head.to_bytes(), &[&tags], &[0xFC], end)
    }

    /// An Ogg Vorbis stream: the identification header, the comment
    /// header, a setup header that sets nothing up, and one empty audio
    /// packet.
    fn vorbis(&self) -> Vec<u8> {
        let comments = [b"\x03vorbis".as_slice(), &self.comments(), &[1]].concat();
        stream(
            &opus::VorbisIdent::stereo().to_bytes(),
            &[&comments, b"\x05vorbis"],
            &[0],
            self.format.audio().samples,
        )
    }

    /// An MP4 file: the file type, one four-octet sample of media data,
    /// then the movie with one AAC track and the item list.
    fn m4a(&self) -> Vec<u8> {
        let file_type = mp4::ftyp(*b"M4A ", 0, &[*b"M4A ", *b"mp42", *b"isom"]);
        let media = mp4::mp4_box(*b"mdat", &[0; 4]);
        let config = mp4::audio_specific_config(2, 4, 2);
        let entry = mp4::sample_entry(&mp4::SampleEntry {
            format: *b"mp4a",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: &mp4::esds(&mp4::Esds {
                object_type: 0x40,
                max_bitrate: 0,
                avg_bitrate: 0,
                specific: Some(&config),
                width: 1,
            }),
        });
        let mut tables = Bytes::new();
        tables
            .bytes(&mp4::stsd(&[&entry]))
            .mp4_box(*b"stts", &mp4_samples::stts(&[(1, 1_024)]))
            .mp4_box(*b"stsc", &mp4_samples::stsc(&[(1, 1, 1)]))
            .mp4_box(*b"stsz", &mp4_samples::stsz(&[4]))
            // The sample follows the 28 octets of the file type box and the
            // 8 of the media data box's header.
            .mp4_box(*b"stco", &mp4_samples::stco(&[36]));
        let track = mp4::trak(*b"soun", &mp4::mdhd(44_100, 1_024), tables.as_slice());
        let movie = mp4::mp4_box(*b"moov", &[track, mp4::udta(false, &self.items())].concat());
        [file_type, media, movie].concat()
    }

    /// A WAV file: 16-bit stereo PCM, four sample frames of silence, and
    /// the `ID3v2` tag in an `id3 ` chunk unless the track is untagged.
    fn wav(&self) -> Vec<u8> {
        let mut chunks = Bytes::new();
        chunks
            .riff_chunk(*b"fmt ", &riff::format(1, 2, 44_100, 16))
            .riff_chunk(*b"data", &[0; 16]);
        if self.oddity != Some(Oddity::Untagged) {
            chunks.riff_chunk(*b"id3 ", &self.id3());
        }
        riff::wave(chunks.as_slice())
    }

    /// An AIFF file: 16-bit stereo PCM, four sample frames of silence, and
    /// the `ID3v2` tag in an `ID3 ` chunk.
    fn aiff(&self) -> Vec<u8> {
        let mut chunks = Bytes::new();
        chunks
            .aiff_chunk(*b"COMM", &riff::comm(2, 4, 16, 44_100))
            .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &[0; 16]))
            .aiff_chunk(*b"ID3 ", &self.id3());
        riff::aiff(*b"AIFF", chunks.as_slice())
    }
}

/// One item of an MP4 item list: a box of type `kind` holding one `data`
/// box.
fn item(kind: [u8; 4], type_code: u32, value: &[u8]) -> Vec<u8> {
    mp4::mp4_box(kind, &mp4::data(type_code, value))
}

/// One page of the generator's Ogg streams, which all have the serial
/// number 1, holding `packets` whole.
fn page(flags: u8, granule: u64, sequence: u32, packets: &[&[u8]]) -> ogg::Page {
    ogg::Page {
        flags,
        granule,
        serial: 1,
        sequence,
        lacing: packets
            .iter()
            .flat_map(|packet| ogg::lacing(packet.len()))
            .collect(),
        body: packets.concat(),
    }
}

/// An Ogg stream of three pages, as the Ogg mappings of Opus and Vorbis
/// ask: the identification header alone on the first, the other `headers`
/// on the second, and one packet of `audio` on the last, where the stream
/// ends at the granule position `end`.
fn stream(identification: &[u8], headers: &[&[u8]], audio: &[u8], end: u64) -> Vec<u8> {
    ogg::write(&[
        page(ogg::FIRST, 0, 0, &[identification]),
        page(0, 0, 1, headers),
        page(ogg::LAST, end, 2, &[audio]),
    ])
}

/// A PNG file of one 8-bit greyscale pixel of the shade `grey` (PNG,
/// second edition, sections 5 and 11.2.2), its pixel data in a stored
/// deflate block (RFC 1950 and RFC 1951, section 3.2.4).
fn png(grey: u8) -> Vec<u8> {
    let mut header = Bytes::new();
    // Width 1, height 1, bit depth 8, then colour type 0 (greyscale) and
    // the only compression, filter and interlace methods, all 0.
    header.u32_be(1).u32_be(1).u8(8).zeros(4);
    // One scanline: filter type 0, then the pixel.
    let scanline = [0, grey];
    let mut data = Bytes::new();
    data.bytes(&[0x78, 0x01]) // a deflate stream with a 32 KiB window
        .u8(1) // the final block, stored
        .u16_le(2) // its length
        .u16_le(!2) // and the length's complement
        .bytes(&scanline)
        .u32_be(adler32(&scanline));
    let mut file = Bytes::new();
    file.bytes(b"\x89PNG\r\n\x1A\n");
    for (len, kind, body) in [
        (13, b"IHDR", header.as_slice()),
        (13, b"IDAT", data.as_slice()),
        (0, b"IEND", &[]),
    ] {
        let checked = [kind.as_slice(), body].concat();
        file.u32_be(len)
            .bytes(&checked)
            .u32_be(crc32_ieee(&checked));
    }
    file.into_vec()
}

/// A baseline JPEG file of one grey pixel (ITU-T T.81, annex B, in a JFIF
/// 1.01 wrapper): one component, a quantisation table of ones, and Huffman
/// tables whose one code, a single zero bit, says "no difference" for the
/// DC coefficient and "end of block" for the rest.
fn jpeg() -> Vec<u8> {
    let mut file = Bytes::new();
    file.bytes(b"\xFF\xD8") // start of image
        .bytes(b"\xFF\xE0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00")
        .bytes(b"\xFF\xDB\x00\x43\x00") // quantisation table 0, 8-bit
        .bytes(&[1; 64])
        // A baseline frame, 8 bits a sample, 1 by 1, one component with
        // identifier 1, sampled once each way, using table 0.
        .bytes(b"\xFF\xC0\x00\x0B\x08\x00\x01\x00\x01\x01\x01\x11\x00")
        // The DC table 0: one code of one bit, for the symbol 0.
        .bytes(b"\xFF\xC4\x00\x14\x00\x01")
        .zeros(16)
        // The AC table 0, the same.
        .bytes(b"\xFF\xC4\x00\x14\x10\x01")
        .zeros(16)
        // The scan of component 1 with those tables, over the whole
        // spectrum; then the two codes, padded with ones; then the end.
        .bytes(b"\xFF\xDA\x00\x08\x01\x01\x00\x00\x3F\x00\x3F\xFF\xD9");
    file.into_vec()
}

/// `SplitMix64`: the seeded sequence every choice is drawn from.
struct Rng(u64);

impl Rng {
    /// The next 64 bits of the sequence.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// The lowest octet of the next 64 bits.
    fn byte(&mut self) -> u8 {
        let [low, ..] = self.next().to_le_bytes();
        low
    }

    /// One of `words`.
    fn pick(&mut self, words: &[&'static str; 8]) -> &'static str {
        words[usize::from(self.byte()) % words.len()]
    }

    /// A name of two words.
    fn name(&mut self) -> String {
        let first = self.pick(&FIRST_WORDS);
        let second = self.pick(&SECOND_WORDS);
        format!("{first} {second}")
    }
}

/// The kind of an album.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// One disc by one artist.
    Plain,
    /// Two discs.
    MultiDisc,
    /// A track artist for every track.
    Compilation,
    /// The shared title.
    SameTitled,
}

/// The library that `seed` and `shape` give: the same files, octet for
/// octet, every time.
///
/// Albums are numbered from 1 in the order of [`Shape`]'s fields, and an
/// album's number is part of its artist's name, its title and its folder,
/// so no two albums are the same album and no two files share a path. An
/// album's folder is `<album artist>/<number> <title>`, and a track's file
/// is `<disc><track> <title>.<extension>`.
///
/// # Panics
///
/// Panics when a name is too long for the tag that holds it, which no
/// generated name is.
#[must_use]
pub fn generate(seed: u64, shape: &Shape) -> Library {
    let mut rng = Rng(seed);
    let mut files = Vec::new();
    let kinds = [
        (Kind::Plain, shape.albums),
        (Kind::MultiDisc, shape.multi_disc),
        (Kind::Compilation, shape.compilations),
        (Kind::SameTitled, shape.same_titled),
    ]
    .into_iter()
    .flat_map(|(kind, count)| std::iter::repeat_n(kind, count));
    for (index, kind) in kinds.enumerate() {
        album(&mut rng, shape, index, kind, &mut files);
    }
    if shape.odd_tags {
        oddities(&mut files);
    }
    if shape.damaged {
        damaged(&mut files);
    }
    Library { files }
}

/// Appends the album at `index` to `files`: its tracks, disc by disc, with
/// the lyrics after the first, then its cover.
fn album(rng: &mut Rng, shape: &Shape, index: usize, kind: Kind, files: &mut Vec<File>) {
    let number = index + 1;
    let format = Format::ALL[index % Format::ALL.len()];
    let artist = format!("{} {number}", rng.name());
    let drawn = rng.name();
    let title = if kind == Kind::SameTitled {
        SHARED_TITLE.to_owned()
    } else {
        format!("{drawn} {number}")
    };
    let year = 1_960 + u16::from(rng.byte() % 64);
    let compilation = kind == Kind::Compilation;
    let album_artist = if compilation {
        VARIOUS_ARTISTS.to_owned()
    } else {
        artist.clone()
    };
    let discs = if kind == Kind::MultiDisc { 2 } else { 1 };
    let folder = format!("{album_artist}/{number:03} {title}");
    for disc in 1..=discs {
        for position in 1..=shape.tracks {
            let name = rng.name();
            let track = Track {
                format,
                title: name.clone(),
                artist: if compilation {
                    rng.name()
                } else {
                    artist.clone()
                },
                album: title.clone(),
                album_artist: album_artist.clone(),
                track: position,
                track_total: shape.tracks,
                disc,
                disc_total: discs,
                year,
                compilation,
                oddity: None,
            };
            let stem = format!("{disc}{position:02} {name}");
            files.push(track_file(&folder, &stem, track));
            if shape.lyrics && disc == 1 && position == 1 {
                files.push(File {
                    folder: folder.clone(),
                    name: format!("{stem}.lrc"),
                    bytes: format!("[00:00.00]{name}\n[00:02.50]{title}\n").into_bytes(),
                    yields: Yield::Lyrics(vec![(0, name), (2_500, title.clone())]),
                });
            }
        }
    }
    if shape.artwork {
        let (name, media_type, bytes) = if index % 2 == 0 {
            ("cover.png", "image/png", png(rng.byte()))
        } else {
            ("cover.jpg", "image/jpeg", jpeg())
        };
        files.push(File {
            folder,
            name: name.to_owned(),
            bytes,
            yields: Yield::Artwork {
                media_type,
                width: 1,
                height: 1,
            },
        });
    }
}

/// The file of `track`, named `stem` and its format's extension.
fn track_file(folder: &str, stem: &str, track: Track) -> File {
    File {
        folder: folder.to_owned(),
        name: format!("{stem}.{}", track.format.extension()),
        bytes: track.encode(),
        yields: Yield::Track(track),
    }
}

/// A track of the album "Oddities" by "Odd Tags", released in 2001 on one
/// disc of three tracks.
fn odd(format: Format, title: &str, track: u16, oddity: Option<Oddity>) -> Track {
    Track {
        format,
        title: title.to_owned(),
        artist: "Odd Tags".to_owned(),
        album: "Oddities".to_owned(),
        album_artist: "Odd Tags".to_owned(),
        track,
        track_total: 3,
        disc: 1,
        disc_total: 1,
        year: 2_001,
        compilation: false,
        oddity,
    }
}

/// Appends the album of odd tags to `files`: one track for each
/// [`Oddity`].
fn oddities(files: &mut Vec<File>) {
    let untagged = Track {
        format: Format::Wav,
        title: String::new(),
        artist: String::new(),
        album: String::new(),
        album_artist: String::new(),
        track: 0,
        track_total: 0,
        disc: 0,
        disc_total: 0,
        year: 0,
        compilation: false,
        oddity: Some(Oddity::Untagged),
    };
    for (stem, track) in [
        (
            "101 Disagreeing",
            odd(Format::Mp3, "Disagreeing", 1, Some(Oddity::Id3v1Disagrees)),
        ),
        (
            "102 Slashed",
            odd(Format::Flac, "Slashed", 2, Some(Oddity::SlashedNumbers)),
        ),
        ("103 Untagged", untagged),
    ] {
        files.push(track_file("Odd Tags/Oddities", stem, track));
    }
}

/// Appends the damaged files to `files`: for each format, the first
/// [`CUT`] octets of an ordinary track.
fn damaged(files: &mut Vec<File>) {
    for format in Format::ALL {
        let mut bytes = odd(format, "Cut Short", 1, None).encode();
        bytes.truncate(CUT);
        files.push(File {
            folder: "Damaged".to_owned(),
            name: format!("cut.{}", format.extension()),
            bytes,
            yields: Yield::Damaged(format),
        });
    }
}

#[cfg(test)]
#[expect(
    clippy::disallowed_methods,
    reason = "the tests read back the tree written into a scratch directory (owner decision 33)"
)]
mod tests {
    use super::*;
    use crate::checksum::{crc8_flac, crc16_flac, crc32_ogg};
    use crate::tempdir::TempDir;
    use std::collections::BTreeMap;
    use std::io::ErrorKind;

    /// A shape with one album of each kind and every extra switched on.
    const SMALL: Shape = Shape {
        albums: 1,
        multi_disc: 1,
        compilations: 1,
        same_titled: 2,
        tracks: 2,
        artwork: true,
        lyrics: true,
        odd_tags: true,
        damaged: true,
    };

    /// A shape with an album of each format, covers, and nothing else.
    const EVERY_FORMAT: Shape = Shape {
        albums: 7,
        multi_disc: 0,
        compilations: 0,
        same_titled: 0,
        tracks: 1,
        artwork: true,
        lyrics: false,
        odd_tags: false,
        damaged: true,
    };

    /// The octets that `digits` spell in hexadecimal.
    fn hex(digits: &str) -> Vec<u8> {
        digits
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// `parts` joined, so a literal file reads one field per line.
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// A track of `format` whose every tag differs from the others.
    fn sample(format: Format) -> Track {
        Track {
            format,
            title: "One".to_owned(),
            artist: "Two".to_owned(),
            album: "Three".to_owned(),
            album_artist: "Four".to_owned(),
            track: 5,
            track_total: 6,
            disc: 7,
            disc_total: 8,
            year: 1999,
            compilation: true,
            oddity: None,
        }
    }

    /// The comment block of [`sample`]: the vendor string, ten comments,
    /// each behind its little-endian length. 180 octets.
    fn sample_comments() -> Vec<u8> {
        cat(&[
            b"\x10\x00\x00\x00gunmetal-testkit",
            b"\x0A\x00\x00\x00",
            b"\x09\x00\x00\x00TITLE=One",
            b"\x0A\x00\x00\x00ARTIST=Two",
            b"\x0B\x00\x00\x00ALBUM=Three",
            b"\x10\x00\x00\x00ALBUMARTIST=Four",
            b"\x0D\x00\x00\x00TRACKNUMBER=5",
            b"\x0C\x00\x00\x00TRACKTOTAL=6",
            b"\x0C\x00\x00\x00DISCNUMBER=7",
            b"\x0B\x00\x00\x00DISCTOTAL=8",
            b"\x09\x00\x00\x00DATE=1999",
            b"\x0D\x00\x00\x00COMPILATION=1",
        ])
    }

    /// The `ID3v2.4` tag of [`sample`]: the header, then eight UTF-8 text
    /// frames with syncsafe sizes and no flags. 124 octets.
    fn sample_id3() -> Vec<u8> {
        cat(&[
            b"ID3\x04\x00\x00\x00\x00\x00\x72",
            b"TIT2\x00\x00\x00\x04\x00\x00\x03One",
            b"TPE1\x00\x00\x00\x04\x00\x00\x03Two",
            b"TALB\x00\x00\x00\x06\x00\x00\x03Three",
            b"TPE2\x00\x00\x00\x05\x00\x00\x03Four",
            b"TRCK\x00\x00\x00\x04\x00\x00\x035/6",
            b"TPOS\x00\x00\x00\x04\x00\x00\x037/8",
            b"TDRC\x00\x00\x00\x05\x00\x00\x031999",
            b"TCMP\x00\x00\x00\x02\x00\x00\x031",
        ])
    }

    /// The STREAMINFO block every generated FLAC file starts with, after
    /// the marker: blocks of 4,096 samples, unknown frame sizes, 44.1 kHz,
    /// two channels of 16 bits, 4,096 samples, no MD5.
    fn stream_info() -> Vec<u8> {
        cat(&[
            b"fLaC\x00\x00\x00\x22",
            b"\x10\x00\x10\x00\x00\x00\x00\x00\x00\x00",
            b"\x0A\xC4\x42\xF0\x00\x00\x10\x00",
            &[0; 16],
        ])
    }

    /// The frame every generated FLAC file ends with: a fixed-blocksize
    /// header for 4,096 samples of 16-bit stereo at 44.1 kHz, frame 0, its
    /// CRC-8, two constant subframes of silence and the CRC-16.
    const FLAC_FRAME: &[u8] = b"\xFF\xF8\xC9\x18\x00\xC2\x00\x00\x00\x00\x00\x00\xB8\xEE";

    /// One MPEG-1 Layer III frame at 32 kbit/s and 44.1 kHz, stereo: the
    /// header and 100 octets of nothing, 104 in all.
    fn mp3_frame() -> Vec<u8> {
        cat(&[b"\xFF\xFB\x10\x00", &[0; 100]])
    }

    /// The path and the yield of every file, with the path owned.
    fn manifest(library: &Library) -> Vec<(String, Yield)> {
        library
            .manifest()
            .into_iter()
            .map(|(path, yields)| (path, yields.clone()))
            .collect()
    }

    /// A manifest entry for an untroubled track.
    fn track(
        path: &str,
        format: Format,
        [title, artist, album, album_artist]: [&str; 4],
        [track, track_total, disc, disc_total, year]: [u16; 5],
        compilation: bool,
    ) -> (String, Yield) {
        (
            path.to_owned(),
            Yield::Track(Track {
                format,
                title: title.to_owned(),
                artist: artist.to_owned(),
                album: album.to_owned(),
                album_artist: album_artist.to_owned(),
                track,
                track_total,
                disc,
                disc_total,
                year,
                compilation,
                oddity: None,
            }),
        )
    }

    /// A manifest entry for a lyrics sidecar of two lines.
    fn lyrics(path: &str, first: &str, second: &str) -> (String, Yield) {
        (
            path.to_owned(),
            Yield::Lyrics(vec![(0, first.to_owned()), (2_500, second.to_owned())]),
        )
    }

    /// A manifest entry for a cover of one pixel.
    fn cover(path: &str, media_type: &'static str) -> (String, Yield) {
        (
            path.to_owned(),
            Yield::Artwork {
                media_type,
                width: 1,
                height: 1,
            },
        )
    }

    /// Every file beneath `root`: its path from `root`, with `/` between
    /// the parts, and its octets.
    fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut folders = vec![root.to_path_buf()];
        while let Some(folder) = folders.pop() {
            for entry in fs::read_dir(&folder).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    folders.push(path);
                } else {
                    let parts: Vec<&str> = path
                        .strip_prefix(root)
                        .unwrap()
                        .iter()
                        .map(|part| part.to_str().unwrap())
                        .collect();
                    found.insert(parts.join("/"), fs::read(&path).unwrap());
                }
            }
        }
        found
    }

    /// The path and octets of every file of `library`, as [`tree`] gives
    /// them.
    fn files(library: &Library) -> BTreeMap<String, Vec<u8>> {
        library
            .files
            .iter()
            .map(|file| (file.path(), file.bytes.clone()))
            .collect()
    }

    /// Whether `file` is a whole track in one of `formats`.
    fn is_track_of(file: &File, formats: &[Format]) -> bool {
        matches!(&file.yields, Yield::Track(track) if formats.contains(&track.format))
    }

    /// The pages of an Ogg stream, each from its capture pattern to the end
    /// of its body.
    fn pages(mut stream: &[u8]) -> Vec<&[u8]> {
        let mut pages = Vec::new();
        while let Some(&segments) = stream.get(26) {
            let table = &stream[27..27 + usize::from(segments)];
            let body: usize = table.iter().map(|&len| usize::from(len)).sum();
            let (page, rest) = stream.split_at(27 + table.len() + body);
            pages.push(page);
            stream = rest;
        }
        pages
    }

    #[test]
    fn draws_the_published_splitmix64_sequence() {
        // The first outputs for the seed 1234567, as the reference
        // implementation (Vigna's splitmix64.c) gives them.
        let mut rng = Rng(1_234_567);
        assert_eq!(
            [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()],
            [
                6_457_827_717_110_365_317,
                3_203_168_211_198_807_973,
                9_817_491_932_198_370_423,
                4_593_380_528_125_082_431,
                16_408_922_859_458_223_821,
            ]
        );
    }

    #[test]
    fn names_each_format_and_its_audio() {
        let table: Vec<(&str, u32, u8, u64)> = Format::ALL
            .into_iter()
            .map(|format| {
                let audio = format.audio();
                (
                    format.extension(),
                    audio.sample_rate,
                    audio.channels,
                    audio.samples,
                )
            })
            .collect();
        assert_eq!(
            table,
            [
                ("flac", 44_100, 2, 4_096),
                ("mp3", 44_100, 2, 2_304),
                ("opus", 48_000, 2, 960),
                ("ogg", 44_100, 2, 1_024),
                ("m4a", 44_100, 2, 1_024),
                ("wav", 44_100, 2, 4),
                ("aiff", 44_100, 2, 4),
            ]
        );
    }

    #[test]
    fn writes_a_png_of_one_grey_pixel() {
        // Checked against an independent encoder: the chunk CRCs and the
        // Adler-32 are those zlib computes, and Pillow decodes the file to
        // one pixel of the shade 0x5A.
        assert_eq!(
            png(0x5A),
            cat(&[
                b"\x89PNG\r\n\x1A\n",
                b"\x00\x00\x00\x0DIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x00\x00\x00\x00",
                b"\x3A\x7E\x9B\x55",
                b"\x00\x00\x00\x0DIDAT\x78\x01\x01\x02\x00\xFD\xFF\x00\x5A\x00\x5C\x00\x5B",
                b"\xE1\x3A\x61\xE8",
                b"\x00\x00\x00\x00IEND\xAE\x42\x60\x82",
            ])
        );
    }

    #[test]
    fn writes_a_jpeg_of_one_grey_pixel() {
        // Pillow decodes these octets to one pixel of the shade 128.
        assert_eq!(
            jpeg(),
            hex(concat!(
                "ffd8ffe000104a46494600010100000100010000",
                "ffdb004300",
                "01010101010101010101010101010101",
                "01010101010101010101010101010101",
                "01010101010101010101010101010101",
                "01010101010101010101010101010101",
                "ffc0000b080001000101011100",
                "ffc40014000100000000000000000000000000000000",
                "ffc40014100100000000000000000000000000000000",
                "ffda0008010100003f003fffd9",
            ))
        );
    }

    #[test]
    fn writes_a_flac_track() {
        assert_eq!(
            sample(Format::Flac).encode(),
            cat(&[
                &stream_info(),
                // The last block: a Vorbis comment of 180 octets.
                b"\x84\x00\x00\xB4",
                &sample_comments(),
                FLAC_FRAME,
            ])
        );
    }

    #[test]
    fn writes_an_mp3_track() {
        assert_eq!(
            sample(Format::Mp3).encode(),
            cat(&[&sample_id3(), &mp3_frame(), &mp3_frame()])
        );
    }

    #[test]
    fn writes_an_opus_track() {
        // Each page: the capture pattern and version, the flags, the
        // granule position, the serial number 1, the sequence number, the
        // checksum (computed by an independent CRC), and the segment table.
        assert_eq!(
            sample(Format::Opus).encode(),
            cat(&[
                b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x00\x00\x00\x00\x02\x62\x85\x5C\x01\x13",
                // Version 1, two channels, a pre-skip of 312, 48 kHz input,
                // no gain, mapping family 0.
                b"OpusHead\x01\x02\x38\x01\x80\xBB\x00\x00\x00\x00\x00",
                b"OggS\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x01\x00\x00\x00\xF8\x96\x3D\xE1\x01\xBC",
                b"OpusTags",
                &sample_comments(),
                // The stream ends at 312 + 960 = 1,272 samples.
                b"OggS\x00\x04\xF8\x04\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x02\x00\x00\x00\x69\xD0\xD2\x56\x01\x01",
                b"\xFC",
            ])
        );
    }

    #[test]
    fn writes_a_vorbis_track() {
        assert_eq!(
            sample(Format::Vorbis).encode(),
            cat(&[
                b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x00\x00\x00\x00\x01\xA8\xAA\xB4\x01\x1E",
                // Version 0, two channels, 44.1 kHz, a nominal 112,000
                // bits per second, blocks of 2^8 and 2^11, the framing bit.
                b"\x01vorbis\x00\x00\x00\x00\x02\x44\xAC\x00\x00",
                b"\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x01",
                // Two packets: the comments (188 octets) and the setup (7).
                b"OggS\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x01\x00\x00\x00\xB1\x6A\x65\x20\x02\xBC\x07",
                b"\x03vorbis",
                &sample_comments(),
                b"\x01",
                b"\x05vorbis",
                // The stream ends at 1,024 samples.
                b"OggS\x00\x04\x00\x04\x00\x00\x00\x00\x00\x00",
                b"\x01\x00\x00\x00\x02\x00\x00\x00\xEA\xF8\xDC\xC9\x01\x01",
                b"\x00",
            ])
        );
    }

    #[test]
    fn writes_an_m4a_track() {
        let file = sample(Format::M4a).encode();
        assert_eq!(
            file,
            cat(&[
                b"\x00\x00\x00\x1CftypM4A \x00\x00\x00\x00M4A mp42isom",
                b"\x00\x00\x00\x0Cmdat\x00\x00\x00\x00",
                b"\x00\x00\x02\x43moov",
                b"\x00\x00\x01\x1Ctrak",
                b"\x00\x00\x01\x14mdia",
                // A timescale of 44,100 and a duration of 1,024.
                b"\x00\x00\x00\x20mdhd\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x00\x00\xAC\x44\x00\x00\x04\x00\x55\xC4\x00\x00",
                b"\x00\x00\x00\x21hdlr\x00\x00\x00\x00\x00\x00\x00\x00soun",
                &[0; 13],
                b"\x00\x00\x00\xCBminf",
                b"\x00\x00\x00\xC3stbl",
                b"\x00\x00\x00\x5Bstsd\x00\x00\x00\x00\x00\x00\x00\x01",
                // Two channels of 16 bits at 44,100 Hz.
                b"\x00\x00\x00\x4Bmp4a\x00\x00\x00\x00\x00\x00\x00\x01",
                b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x10\x00\x00\x00\x00",
                b"\xAC\x44\x00\x00",
                // MPEG-4 audio, an audio stream; then the configuration:
                // AAC LC, frequency index 4, two channels.
                b"\x00\x00\x00\x27esds\x00\x00\x00\x00\x03\x19\x00\x01\x00",
                b"\x04\x11\x40\x15\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x05\x02\x12\x10\x06\x01\x02",
                // One sample of 1,024 time units, in one chunk, 4 octets
                // long, 36 octets into the file.
                b"\x00\x00\x00\x18stts\x00\x00\x00\x00\x00\x00\x00\x01",
                b"\x00\x00\x00\x01\x00\x00\x04\x00",
                b"\x00\x00\x00\x1Cstsc\x00\x00\x00\x00\x00\x00\x00\x01",
                b"\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01",
                b"\x00\x00\x00\x18stsz\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x00\x00\x00\x01\x00\x00\x00\x04",
                b"\x00\x00\x00\x14stco\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x24",
                b"\x00\x00\x01\x1Fudta",
                b"\x00\x00\x01\x17meta\x00\x00\x00\x00",
                b"\x00\x00\x00\x21hdlr\x00\x00\x00\x00\x00\x00\x00\x00mdir",
                &[0; 13],
                b"\x00\x00\x00\xEAilst",
                b"\x00\x00\x00\x1B\xA9nam\x00\x00\x00\x13data\x00\x00\x00\x01\x00\x00\x00\x00One",
                b"\x00\x00\x00\x1B\xA9ART\x00\x00\x00\x13data\x00\x00\x00\x01\x00\x00\x00\x00Two",
                b"\x00\x00\x00\x1D\xA9alb\x00\x00\x00\x15data\x00\x00\x00\x01\x00\x00\x00\x00Three",
                b"\x00\x00\x00\x1CaART\x00\x00\x00\x14data\x00\x00\x00\x01\x00\x00\x00\x00Four",
                b"\x00\x00\x00\x20trkn\x00\x00\x00\x18data\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x00\x00\x00\x05\x00\x06\x00\x00",
                b"\x00\x00\x00\x1Edisk\x00\x00\x00\x16data\x00\x00\x00\x00\x00\x00\x00\x00",
                b"\x00\x00\x00\x07\x00\x08",
                b"\x00\x00\x00\x1C\xA9day\x00\x00\x00\x14data\x00\x00\x00\x01\x00\x00\x00\x001999",
                b"\x00\x00\x00\x19cpil\x00\x00\x00\x11data\x00\x00\x00\x15\x00\x00\x00\x00\x01",
            ])
        );
        // The chunk offset points at the sample.
        assert_eq!(&file[36..40], [0; 4]);
        assert_eq!(&file[28..36], b"\x00\x00\x00\x0Cmdat");
    }

    #[test]
    fn writes_a_wav_track() {
        assert_eq!(
            sample(Format::Wav).encode(),
            cat(&[
                b"RIFF\xB8\x00\x00\x00WAVE",
                // PCM, two channels, 44,100 Hz, 176,400 octets a second,
                // frames of 4 octets, 16 bits.
                b"fmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xAC\x00\x00",
                b"\x10\xB1\x02\x00\x04\x00\x10\x00",
                b"data\x10\x00\x00\x00",
                &[0; 16],
                b"id3 \x7C\x00\x00\x00",
                &sample_id3(),
            ])
        );
    }

    #[test]
    fn writes_an_aiff_track() {
        assert_eq!(
            sample(Format::Aiff).encode(),
            cat(&[
                b"FORM\x00\x00\x00\xC2AIFF",
                // Two channels, four sample frames, 16 bits, and 44,100 as
                // an 80-bit extended number.
                b"COMM\x00\x00\x00\x12\x00\x02\x00\x00\x00\x04\x00\x10",
                b"\x40\x0E\xAC\x44\x00\x00\x00\x00\x00\x00",
                b"SSND\x00\x00\x00\x18\x00\x00\x00\x00\x00\x00\x00\x00",
                &[0; 16],
                b"ID3 \x00\x00\x00\x7C",
                &sample_id3(),
            ])
        );
    }

    #[test]
    fn marks_only_a_compilation_as_one() {
        let track = Track {
            compilation: false,
            ..sample(Format::Flac)
        };
        let comments = track.comments();
        assert_eq!(&comments[163..], b"\x0D\x00\x00\x00COMPILATION=0");
        let id3 = track.id3();
        assert_eq!(&id3[112..], b"TCMP\x00\x00\x00\x02\x00\x00\x030");
        let items = track.items();
        assert_eq!(
            &items[201..],
            b"\x00\x00\x00\x19cpil\x00\x00\x00\x11data\x00\x00\x00\x15\x00\x00\x00\x00\x00"
        );
    }

    #[test]
    fn generates_nothing_for_an_empty_shape() {
        assert_eq!(generate(7, &Shape::default()), Library { files: vec![] });
        // Tracks without an album to hold them are no files either.
        let shape = Shape {
            tracks: 3,
            ..Shape::default()
        };
        assert_eq!(generate(7, &shape), Library { files: vec![] });
    }

    #[test]
    fn generates_only_what_the_shape_asks_for() {
        let shape = Shape {
            albums: 1,
            tracks: 1,
            ..Shape::default()
        };
        assert_eq!(
            manifest(&generate(7, &shape)),
            [track(
                "Hollow Echo 1/001 Cobalt Daybreak 1/101 Brass Garden.flac",
                Format::Flac,
                [
                    "Brass Garden",
                    "Hollow Echo 1",
                    "Cobalt Daybreak 1",
                    "Hollow Echo 1"
                ],
                [1, 1, 1, 1, 1986],
                false,
            )]
        );
    }

    /// The folders of the albums that the seed 7 and [`SMALL`] give. The
    /// names and years here and in the manifests below are those an
    /// independent `SplitMix64` draws for that seed: two words for the
    /// artist, two for the title, the year, then for each track two words
    /// for its title and, on a compilation, two for its artist.
    const FOLDERS: [&str; 5] = [
        "Hollow Echo 1/001 Cobalt Daybreak 1",
        "Dusty Echo 2/002 Golden Anthem 2",
        "Various Artists/003 Brass Canyon 3",
        "Faded Harbour 4/004 Greatest Hits",
        "Hollow Garden 5/005 Greatest Hits",
    ];

    /// The path of the file `name` in the album at `folder`.
    fn at(folder: usize, name: &str) -> String {
        format!("{}/{name}", FOLDERS[folder])
    }

    /// The manifest of the ordinary album and the set of two discs.
    fn one_artist_albums() -> Vec<(String, Yield)> {
        let plain = |title| [title, "Hollow Echo 1", "Cobalt Daybreak 1", "Hollow Echo 1"];
        let set = |title| [title, "Dusty Echo 2", "Golden Anthem 2", "Dusty Echo 2"];
        vec![
            track(
                &at(0, "101 Brass Garden.flac"),
                Format::Flac,
                plain("Brass Garden"),
                [1, 2, 1, 1, 1986],
                false,
            ),
            lyrics(
                &at(0, "101 Brass Garden.lrc"),
                "Brass Garden",
                "Cobalt Daybreak 1",
            ),
            track(
                &at(0, "102 Golden Bridge.flac"),
                Format::Flac,
                plain("Golden Bridge"),
                [2, 2, 1, 1, 1986],
                false,
            ),
            cover(&at(0, "cover.png"), "image/png"),
            track(
                &at(1, "101 Amber Harbour.mp3"),
                Format::Mp3,
                set("Amber Harbour"),
                [1, 2, 1, 2, 1998],
                false,
            ),
            lyrics(
                &at(1, "101 Amber Harbour.lrc"),
                "Amber Harbour",
                "Golden Anthem 2",
            ),
            track(
                &at(1, "102 Hollow Fable.mp3"),
                Format::Mp3,
                set("Hollow Fable"),
                [2, 2, 1, 2, 1998],
                false,
            ),
            track(
                &at(1, "201 Amber Harbour.mp3"),
                Format::Mp3,
                set("Amber Harbour"),
                [1, 2, 2, 2, 1998],
                false,
            ),
            track(
                &at(1, "202 Faded Fable.mp3"),
                Format::Mp3,
                set("Faded Fable"),
                [2, 2, 2, 2, 1998],
                false,
            ),
            cover(&at(1, "cover.jpg"), "image/jpeg"),
        ]
    }

    /// The manifest of the compilation and the two same-titled albums.
    fn shared_name_albums() -> Vec<(String, Yield)> {
        let various = |title, artist| [title, artist, "Brass Canyon 3", "Various Artists"];
        let first = |title| [title, "Faded Harbour 4", "Greatest Hits", "Faded Harbour 4"];
        let second = |title| [title, "Hollow Garden 5", "Greatest Hits", "Hollow Garden 5"];
        vec![
            track(
                &at(2, "101 Hollow Fable.opus"),
                Format::Opus,
                various("Hollow Fable", "Amber Echo"),
                [1, 2, 1, 1, 1991],
                true,
            ),
            lyrics(
                &at(2, "101 Hollow Fable.lrc"),
                "Hollow Fable",
                "Brass Canyon 3",
            ),
            track(
                &at(2, "102 Amber Canyon.opus"),
                Format::Opus,
                various("Amber Canyon", "Brass Daybreak"),
                [2, 2, 1, 1, 1991],
                true,
            ),
            cover(&at(2, "cover.png"), "image/png"),
            track(
                &at(3, "101 Amber Echo.ogg"),
                Format::Vorbis,
                first("Amber Echo"),
                [1, 2, 1, 1, 1969],
                false,
            ),
            lyrics(&at(3, "101 Amber Echo.lrc"), "Amber Echo", "Greatest Hits"),
            track(
                &at(3, "102 Cobalt Garden.ogg"),
                Format::Vorbis,
                first("Cobalt Garden"),
                [2, 2, 1, 1, 1969],
                false,
            ),
            cover(&at(3, "cover.jpg"), "image/jpeg"),
            track(
                &at(4, "101 Cobalt Canyon.m4a"),
                Format::M4a,
                second("Cobalt Canyon"),
                [1, 2, 1, 1, 1970],
                false,
            ),
            lyrics(
                &at(4, "101 Cobalt Canyon.lrc"),
                "Cobalt Canyon",
                "Greatest Hits",
            ),
            track(
                &at(4, "102 Golden Daybreak.m4a"),
                Format::M4a,
                second("Golden Daybreak"),
                [2, 2, 1, 1, 1970],
                false,
            ),
            cover(&at(4, "cover.png"), "image/png"),
        ]
    }

    /// A manifest entry for a tagged track of the album of oddities.
    fn odd_entry(
        name: &str,
        format: Format,
        title: &str,
        number: u16,
        oddity: Oddity,
    ) -> (String, Yield) {
        (
            format!("Odd Tags/Oddities/{name}"),
            Yield::Track(Track {
                format,
                title: title.to_owned(),
                artist: "Odd Tags".to_owned(),
                album: "Oddities".to_owned(),
                album_artist: "Odd Tags".to_owned(),
                track: number,
                track_total: 3,
                disc: 1,
                disc_total: 1,
                year: 2001,
                compilation: false,
                oddity: Some(oddity),
            }),
        )
    }

    /// The manifest of the album of oddities and the damaged files.
    fn extras() -> Vec<(String, Yield)> {
        let cut = |name: &str, format| (format!("Damaged/{name}"), Yield::Damaged(format));
        vec![
            odd_entry(
                "101 Disagreeing.mp3",
                Format::Mp3,
                "Disagreeing",
                1,
                Oddity::Id3v1Disagrees,
            ),
            odd_entry(
                "102 Slashed.flac",
                Format::Flac,
                "Slashed",
                2,
                Oddity::SlashedNumbers,
            ),
            (
                "Odd Tags/Oddities/103 Untagged.wav".to_owned(),
                Yield::Track(Track {
                    format: Format::Wav,
                    title: String::new(),
                    artist: String::new(),
                    album: String::new(),
                    album_artist: String::new(),
                    track: 0,
                    track_total: 0,
                    disc: 0,
                    disc_total: 0,
                    year: 0,
                    compilation: false,
                    oddity: Some(Oddity::Untagged),
                }),
            ),
            cut("cut.flac", Format::Flac),
            cut("cut.mp3", Format::Mp3),
            cut("cut.opus", Format::Opus),
            cut("cut.ogg", Format::Vorbis),
            cut("cut.m4a", Format::M4a),
            cut("cut.wav", Format::Wav),
            cut("cut.aiff", Format::Aiff),
        ]
    }

    #[test]
    fn generates_the_manifest_of_a_small_library() {
        assert_eq!(
            manifest(&generate(7, &SMALL)),
            [one_artist_albums(), shared_name_albums(), extras()].concat()
        );
    }

    #[test]
    fn writes_each_file_from_what_its_manifest_entry_says() {
        let library = generate(7, &SMALL);
        let files = files(&library);
        // A track is its manifest entry, encoded.
        let first = Track {
            format: Format::Flac,
            title: "Brass Garden".to_owned(),
            artist: "Hollow Echo 1".to_owned(),
            album: "Cobalt Daybreak 1".to_owned(),
            album_artist: "Hollow Echo 1".to_owned(),
            track: 1,
            track_total: 2,
            disc: 1,
            disc_total: 1,
            year: 1986,
            compilation: false,
            oddity: None,
        };
        assert_eq!(
            files["Hollow Echo 1/001 Cobalt Daybreak 1/101 Brass Garden.flac"],
            cat(&[
                &stream_info(),
                b"\x84\x00\x00\xDC",
                b"\x10\x00\x00\x00gunmetal-testkit",
                b"\x0A\x00\x00\x00",
                b"\x12\x00\x00\x00TITLE=Brass Garden",
                b"\x14\x00\x00\x00ARTIST=Hollow Echo 1",
                b"\x17\x00\x00\x00ALBUM=Cobalt Daybreak 1",
                b"\x19\x00\x00\x00ALBUMARTIST=Hollow Echo 1",
                b"\x0D\x00\x00\x00TRACKNUMBER=1",
                b"\x0C\x00\x00\x00TRACKTOTAL=2",
                b"\x0C\x00\x00\x00DISCNUMBER=1",
                b"\x0B\x00\x00\x00DISCTOTAL=1",
                b"\x09\x00\x00\x00DATE=1986",
                b"\x0D\x00\x00\x00COMPILATION=0",
                FLAC_FRAME,
            ])
        );
        assert_eq!(
            files["Hollow Echo 1/001 Cobalt Daybreak 1/101 Brass Garden.flac"],
            first.encode()
        );
        assert_eq!(
            files["Hollow Echo 1/001 Cobalt Daybreak 1/101 Brass Garden.lrc"],
            b"[00:00.00]Brass Garden\n[00:02.50]Cobalt Daybreak 1\n"
        );
        // The seed draws the shade of a PNG cover: 105 for the first album.
        // The checksums are those an independent encoder computes.
        assert_eq!(
            files["Hollow Echo 1/001 Cobalt Daybreak 1/cover.png"],
            hex(concat!(
                "89504e470d0a1a0a",
                "0000000d49484452000000010000000108000000003a7e9b55",
                "0000000d494441547801010200fdff0069006b006a77415091",
                "0000000049454e44ae426082",
            ))
        );
        assert_eq!(files["Dusty Echo 2/002 Golden Anthem 2/cover.jpg"], jpeg());
    }

    #[test]
    fn writes_the_odd_tags_the_manifest_names() {
        let library = generate(7, &SMALL);
        let files = files(&library);
        // The ID3v2 tag says "Disagreeing" by "Odd Tags"; the ID3v1 tag at
        // the end says otherwise.
        assert_eq!(
            files["Odd Tags/Oddities/101 Disagreeing.mp3"],
            cat(&[
                b"ID3\x04\x00\x00\x00\x00\x01\x06",
                b"TIT2\x00\x00\x00\x0C\x00\x00\x03Disagreeing",
                b"TPE1\x00\x00\x00\x09\x00\x00\x03Odd Tags",
                b"TALB\x00\x00\x00\x09\x00\x00\x03Oddities",
                b"TPE2\x00\x00\x00\x09\x00\x00\x03Odd Tags",
                b"TRCK\x00\x00\x00\x04\x00\x00\x031/3",
                b"TPOS\x00\x00\x00\x04\x00\x00\x031/1",
                b"TDRC\x00\x00\x00\x05\x00\x00\x032001",
                b"TCMP\x00\x00\x00\x02\x00\x00\x030",
                &mp3_frame(),
                &mp3_frame(),
                b"TAGAgreeable",
                &[0; 21],
                b"Someone Else",
                &[0; 18],
                &[0; 65],
            ])
        );
        assert_eq!(
            files["Odd Tags/Oddities/102 Slashed.flac"],
            cat(&[
                &stream_info(),
                b"\x84\x00\x00\x98",
                b"\x10\x00\x00\x00gunmetal-testkit",
                b"\x07\x00\x00\x00",
                b"\x0D\x00\x00\x00title=Slashed",
                b"\x0F\x00\x00\x00artist=Odd Tags",
                b"\x0E\x00\x00\x00album=Oddities",
                b"\x14\x00\x00\x00albumartist=Odd Tags",
                b"\x0F\x00\x00\x00tracknumber=2/3",
                b"\x0E\x00\x00\x00discnumber=1/1",
                b"\x09\x00\x00\x00date=2001",
                FLAC_FRAME,
            ])
        );
        assert_eq!(
            files["Odd Tags/Oddities/103 Untagged.wav"],
            cat(&[
                b"RIFF\x34\x00\x00\x00WAVE",
                b"fmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xAC\x00\x00",
                b"\x10\xB1\x02\x00\x04\x00\x10\x00",
                b"data\x10\x00\x00\x00",
                &[0; 16],
            ])
        );
    }

    #[test]
    fn cuts_each_damaged_file_short_inside_its_header() {
        let library = generate(7, &SMALL);
        let damaged: Vec<(&str, &[u8])> = library
            .files
            .iter()
            .filter(|file| file.folder == "Damaged")
            .map(|file| (file.name.as_str(), file.bytes.as_slice()))
            .collect();
        // Each keeps the 12 octets that name its format and declare more
        // than the file holds: a 34-octet STREAMINFO block, a tag of 132
        // octets after its header, an Ogg page, a 28-octet box, and forms
        // of 202 and 212 octets.
        let expected: [(&str, &[u8]); 7] = [
            ("cut.flac", b"fLaC\x00\x00\x00\x22\x10\x00\x10\x00"),
            ("cut.mp3", b"ID3\x04\x00\x00\x00\x00\x01\x04TI"),
            ("cut.opus", b"OggS\x00\x02\x00\x00\x00\x00\x00\x00"),
            ("cut.ogg", b"OggS\x00\x02\x00\x00\x00\x00\x00\x00"),
            ("cut.m4a", b"\x00\x00\x00\x1CftypM4A "),
            ("cut.wav", b"RIFF\xCA\x00\x00\x00WAVE"),
            ("cut.aiff", b"FORM\x00\x00\x00\xD4AIFF"),
        ];
        assert_eq!(damaged, expected);
    }

    #[test]
    fn gives_the_same_library_for_the_same_seed_and_another_for_another() {
        assert_eq!(generate(7, &SMALL), generate(7, &SMALL));
        let one = manifest(&generate(7, &SMALL));
        let other = manifest(&generate(8, &SMALL));
        assert_ne!(one[0], other[0]);
        // What the seed does not choose stays as it is.
        assert_eq!(one[22..], other[22..]);
    }

    #[test]
    fn takes_the_formats_in_turn_so_seven_albums_cover_them_all() {
        let library = generate(7, &EVERY_FORMAT);
        let formats: Vec<Format> = library
            .files
            .iter()
            .filter_map(|file| match &file.yields {
                Yield::Track(track) => Some(track.format),
                _ => None,
            })
            .collect();
        assert_eq!(formats, Format::ALL);
        let covers: Vec<&str> = library
            .files
            .iter()
            .filter(|file| file.name.starts_with("cover"))
            .map(|file| file.name.as_str())
            .collect();
        assert_eq!(
            covers,
            [
                "cover.png",
                "cover.jpg",
                "cover.png",
                "cover.jpg",
                "cover.png",
                "cover.jpg",
                "cover.png"
            ]
        );
    }

    #[test]
    fn every_flac_frame_carries_the_checksums_of_its_octets() {
        let library = generate(7, &EVERY_FORMAT);
        let frames: Vec<&[u8]> = library
            .files
            .iter()
            .filter(|file| is_track_of(file, &[Format::Flac]))
            .map(|file| &file.bytes[file.bytes.len() - 14..])
            .collect();
        assert_eq!(frames.len(), 1);
        for frame in frames {
            assert_eq!(frame[5], crc8_flac(&frame[..5]));
            assert_eq!(frame[12..], crc16_flac(&frame[..12]).to_be_bytes());
        }
    }

    #[test]
    fn every_ogg_page_carries_the_checksum_of_its_octets() {
        let library = generate(7, &EVERY_FORMAT);
        let streams: Vec<&[u8]> = library
            .files
            .iter()
            .filter(|file| is_track_of(file, &[Format::Opus, Format::Vorbis]))
            .map(|file| file.bytes.as_slice())
            .collect();
        assert_eq!(streams.len(), 2);
        for stream in streams {
            let pages = pages(stream);
            assert_eq!(pages.len(), 3);
            for page in pages {
                let mut zeroed = page.to_vec();
                zeroed[22..26].fill(0);
                assert_eq!(page[..4], *b"OggS");
                assert_eq!(page[22..26], crc32_ogg(&zeroed).to_le_bytes());
            }
        }
    }

    #[test]
    fn every_png_chunk_carries_the_checksum_of_its_octets() {
        let library = generate(7, &EVERY_FORMAT);
        let covers: Vec<&[u8]> = library
            .files
            .iter()
            .filter(|file| file.name == "cover.png")
            .map(|file| file.bytes.as_slice())
            .collect();
        assert_eq!(covers.len(), 4);
        for cover in covers {
            // The signature, then IHDR at 8, IDAT at 33 and IEND at 58:
            // a length, the checked octets, and their CRC.
            assert_eq!(cover.len(), 70);
            for (start, len) in [(8, 13), (33, 13), (58, 0)] {
                let checked = &cover[start + 4..start + 8 + len];
                let crc = &cover[start + 8 + len..start + 12 + len];
                assert_eq!(cover[start..start + 4], [0, 0, 0, len.to_le_bytes()[0]]);
                assert_eq!(crc, crc32_ieee(checked).to_be_bytes());
            }
            // The scanline in its stored block, then its Adler-32.
            assert_eq!(cover[50..54], adler32(&cover[48..50]).to_be_bytes());
        }
    }

    #[test]
    fn writes_the_same_tree_twice_and_it_holds_every_file() {
        let library = generate(7, &SMALL);
        let first = TempDir::new("testkit-library").unwrap();
        let second = TempDir::new("testkit-library").unwrap();
        assert_eq!(library.write_to(first.path()).ok(), Some(()));
        assert_eq!(generate(7, &SMALL).write_to(second.path()).ok(), Some(()));
        let written = tree(first.path());
        assert_eq!(written.len(), 32);
        assert_eq!(written, files(&library));
        assert_eq!(written, tree(second.path()));
    }

    #[test]
    fn passes_on_the_error_of_a_folder_it_cannot_create() {
        let dir = TempDir::new("testkit-library").unwrap();
        // A file sits where the artist's folder should go.
        fs::write(dir.path().join("Hollow Echo 1"), b"in the way").unwrap();
        let error = generate(7, &SMALL).write_to(dir.path()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotADirectory);
        assert_eq!(tree(dir.path()).len(), 1);
    }

    #[test]
    fn passes_on_the_error_of_a_file_it_cannot_write() {
        let dir = TempDir::new("testkit-library").unwrap();
        // A folder sits where the second track should go.
        let album = dir.path().join("Hollow Echo 1/001 Cobalt Daybreak 1");
        fs::create_dir_all(album.join("102 Golden Bridge.flac")).unwrap();
        let library = generate(7, &SMALL);
        let error = library.write_to(dir.path()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::IsADirectory);
        // The files before it stay written.
        assert_eq!(
            tree(dir.path()),
            files(&Library {
                files: library.files[..2].to_vec()
            })
        );
    }
}
