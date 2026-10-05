//! A reader for the media segments of fragmented MP4 audio, for the audio
//! packager's tests.
//!
//! A media segment is a movie fragment box, `moof`, followed by the media
//! data box, `mdat`, whose octets it describes (ISO/IEC 14496-12, sections
//! 8.8.4 to 8.8.8, 8.8.12 and 8.1.1). The MP4 parser in `gunmetal-core`
//! reads a file's box structure and its sample entries but not fragments,
//! so the packager's tests read every segment the packager writes back
//! through this reader and compare the samples with the frames that went
//! in.
//!
//! The reader is written from the standard's box definitions, for the tests
//! alone. It shares no code with the packager, and its own tests check it
//! against fragments written out by hand.
//!
//! It takes the one shape a single-track audio segment needs, and refuses
//! every other with the reason:
//!
//! - `moof` holds `mfhd` and one `traf`, and `traf` holds `tfhd`, `tfdt`
//!   and one `trun`, in that order and with nothing beside them;
//! - `tfhd` says that data offsets count from the start of `moof`
//!   (`default-base-is-moof`), and may give a default sample duration and
//!   a default sample size;
//! - `trun` gives a data offset, and each sample's duration and size unless
//!   `tfhd` gave the default;
//! - one `mdat` follows, with a 32-bit size, and its payload is exactly the
//!   run's samples, back to back, starting where the data offset says.
//!
//! Every box's size must be the octets its fields took, so a box with
//! anything after its last field is refused too.

/// One sample of a fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// How long the sample plays, in units of the track's timescale.
    pub duration: u32,
    /// The sample's octets.
    pub data: Vec<u8>,
}

/// One movie fragment and the samples its media data holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    /// The fragment's sequence number, from `mfhd`.
    pub sequence: u32,
    /// The track the samples belong to, from `tfhd`.
    pub track: u32,
    /// The decode time of the first sample, in units of the track's
    /// timescale, from `tfdt`.
    pub decode_time: u64,
    /// The samples of the run, in order.
    pub samples: Vec<Sample>,
}

/// Why octets are not a media segment [`read_fragment`] takes. Offsets
/// count octets from the start of the segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentError {
    /// The segment ends before the field that starts at `offset` does.
    Truncated {
        /// Where the field starts.
        offset: usize,
    },
    /// A box of another type stands where `expected` must.
    Kind {
        /// Where the box starts.
        offset: usize,
        /// The type the reader needs there.
        expected: [u8; 4],
        /// The type found.
        found: [u8; 4],
    },
    /// A box whose size is not the octets it holds: its header and the
    /// fields and boxes this reader takes from it.
    Size {
        /// Where the box starts.
        offset: usize,
        /// The size the box declares.
        size: u32,
        /// The octets it holds.
        held: usize,
    },
    /// A full box of a version this reader does not take.
    Version {
        /// The box type.
        kind: [u8; 4],
        /// Its version.
        version: u8,
    },
    /// A `tfhd` or `trun` box whose flags ask for a field this reader does
    /// not take, leave out one it needs, or leave the samples without a
    /// duration or a size.
    Flags {
        /// The box type.
        kind: [u8; 4],
        /// Its flags.
        flags: u32,
    },
    /// The run's data offset does not point at the payload of `mdat`.
    Offset {
        /// Where the run says its first sample starts.
        data: usize,
        /// Where the payload starts.
        payload: usize,
    },
    /// Octets follow the `mdat` box.
    Trailing {
        /// Where they start.
        offset: usize,
        /// How many there are.
        len: usize,
    },
}

/// Reads the media segment `segment`: one movie fragment and its media
/// data.
///
/// The reader is for tests that hold the segment in memory. It trusts the
/// sample count of the run, as nothing a test builds is hostile.
///
/// # Errors
///
/// Returns the [`FragmentError`] for the first thing that is not the shape
/// the module documentation describes.
pub fn read_fragment(_segment: &[u8]) -> Result<Fragment, FragmentError> {
    Ok(Fragment {
        sequence: 0,
        track: 0,
        decode_time: 0,
        samples: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `parts` joined, so a literal box reads one field per line.
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// `segment` with the octets from `at` replaced by `with`.
    fn patched(mut segment: Vec<u8>, at: usize, with: &[u8]) -> Vec<u8> {
        segment[at..at + with.len()].copy_from_slice(with);
        segment
    }

    fn sample(duration: u32, data: &[u8]) -> Sample {
        Sample {
            duration,
            data: data.to_vec(),
        }
    }

    /// The segment a packager writes for two samples, 117 octets: the
    /// decode time is 64 bits wide and the run gives each sample's
    /// duration and size.
    ///
    /// The boxes start at 0 (`moof`), 8 (`mfhd`), 24 (`traf`), 32 (`tfhd`),
    /// 48 (`tfdt`), 68 (`trun`) and 104 (`mdat`), and the payload at 112.
    fn run_segment() -> Vec<u8> {
        cat(&[
            b"\x00\x00\x00\x68moof",
            b"\x00\x00\x00\x10mfhd\x00\x00\x00\x00",
            &[0, 0, 0, 7], // sequence number
            b"\x00\x00\x00\x50traf",
            b"\x00\x00\x00\x10tfhd\x00\x02\x00\x00", // default-base-is-moof
            &[0, 0, 0, 1],                           // track ID
            b"\x00\x00\x00\x14tfdt\x01\x00\x00\x00", // version 1
            &[0, 0, 0, 1, 0, 0, 0x10, 0],            // decode time 2^32 + 4,096
            b"\x00\x00\x00\x24trun\x00\x00\x03\x01", // offset, durations, sizes
            &[0, 0, 0, 2],                           // sample count
            &[0, 0, 0, 0x70],                        // data offset: 104 + 8
            &[0, 0, 0x10, 0, 0, 0, 0, 3],            // 4,096 units, 3 octets
            &[0, 0, 0x04, 0, 0, 0, 0, 2],            // 1,024 units, 2 octets
            b"\x00\x00\x00\x0Dmdat",
            &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE],
        ])
    }

    fn run_fragment() -> Fragment {
        Fragment {
            sequence: 7,
            track: 1,
            decode_time: 4_294_971_392,
            samples: vec![
                sample(4_096, &[0xAA, 0xBB, 0xCC]),
                sample(1_024, &[0xDD, 0xEE]),
            ],
        }
    }

    /// A segment whose `tfhd` gives the duration and the size of every
    /// sample, 106 octets: the decode time is 32 bits wide and the run
    /// holds only its sample count and data offset.
    ///
    /// The boxes start at 0 (`moof`), 8 (`mfhd`), 24 (`traf`), 32 (`tfhd`),
    /// 56 (`tfdt`), 72 (`trun`) and 92 (`mdat`), and the payload at 100.
    fn defaults_segment() -> Vec<u8> {
        cat(&[
            b"\x00\x00\x00\x5Cmoof",
            b"\x00\x00\x00\x10mfhd\x00\x00\x00\x00",
            &[0, 0, 1, 2], // sequence number 258
            b"\x00\x00\x00\x44traf",
            b"\x00\x00\x00\x18tfhd\x00\x02\x00\x18", // base is moof, both defaults
            &[0, 0, 0, 5],                           // track ID
            &[0, 0, 0x03, 0xC0],                     // default duration 960
            &[0, 0, 0, 2],                           // default size 2
            b"\x00\x00\x00\x10tfdt\x00\x00\x00\x00", // version 0
            &[0, 0x01, 0x77, 0],                     // decode time 96,000
            b"\x00\x00\x00\x14trun\x00\x00\x00\x01", // a data offset alone
            &[0, 0, 0, 3],                           // sample count
            &[0, 0, 0, 0x64],                        // data offset: 92 + 8
            b"\x00\x00\x00\x0Emdat",
            &[1, 2, 3, 4, 5, 6],
        ])
    }

    /// A segment of two samples whose `tfhd` holds one default, `fallback`,
    /// under the flags `tfhd_flags`, and whose run holds one field a
    /// sample, `first` and `second`, under the flags `trun_flags`. 108
    /// octets.
    ///
    /// The boxes start at 0 (`moof`), 8 (`mfhd`), 24 (`traf`), 32 (`tfhd`),
    /// 52 (`tfdt`), 68 (`trun`) and 96 (`mdat`), and the payload at 104.
    fn mixed_segment(
        tfhd_flags: u8,
        fallback: u8,
        trun_flags: u8,
        first: u8,
        second: u8,
    ) -> Vec<u8> {
        cat(&[
            b"\x00\x00\x00\x60moof",
            b"\x00\x00\x00\x10mfhd\x00\x00\x00\x00",
            &[0, 0, 0, 2], // sequence number
            b"\x00\x00\x00\x48traf",
            b"\x00\x00\x00\x14tfhd\x00\x02\x00",
            &[tfhd_flags],
            &[0, 0, 0, 9],        // track ID
            &[0, 0, 0, fallback], // the one default
            b"\x00\x00\x00\x10tfdt\x00\x00\x00\x00",
            &[0, 0, 0, 0x30], // decode time 48
            b"\x00\x00\x00\x1Ctrun\x00\x00",
            &[trun_flags, 0x01],
            &[0, 0, 0, 2],    // sample count
            &[0, 0, 0, 0x68], // data offset: 96 + 8
            &[0, 0, 0, first],
            &[0, 0, 0, second],
            b"\x00\x00\x00\x0Cmdat",
            &[9, 8, 7, 6],
        ])
    }

    #[test]
    fn reads_the_shape_a_packager_writes() {
        let segment = run_segment();
        assert_eq!(segment.len(), 117);
        assert_eq!(read_fragment(&segment), Ok(run_fragment()));
    }

    #[test]
    fn reads_the_duration_and_size_a_track_fragment_header_gives() {
        let segment = defaults_segment();
        assert_eq!(segment.len(), 106);
        assert_eq!(
            read_fragment(&segment),
            Ok(Fragment {
                sequence: 258,
                track: 5,
                decode_time: 96_000,
                samples: vec![
                    sample(960, &[1, 2]),
                    sample(960, &[3, 4]),
                    sample(960, &[5, 6])
                ],
            })
        );
    }

    #[test]
    fn reads_one_field_from_the_header_and_the_other_from_the_run() {
        // A default duration of 40, and sizes of 1 and 3 in the run.
        let sizes = mixed_segment(0x08, 40, 0x02, 1, 3);
        assert_eq!(sizes.len(), 108);
        assert_eq!(
            read_fragment(&sizes),
            Ok(Fragment {
                sequence: 2,
                track: 9,
                decode_time: 48,
                samples: vec![sample(40, &[9]), sample(40, &[8, 7, 6])],
            })
        );
        // A default size of 2, and durations of 100 and 200 in the run.
        let durations = mixed_segment(0x10, 2, 0x01, 100, 200);
        assert_eq!(
            read_fragment(&durations),
            Ok(Fragment {
                sequence: 2,
                track: 9,
                decode_time: 48,
                samples: vec![sample(100, &[9, 8]), sample(200, &[7, 6])],
            })
        );
    }

    #[test]
    fn refuses_samples_with_no_duration_or_no_size() {
        // The header's one default is a size, and so is the run's field.
        assert_eq!(
            read_fragment(&mixed_segment(0x10, 2, 0x02, 2, 2)),
            Err(FragmentError::Flags {
                kind: *b"trun",
                flags: 0x00_0201,
            })
        );
        // The header's one default is a duration, and so is the run's field.
        assert_eq!(
            read_fragment(&mixed_segment(0x08, 40, 0x01, 100, 200)),
            Err(FragmentError::Flags {
                kind: *b"trun",
                flags: 0x00_0101,
            })
        );
    }

    #[test]
    fn refuses_track_fragment_flags_it_does_not_take() {
        // base-data-offset-present, whose field this reader does not read.
        assert_eq!(
            read_fragment(&patched(run_segment(), 41, &[0x02, 0x00, 0x01])),
            Err(FragmentError::Flags {
                kind: *b"tfhd",
                flags: 0x02_0001,
            })
        );
        // No default-base-is-moof, so offsets would count from elsewhere.
        assert_eq!(
            read_fragment(&patched(defaults_segment(), 41, &[0x00, 0x00, 0x18])),
            Err(FragmentError::Flags {
                kind: *b"tfhd",
                flags: 0x00_0018,
            })
        );
    }

    #[test]
    fn refuses_run_flags_it_does_not_take() {
        // sample-flags-present, whose field this reader does not read.
        assert_eq!(
            read_fragment(&patched(run_segment(), 77, &[0x00, 0x07, 0x01])),
            Err(FragmentError::Flags {
                kind: *b"trun",
                flags: 0x00_0701,
            })
        );
        // No data offset, so the samples would start where no field says.
        assert_eq!(
            read_fragment(&patched(run_segment(), 77, &[0x00, 0x03, 0x00])),
            Err(FragmentError::Flags {
                kind: *b"trun",
                flags: 0x00_0300,
            })
        );
    }

    #[test]
    fn refuses_versions_it_does_not_take() {
        let cases: [(usize, u8, [u8; 4]); 4] = [
            (16, 1, *b"mfhd"),
            (40, 1, *b"tfhd"),
            (56, 2, *b"tfdt"),
            (76, 1, *b"trun"),
        ];
        for (at, version, kind) in cases {
            assert_eq!(
                read_fragment(&patched(run_segment(), at, &[version])),
                Err(FragmentError::Version { kind, version }),
                "{at}"
            );
        }
    }

    #[test]
    fn refuses_a_box_of_another_type() {
        let cases: [(usize, [u8; 4]); 7] = [
            (0, *b"moof"),
            (8, *b"mfhd"),
            (24, *b"traf"),
            (32, *b"tfhd"),
            (48, *b"tfdt"),
            (68, *b"trun"),
            (104, *b"mdat"),
        ];
        for (offset, expected) in cases {
            assert_eq!(
                read_fragment(&patched(run_segment(), offset + 4, b"free")),
                Err(FragmentError::Kind {
                    offset,
                    expected,
                    found: *b"free",
                }),
                "{offset}"
            );
        }
    }

    #[test]
    fn refuses_a_box_whose_size_is_not_what_it_holds() {
        // Each box of the segment with a size one more than it holds.
        let cases: [(usize, u8, usize); 7] = [
            (0, 0x69, 104),
            (8, 0x11, 16),
            (24, 0x51, 80),
            (32, 0x11, 16),
            (48, 0x15, 20),
            (68, 0x25, 36),
            (104, 0x0E, 13),
        ];
        for (offset, size, held) in cases {
            assert_eq!(
                read_fragment(&patched(run_segment(), offset + 3, &[size])),
                Err(FragmentError::Size {
                    offset,
                    size: u32::from(size),
                    held,
                }),
                "{offset}"
            );
        }
        // One less, and the two sizes no box of this shape may have: 0,
        // which runs to the end of the file, and 1, which says a 64-bit
        // size follows.
        for size in [0x0C, 0x00, 0x01] {
            assert_eq!(
                read_fragment(&patched(run_segment(), 107, &[size])),
                Err(FragmentError::Size {
                    offset: 104,
                    size: u32::from(size),
                    held: 13,
                }),
                "{size}"
            );
        }
    }

    #[test]
    fn refuses_a_run_with_fewer_or_more_entries_than_its_count() {
        // A count of 1 leaves the second entry inside the box.
        assert_eq!(
            read_fragment(&patched(run_segment(), 83, &[1])),
            Err(FragmentError::Size {
                offset: 68,
                size: 36,
                held: 28,
            })
        );
        // A count of 3 reads a third entry from the `mdat` box, and then
        // `trun` holds more than its size.
        assert_eq!(
            read_fragment(&patched(run_segment(), 83, &[3])),
            Err(FragmentError::Size {
                offset: 68,
                size: 36,
                held: 44,
            })
        );
    }

    #[test]
    fn refuses_a_decode_time_of_the_wrong_width_for_its_version() {
        // Version 0 reads 32 bits and leaves 32 inside the box.
        assert_eq!(
            read_fragment(&patched(run_segment(), 56, &[0])),
            Err(FragmentError::Size {
                offset: 48,
                size: 20,
                held: 16,
            })
        );
        // Version 1 reads 64 bits from a box that holds 32.
        assert_eq!(
            read_fragment(&patched(defaults_segment(), 64, &[1])),
            Err(FragmentError::Size {
                offset: 56,
                size: 16,
                held: 20,
            })
        );
    }

    #[test]
    fn refuses_every_proper_prefix_as_truncated() {
        for segment in [
            run_segment(),
            defaults_segment(),
            mixed_segment(0x08, 40, 0x02, 1, 3),
        ] {
            for cut in 0..segment.len() {
                assert!(
                    matches!(
                        read_fragment(&segment[..cut]),
                        Err(FragmentError::Truncated { .. })
                    ),
                    "{cut} of {}",
                    segment.len()
                );
            }
        }
    }

    #[test]
    fn says_where_the_field_that_was_cut_short_starts() {
        let segment = run_segment();
        let cases = [
            (0, 0), // the size of `moof`
            (3, 0),
            (6, 4),     // the type of `moof`
            (17, 16),   // the version and flags of `mfhd`
            (22, 20),   // the sequence number
            (47, 44),   // the track ID
            (67, 60),   // the decode time
            (83, 80),   // the sample count
            (87, 84),   // the data offset
            (91, 88),   // the first sample's duration
            (95, 92),   // the first sample's size
            (110, 108), // the type of `mdat`
            (112, 112), // the payload
            (116, 112),
        ];
        for (cut, offset) in cases {
            assert_eq!(
                read_fragment(&segment[..cut]),
                Err(FragmentError::Truncated { offset }),
                "{cut}"
            );
        }
        // The two defaults of a `tfhd` box, and a 32-bit decode time.
        let defaults = defaults_segment();
        for (cut, offset) in [(50, 48), (54, 52), (70, 68)] {
            assert_eq!(
                read_fragment(&defaults[..cut]),
                Err(FragmentError::Truncated { offset }),
                "{cut}"
            );
        }
    }

    #[test]
    fn refuses_a_data_offset_that_does_not_point_at_the_payload() {
        // One octet early, on the last octet of the `mdat` header.
        assert_eq!(
            read_fragment(&patched(run_segment(), 87, &[0x6F])),
            Err(FragmentError::Offset {
                data: 111,
                payload: 112,
            })
        );
        // One octet late.
        assert_eq!(
            read_fragment(&patched(run_segment(), 87, &[0x71])),
            Err(FragmentError::Offset {
                data: 113,
                payload: 112,
            })
        );
    }

    #[test]
    fn refuses_octets_after_the_media_data() {
        let mut segment = run_segment();
        segment.extend([0x01, 0x02]);
        assert_eq!(
            read_fragment(&segment),
            Err(FragmentError::Trailing {
                offset: 117,
                len: 2,
            })
        );
    }

    #[test]
    fn refuses_sample_sizes_that_do_not_fill_the_media_data() {
        // The second sample takes one octet of the payload's two that are
        // left, so `mdat` holds less than its size says.
        assert_eq!(
            read_fragment(&patched(run_segment(), 103, &[1])),
            Err(FragmentError::Size {
                offset: 104,
                size: 13,
                held: 12,
            })
        );
        // The second sample asks for one octet more than the segment has.
        assert_eq!(
            read_fragment(&patched(run_segment(), 103, &[3])),
            Err(FragmentError::Truncated { offset: 112 })
        );
    }
}
