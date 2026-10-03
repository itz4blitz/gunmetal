//! Builders for the bodies of the MP4 sample table boxes: `stts`, `stsc`,
//! `stsz`, `stz2`, `stco` and `co64` (ISO/IEC 14496-12, sections 8.6.1.2,
//! 8.7.3, 8.7.4 and 8.7.5).
//!
//! A body is everything in the box after its size and type, starting with
//! the full-box version and flags; [`Bytes::mp4_box`] wraps one in its
//! header. Every builder writes version 0 with no flags, the only version
//! the specification defines for these boxes. The writers share no code with
//! the parser they feed.

use crate::bytes::Bytes;

/// The full-box header every body starts with: version 0 and no flags.
fn full_box() -> Bytes {
    let mut body = Bytes::new();
    body.u8(0).u24_be(0);
    body
}

/// The 32-bit entry count of `entries`.
///
/// # Panics
///
/// Panics when there are more than `u32::MAX` entries.
fn count<T>(entries: &[T]) -> u32 {
    u32::try_from(entries.len()).expect("a sample table counts its entries in 32 bits")
}

/// A `TimeToSampleBox` body: each run is a sample count and the decoding
/// time delta every sample in the run has, in the track's timescale.
#[must_use]
pub fn stts(runs: &[(u32, u32)]) -> Vec<u8> {
    let mut body = full_box();
    body.u32_be(count(runs));
    for &(samples, delta) in runs {
        body.u32_be(samples).u32_be(delta);
    }
    body.into_vec()
}

/// A `SampleToChunkBox` body: each run is the first chunk it applies to
/// (counted from 1), the samples in each of its chunks and their sample
/// description index.
#[must_use]
pub fn stsc(runs: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut body = full_box();
    body.u32_be(count(runs));
    for &(first_chunk, samples_per_chunk, description) in runs {
        body.u32_be(first_chunk)
            .u32_be(samples_per_chunk)
            .u32_be(description);
    }
    body.into_vec()
}

/// A `SampleSizeBox` body that lists the size of every sample: a sample
/// size of zero, the sample count, then one 32-bit size per sample.
#[must_use]
pub fn stsz(sizes: &[u32]) -> Vec<u8> {
    let mut body = full_box();
    body.u32_be(0).u32_be(count(sizes));
    for &size in sizes {
        body.u32_be(size);
    }
    body.into_vec()
}

/// A `SampleSizeBox` body for `samples` samples that all have `size`
/// octets, which lists no sizes.
///
/// # Panics
///
/// Panics when `size` is zero, which means the sizes are listed.
#[must_use]
pub fn stsz_constant(size: u32, samples: u32) -> Vec<u8> {
    assert!(
        size != 0,
        "a sample size of zero means the sizes are listed"
    );
    let mut body = full_box();
    body.u32_be(size).u32_be(samples);
    body.into_vec()
}

/// A `CompactSampleSizeBox` body: 24 reserved bits, the field size, the
/// sample count, then every size in a field of `bits` bits. Fields of 4
/// bits are packed two to an octet, the first in the high half, and the
/// last octet is padded with zero bits.
///
/// # Panics
///
/// Panics when `bits` is not 4, 8 or 16, or a size does not fit in it.
#[must_use]
pub fn stz2(bits: u8, sizes: &[u16]) -> Vec<u8> {
    assert!(
        matches!(bits, 4 | 8 | 16),
        "a compact sample size field is 4, 8 or 16 bits, not {bits}"
    );
    for &size in sizes {
        assert!(
            u32::from(size) < 1 << bits,
            "{size} does not fit in {bits} bits"
        );
    }
    let mut body = full_box();
    body.u24_be(0).u8(bits).u32_be(count(sizes));
    match bits {
        4 => {
            for pair in sizes.chunks(2) {
                let high = pair.first().copied().unwrap_or_default();
                let low = pair.get(1).copied().unwrap_or_default();
                body.u8(u8::try_from((high << 4) + low).expect("two nibbles fit an octet"));
            }
        }
        8 => {
            for &size in sizes {
                body.u8(u8::try_from(size).expect("the size was checked to fit"));
            }
        }
        _ => {
            for &size in sizes {
                body.u16_be(size);
            }
        }
    }
    body.into_vec()
}

/// A `ChunkOffsetBox` body: the file offset of every chunk in 32 bits.
#[must_use]
pub fn stco(offsets: &[u32]) -> Vec<u8> {
    let mut body = full_box();
    body.u32_be(count(offsets));
    for &offset in offsets {
        body.u32_be(offset);
    }
    body.into_vec()
}

/// A `ChunkLargeOffsetBox` body: the file offset of every chunk in 64 bits.
#[must_use]
pub fn co64(offsets: &[u64]) -> Vec<u8> {
    let mut body = full_box();
    body.u32_be(count(offsets));
    for &offset in offsets {
        body.u64_be(offset);
    }
    body.into_vec()
}

/// The `stts` runs for samples whose decoding time deltas are `deltas`, in
/// order: each run holds consecutive samples with the same delta.
///
/// # Panics
///
/// Panics when one run would hold more than `u32::MAX` samples.
#[must_use]
pub fn time_runs(deltas: &[u32]) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for &delta in deltas {
        match runs.last_mut() {
            Some((samples, last)) if *last == delta => {
                *samples = samples.checked_add(1).expect("a run fits 32 bits");
            }
            _ => runs.push((1, delta)),
        }
    }
    runs
}

/// The `stsc` runs for chunks holding `samples_per_chunk` samples each, in
/// order, all with sample description 1: a run starts at the first chunk
/// and wherever the count changes.
///
/// # Panics
///
/// Panics when there are more than `u32::MAX` chunks.
#[must_use]
pub fn chunk_runs(samples_per_chunk: &[u32]) -> Vec<(u32, u32, u32)> {
    let mut runs: Vec<(u32, u32, u32)> = Vec::new();
    for (index, &samples) in samples_per_chunk.iter().enumerate() {
        if runs.last().is_none_or(|&(_, last, _)| last != samples) {
            let chunk = u32::try_from(index + 1).expect("chunks are numbered in 32 bits");
            runs.push((chunk, samples, 1));
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Version 0 and no flags, as every body starts.
    const FULL_BOX: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

    #[test]
    fn writes_time_to_sample_runs_as_count_then_delta() {
        assert_eq!(
            stts(&[(3, 1024), (1, 0x0102_0304)]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x02], // entry count
                &[0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x04, 0x00],
                &[0x00, 0x00, 0x00, 0x01, 0x01, 0x02, 0x03, 0x04],
            ]
            .concat()
        );
    }

    #[test]
    fn writes_an_empty_table_as_its_header_and_a_zero_count() {
        let empty = [&FULL_BOX[..], &[0x00, 0x00, 0x00, 0x00]].concat();
        assert_eq!(stts(&[]), empty);
        assert_eq!(stsc(&[]), empty);
        assert_eq!(stco(&[]), empty);
        assert_eq!(co64(&[]), empty);
        assert_eq!(stsz(&[]), [&empty[..], &[0x00, 0x00, 0x00, 0x00]].concat());
    }

    #[test]
    fn writes_sample_to_chunk_runs_as_first_chunk_samples_and_description() {
        assert_eq!(
            stsc(&[(1, 4, 1), (3, 0x0A0B_0C0D, 2)]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x02], // entry count
                &[
                    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01
                ],
                &[
                    0x00, 0x00, 0x00, 0x03, 0x0A, 0x0B, 0x0C, 0x0D, 0x00, 0x00, 0x00, 0x02
                ],
            ]
            .concat()
        );
    }

    #[test]
    fn writes_listed_sample_sizes_after_a_size_of_zero() {
        assert_eq!(
            stsz(&[0x0102, 7, 0x0A0B_0C0D]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x00], // sample size: listed
                &[0x00, 0x00, 0x00, 0x03], // sample count
                &[0x00, 0x00, 0x01, 0x02],
                &[0x00, 0x00, 0x00, 0x07],
                &[0x0A, 0x0B, 0x0C, 0x0D],
            ]
            .concat()
        );
    }

    #[test]
    fn writes_a_constant_sample_size_with_no_list() {
        assert_eq!(
            stsz_constant(0x0102_0304, 0x0506_0708),
            [
                &FULL_BOX[..],
                &[0x01, 0x02, 0x03, 0x04], // sample size
                &[0x05, 0x06, 0x07, 0x08], // sample count
            ]
            .concat()
        );
    }

    #[test]
    #[should_panic(expected = "a sample size of zero means the sizes are listed")]
    fn refuses_a_constant_sample_size_of_zero() {
        let _ = stsz_constant(0, 1);
    }

    #[test]
    fn packs_four_bit_sizes_two_to_an_octet_high_half_first() {
        assert_eq!(
            stz2(4, &[0x1, 0xF, 0x3]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x04], // reserved, field size
                &[0x00, 0x00, 0x00, 0x03], // sample count
                &[0x1F, 0x30],             // the last octet padded
            ]
            .concat()
        );
        assert_eq!(
            stz2(4, &[0xA, 0xB]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0xAB],
            ]
            .concat()
        );
    }

    #[test]
    fn writes_eight_and_sixteen_bit_sizes_one_to_a_field() {
        assert_eq!(
            stz2(8, &[0x01, 0xFF]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x02, 0x01, 0xFF],
            ]
            .concat()
        );
        assert_eq!(
            stz2(16, &[0x0102, 0xFFFF]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x02],
                &[0x01, 0x02, 0xFF, 0xFF],
            ]
            .concat()
        );
    }

    #[test]
    #[should_panic(expected = "a compact sample size field is 4, 8 or 16 bits, not 12")]
    fn refuses_a_field_size_the_box_does_not_define() {
        let _ = stz2(12, &[]);
    }

    #[test]
    #[should_panic(expected = "16 does not fit in 4 bits")]
    fn refuses_a_size_too_wide_for_four_bits() {
        let _ = stz2(4, &[15, 16]);
    }

    #[test]
    #[should_panic(expected = "256 does not fit in 8 bits")]
    fn refuses_a_size_too_wide_for_eight_bits() {
        let _ = stz2(8, &[255, 256]);
    }

    #[test]
    fn writes_32_and_64_bit_chunk_offsets() {
        assert_eq!(
            stco(&[0x0102_0304, 8]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x02], // entry count
                &[0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x08],
            ]
            .concat()
        );
        assert_eq!(
            co64(&[0x0102_0304_0506_0708]),
            [
                &FULL_BOX[..],
                &[0x00, 0x00, 0x00, 0x01], // entry count
                &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            ]
            .concat()
        );
    }

    #[test]
    fn writes_a_body_that_a_box_header_wraps() {
        let mut written = Bytes::new();
        written.mp4_box(*b"stco", &stco(&[48]));
        assert_eq!(
            written.into_vec(),
            [
                &[0x00, 0x00, 0x00, 0x14][..],
                b"stco",
                &FULL_BOX,
                &[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x30],
            ]
            .concat()
        );
    }

    #[test]
    fn groups_equal_neighbouring_deltas_into_one_run() {
        assert_eq!(time_runs(&[]), []);
        assert_eq!(
            time_runs(&[1024, 1024, 1024, 512, 1024, 0, 0]),
            [(3, 1024), (1, 512), (1, 1024), (2, 0)]
        );
    }

    #[test]
    fn starts_a_chunk_run_at_chunk_one_and_wherever_the_count_changes() {
        assert_eq!(chunk_runs(&[]), []);
        assert_eq!(
            chunk_runs(&[4, 4, 4, 2, 2, 4, 0]),
            [(1, 4, 1), (4, 2, 1), (6, 4, 1), (7, 0, 1)]
        );
    }

    /// Expands `stts` runs back into one delta per sample.
    fn expand_time(runs: &[(u32, u32)]) -> Vec<u32> {
        runs.iter()
            .flat_map(|&(samples, delta)| (0..samples).map(move |_| delta))
            .collect()
    }

    /// Expands `stsc` runs back into one count per chunk, for `chunks`
    /// chunks.
    fn expand_chunks(runs: &[(u32, u32, u32)], chunks: usize) -> Vec<u32> {
        (1..=chunks)
            .map(|chunk| {
                runs.iter()
                    .rev()
                    .find(|&&(first, _, _)| first as usize <= chunk)
                    .map_or(u32::MAX, |&(_, samples, _)| samples)
            })
            .collect()
    }

    proptest! {
        #[test]
        fn time_runs_expand_back_to_the_deltas(deltas in vec(0_u32..3, 0..40)) {
            let runs = time_runs(&deltas);
            prop_assert_eq!(expand_time(&runs), deltas);
            // Neighbouring runs never share a delta, and no run is empty.
            prop_assert!(runs.windows(2).all(|pair| pair[0].1 != pair[1].1));
            prop_assert!(runs.iter().all(|&(samples, _)| samples > 0));
        }

        #[test]
        fn chunk_runs_expand_back_to_the_counts(counts in vec(0_u32..3, 0..40)) {
            let runs = chunk_runs(&counts);
            prop_assert_eq!(expand_chunks(&runs, counts.len()), counts);
            prop_assert!(runs.windows(2).all(|pair| pair[0].1 != pair[1].1));
        }
    }
}
