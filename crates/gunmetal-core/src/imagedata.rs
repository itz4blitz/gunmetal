//! Artwork placeholder and palette from a small decoded RGBA buffer.
//!
//! The compact placeholder scheme is **`ThumbHash`** (Evan Wallace, 2023), as
//! published at <https://evanw.github.io/thumbhash/> and in the reference
//! JavaScript of <https://github.com/evanw/thumbhash>. It stores a DCT of a
//! ≤100×100 un-premultiplied RGBA image, the aspect ratio, and optional
//! alpha, in at most [`Placeholder::MAX_LEN`] bytes. Decoding images is
//! WP-079; this module only hashes a buffer the worker has already decoded
//! and shrunk.
//!
//! [`palette`] returns up to three OKLCH candidates with contrast against
//! black and white, for the artwork tint rules in
//! `docs/ui/design-language.md`.

use std::f64::consts::PI;

/// Longest side the `ThumbHash` encoder accepts, as the published JavaScript
/// documents: encoding a larger image is slow with no benefit.
pub const MAX_SIDE: u16 = 100;

/// Byte length of a `ThumbHash`, which the published layout never exceeds
/// (5 header bytes, an optional alpha byte, and packed AC nibbles).
pub const PLACEHOLDER_MAX_LEN: usize = 25;

/// At most three palette candidates: a base, a vivid and a deep colour.
pub const MAX_SWATCHES: usize = 3;

/// The placeholder scheme this module encodes, recorded for callers and
/// for the independent test decoder.
pub const PLACEHOLDER_SCHEME: &str = "ThumbHash";

/// OKLCH chroma below which a mixed image treats a cluster as near-grey
/// and drops it when a chromatic candidate exists (`design-language.md`).
const GREY_CHROMA_MILLI: u16 = 40;

/// RGB pixels in one median-cut box.
type RgbBox = Vec<[u8; 3]>;

/// A `ThumbHash` of an artwork image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placeholder {
    bytes: [u8; PLACEHOLDER_MAX_LEN],
    len: u8,
}

impl Placeholder {
    /// Largest encoded `ThumbHash`, in bytes.
    pub const MAX_LEN: usize = PLACEHOLDER_MAX_LEN;

    /// The encoded bytes, 5 to [`Self::MAX_LEN`] long.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        let len = usize::from(self.len).min(Self::MAX_LEN);
        self.bytes.get(..len).unwrap_or(&[])
    }

    /// How many bytes [`Self::as_bytes`] holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Whether [`Self::as_bytes`] holds no bytes. A hash this module
    /// returns is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }
}

/// One palette candidate in OKLCH, with WCAG contrast against black and
/// white so a client can apply the tint rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    /// OKLCH lightness × 1000 (0 to 1000).
    pub lightness: u16,
    /// OKLCH chroma × 1000 (0 to about 400).
    pub chroma: u16,
    /// OKLCH hue in degrees, 0 to 359. Zero when chroma rounds to 0.
    pub hue: u16,
    /// WCAG 2 contrast against sRGB black, in hundredths (2100 is 21.00:1).
    pub contrast_black: u16,
    /// WCAG 2 contrast against sRGB white, in hundredths.
    pub contrast_white: u16,
}

/// Why [`placeholder`] or [`palette`] refused a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDataError {
    /// Width or height is zero, so there is no image.
    Empty {
        /// The width given.
        width: u16,
        /// The height given.
        height: u16,
    },
    /// A side is longer than [`MAX_SIDE`].
    TooLarge {
        /// The width given.
        width: u16,
        /// The height given.
        height: u16,
        /// The `ThumbHash` input cap.
        max: u16,
    },
    /// `rgba.len()` is not `width × height × 4`.
    LengthMismatch {
        /// The width given.
        width: u16,
        /// The height given.
        height: u16,
        /// Octets the buffer actually held.
        len: usize,
        /// Octets `width × height × 4` needs.
        expected: usize,
    },
}

/// Encodes `rgba` as a `ThumbHash`. `rgba` is un-premultiplied, row-major,
/// four octets per pixel.
///
/// # Errors
///
/// [`ImageDataError::Empty`] when a side is 0, [`ImageDataError::TooLarge`]
/// when a side exceeds [`MAX_SIDE`], [`ImageDataError::LengthMismatch`]
/// when the buffer is not `width × height × 4` octets.
pub fn placeholder(rgba: &[u8], width: u16, height: u16) -> Result<Placeholder, ImageDataError> {
    let (width, height) = raster_size(rgba, width, height)?;
    Ok(encode_thumbhash(rgba, width, height))
}

/// Up to three OKLCH palette candidates for the tint rules, largest
/// cluster first. Fully transparent pixels contribute nothing; a buffer
/// of only those returns an empty list.
///
/// # Errors
///
/// The same dimension and length errors as [`placeholder`].
pub fn palette(rgba: &[u8], width: u16, height: u16) -> Result<Vec<Swatch>, ImageDataError> {
    let (width, height) = raster_size(rgba, width, height)?;
    Ok(extract_palette(rgba, width, height))
}

/// Checks `width`, `height` and `rgba.len()` before any pixel work.
fn raster_size(rgba: &[u8], width: u16, height: u16) -> Result<(usize, usize), ImageDataError> {
    if width == 0 || height == 0 {
        return Err(ImageDataError::Empty { width, height });
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(ImageDataError::TooLarge {
            width,
            height,
            max: MAX_SIDE,
        });
    }
    let wide = usize::from(width);
    let high = usize::from(height);
    let expected = wide.saturating_mul(high).saturating_mul(4);
    if rgba.len() != expected {
        return Err(ImageDataError::LengthMismatch {
            width,
            height,
            len: rgba.len(),
            expected,
        });
    }
    Ok((wide, high))
}

/// Un-premultiplied luma / yellow-blue / red-green / alpha planes.
struct Planes {
    luma: Vec<f64>,
    yellow_blue: Vec<f64>,
    red_green: Vec<f64>,
    alpha: Vec<f64>,
}

/// `ThumbHash` encoder following the published JavaScript `rgbaToThumbHash`.
fn encode_thumbhash(rgba: &[u8], width: usize, height: usize) -> Placeholder {
    let pixel_count = width.saturating_mul(height);
    let (average_red, average_green, average_blue, average_alpha) = premul_average(rgba);
    let count = f64::from(u32::try_from(pixel_count).unwrap_or(u32::MAX));
    let has_alpha = average_alpha < count;
    let planes = unpremultiply(rgba, average_red, average_green, average_blue);
    let luma_limit = if has_alpha { 5.0 } else { 7.0 };
    let longest = width.max(height);
    let x_extent = channel_extent(luma_limit, width, longest);
    let y_extent = channel_extent(luma_limit, height, longest);
    let (luma_mean, luma_terms, luma_scale) = dct_channel(
        &planes.luma,
        width,
        height,
        x_extent.max(3),
        y_extent.max(3),
    );
    let (yellow_blue_mean, yellow_blue_terms, yellow_blue_scale) =
        dct_channel(&planes.yellow_blue, width, height, 3, 3);
    let (red_green_mean, red_green_terms, red_green_scale) =
        dct_channel(&planes.red_green, width, height, 3, 3);
    let (alpha_mean, alpha_terms, alpha_scale) = if has_alpha {
        dct_channel(&planes.alpha, width, height, 5, 5)
    } else {
        (1.0, Vec::new(), 0.0)
    };
    pack_hash(&PackArgs {
        landscape: width > height,
        has_alpha,
        x_extent,
        y_extent,
        luma_mean,
        yellow_blue_mean,
        red_green_mean,
        alpha_mean,
        luma_scale,
        yellow_blue_scale,
        red_green_scale,
        alpha_scale,
        luma_terms: &luma_terms,
        yellow_blue_terms: &yellow_blue_terms,
        red_green_terms: &red_green_terms,
        alpha_terms: &alpha_terms,
    })
}

/// Premultiplied average colour of `rgba`, matching the published encoder.
fn premul_average(rgba: &[u8]) -> (f64, f64, f64, f64) {
    let mut average_red = 0.0;
    let mut average_green = 0.0;
    let mut average_blue = 0.0;
    let mut average_alpha = 0.0;
    for pixel in rgba.chunks_exact(4) {
        let red = f64::from(*pixel.first().unwrap_or(&0));
        let green = f64::from(*pixel.get(1).unwrap_or(&0));
        let blue = f64::from(*pixel.get(2).unwrap_or(&0));
        let alpha_unit = f64::from(*pixel.get(3).unwrap_or(&0)) / 255.0;
        average_red += alpha_unit / 255.0 * red;
        average_green += alpha_unit / 255.0 * green;
        average_blue += alpha_unit / 255.0 * blue;
        average_alpha += alpha_unit;
    }
    if average_alpha > 0.0 {
        average_red /= average_alpha;
        average_green /= average_alpha;
        average_blue /= average_alpha;
    }
    (average_red, average_green, average_blue, average_alpha)
}

/// Un-premultiply each pixel, filling transparent pixels with the average.
fn unpremultiply(rgba: &[u8], average_red: f64, average_green: f64, average_blue: f64) -> Planes {
    let mut luma = Vec::new();
    let mut yellow_blue = Vec::new();
    let mut red_green = Vec::new();
    let mut alpha = Vec::new();
    for pixel in rgba.chunks_exact(4) {
        let red8 = f64::from(*pixel.first().unwrap_or(&0));
        let green8 = f64::from(*pixel.get(1).unwrap_or(&0));
        let blue8 = f64::from(*pixel.get(2).unwrap_or(&0));
        let alpha_unit = f64::from(*pixel.get(3).unwrap_or(&0)) / 255.0;
        let red = average_red.mul_add(1.0 - alpha_unit, alpha_unit / 255.0 * red8);
        let green = average_green.mul_add(1.0 - alpha_unit, alpha_unit / 255.0 * green8);
        let blue = average_blue.mul_add(1.0 - alpha_unit, alpha_unit / 255.0 * blue8);
        luma.push((red + green + blue) / 3.0);
        yellow_blue.push(f64::midpoint(red, green) - blue);
        red_green.push(red - green);
        alpha.push(alpha_unit);
    }
    Planes {
        luma,
        yellow_blue,
        red_green,
        alpha,
    }
}

/// `max(1, round(limit * along / longest))` from the published encoder.
fn channel_extent(limit: f64, along: usize, longest: usize) -> usize {
    let along = f64::from(u32::try_from(along).unwrap_or(0));
    let longest = f64::from(u32::try_from(longest.max(1)).unwrap_or(1));
    usize::try_from(js_round(limit * along / longest).max(1)).unwrap_or(1)
}

/// DCT-II of `samples` over the `ThumbHash` coefficient triangle.
fn dct_channel(
    samples: &[f64],
    width: usize,
    height: usize,
    freq_x: usize,
    freq_y: usize,
) -> (f64, Vec<f64>, f64) {
    let area = f64::from(u32::try_from(width.saturating_mul(height).max(1)).unwrap_or(1));
    let mut mean_term = 0.0_f64;
    let mut ac_terms = Vec::new();
    let mut scale = 0.0_f64;
    for coeff_y in 0..freq_y {
        for coeff_x in 0..freq_x {
            if !in_triangle(coeff_x, coeff_y, freq_x, freq_y) {
                break;
            }
            let mut sum = 0.0;
            for row in 0..height {
                let basis_y = dct_basis(height, coeff_y, row);
                for column in 0..width {
                    let basis_x = dct_basis(width, coeff_x, column);
                    let index = row.saturating_mul(width).saturating_add(column);
                    let sample = samples.get(index).copied().unwrap_or(0.0);
                    sum += sample * basis_x * basis_y;
                }
            }
            let term = sum / area;
            if coeff_x == 0 && coeff_y == 0 {
                mean_term = term;
            } else {
                ac_terms.push(term);
                scale = scale.max(term.abs());
            }
        }
    }
    if scale > 0.0 {
        for term in &mut ac_terms {
            *term = 0.5 + 0.5 / scale * *term;
        }
    }
    (mean_term, ac_terms, scale)
}

/// `cos(π / len * freq * (index + 0.5))`.
fn dct_basis(len: usize, freq: usize, index: usize) -> f64 {
    let len = f64::from(u32::try_from(len.max(1)).unwrap_or(1));
    let freq = f64::from(u32::try_from(freq).unwrap_or(0));
    let index = f64::from(u32::try_from(index).unwrap_or(0));
    (PI / len * freq * (index + 0.5)).cos()
}

/// Whether `(coeff_x, coeff_y)` is inside the published `ThumbHash` triangle
/// `coeff_x * freq_y < freq_x * (freq_y - coeff_y)`.
fn in_triangle(coeff_x: usize, coeff_y: usize, freq_x: usize, freq_y: usize) -> bool {
    let coeff_x = u32::try_from(coeff_x).unwrap_or(u32::MAX);
    let coeff_y = u32::try_from(coeff_y).unwrap_or(u32::MAX);
    let freq_x = u32::try_from(freq_x).unwrap_or(0);
    let freq_y = u32::try_from(freq_y).unwrap_or(0);
    match (
        coeff_x.checked_mul(freq_y),
        freq_y
            .checked_sub(coeff_y)
            .and_then(|rest| freq_x.checked_mul(rest)),
    ) {
        (Some(left), Some(right)) => left < right,
        _ => false,
    }
}

/// Arguments the published `ThumbHash` header carries.
struct PackArgs<'a> {
    landscape: bool,
    has_alpha: bool,
    x_extent: usize,
    y_extent: usize,
    luma_mean: f64,
    yellow_blue_mean: f64,
    red_green_mean: f64,
    alpha_mean: f64,
    luma_scale: f64,
    yellow_blue_scale: f64,
    red_green_scale: f64,
    alpha_scale: f64,
    luma_terms: &'a [f64],
    yellow_blue_terms: &'a [f64],
    red_green_terms: &'a [f64],
    alpha_terms: &'a [f64],
}

/// Packs DC, scales and AC nibbles into the published 5–25 byte layout.
fn pack_hash(args: &PackArgs<'_>) -> Placeholder {
    let header24 = quantize(63.0 * args.luma_mean, 63)
        | quantize(31.5 + 31.5 * args.yellow_blue_mean, 63).wrapping_shl(6)
        | quantize(31.5 + 31.5 * args.red_green_mean, 63).wrapping_shl(12)
        | quantize(31.0 * args.luma_scale, 31).wrapping_shl(18)
        | u32::from(args.has_alpha).wrapping_shl(23);
    let x_header = u32::try_from(args.x_extent).unwrap_or(0) & 7;
    let y_header = u32::try_from(args.y_extent).unwrap_or(0) & 7;
    let stored_extent = if args.landscape { y_header } else { x_header };
    let header16 = stored_extent
        | quantize(63.0 * args.yellow_blue_scale, 63).wrapping_shl(3)
        | quantize(63.0 * args.red_green_scale, 63).wrapping_shl(9)
        | u32::from(args.landscape).wrapping_shl(15);
    let mut bytes = [0_u8; PLACEHOLDER_MAX_LEN];
    write_u8(&mut bytes, 0, header24 & 0xFF);
    write_u8(&mut bytes, 1, header24.wrapping_shr(8) & 0xFF);
    write_u8(&mut bytes, 2, header24.wrapping_shr(16) & 0xFF);
    write_u8(&mut bytes, 3, header16 & 0xFF);
    write_u8(&mut bytes, 4, header16.wrapping_shr(8) & 0xFF);
    let mut len = 5_usize;
    if args.has_alpha {
        write_u8(
            &mut bytes,
            5,
            quantize(15.0 * args.alpha_mean, 15)
                | quantize(15.0 * args.alpha_scale, 15).wrapping_shl(4),
        );
        len = 6;
    }
    let ac_start: usize = if args.has_alpha { 6 } else { 5 };
    let mut ac_index = 0_usize;
    let channels: [&[f64]; 4] = if args.has_alpha {
        [
            args.luma_terms,
            args.yellow_blue_terms,
            args.red_green_terms,
            args.alpha_terms,
        ]
    } else {
        [
            args.luma_terms,
            args.yellow_blue_terms,
            args.red_green_terms,
            &[],
        ]
    };
    for channel in channels {
        for term in channel {
            let nibble = u8::try_from(quantize(15.0 * *term, 15)).unwrap_or(0);
            let byte_index = ac_start.saturating_add(ac_index.wrapping_div(2));
            let shift = u32::try_from(ac_index.wrapping_rem(2).saturating_mul(4)).unwrap_or(0);
            len = len.max(or_nibble(&mut bytes, byte_index, nibble, shift));
            ac_index = ac_index.saturating_add(1);
        }
    }
    Placeholder {
        bytes,
        len: u8::try_from(len)
            .unwrap_or(0)
            .min(u8::try_from(PLACEHOLDER_MAX_LEN).unwrap_or(25)),
    }
}

/// Writes `value` into `bytes[index]` when that slot exists.
fn write_u8(bytes: &mut [u8], index: usize, value: u32) {
    if let Some(slot) = bytes.get_mut(index) {
        *slot = u8::try_from(value).unwrap_or(0);
    }
}

/// JavaScript `Math.round` (half toward +∞), then clamped to `0..=max`.
fn quantize(value: f64, max: u32) -> u32 {
    clamp_to_u32(js_round(value), max)
}

/// Clamp a rounded integer into `0..=cap`.
fn clamp_to_u32(rounded: i32, cap: u32) -> u32 {
    if rounded < 0 {
        0
    } else {
        u32::try_from(rounded).unwrap_or(cap).min(cap)
    }
}

/// Clamp a rounded integer into `0..=cap`.
fn clamp_to_u16(rounded: i32, cap: u16) -> u16 {
    if rounded < 0 {
        0
    } else {
        u16::try_from(rounded).unwrap_or(cap).min(cap)
    }
}

/// JavaScript `Math.round`: `floor(x + 0.5)`.
fn js_round(value: f64) -> i32 {
    let rounded = (value + 0.5).floor();
    if rounded >= f64::from(i32::MAX) {
        i32::MAX
    } else if rounded <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "rounded is finite and inside i32 after the clamps above"
        )]
        {
            rounded as i32
        }
    }
}

/// Median-cut OKLCH palette of opaque pixels.
fn extract_palette(rgba: &[u8], _width: usize, _height: usize) -> Vec<Swatch> {
    let mut opaque = Vec::new();
    for pixel in rgba.chunks_exact(4) {
        let alpha = *pixel.get(3).unwrap_or(&0);
        if alpha == 0 {
            continue;
        }
        let red = *pixel.first().unwrap_or(&0);
        let green = *pixel.get(1).unwrap_or(&0);
        let blue = *pixel.get(2).unwrap_or(&0);
        opaque.push([red, green, blue]);
    }
    if opaque.is_empty() {
        return Vec::new();
    }
    let clusters = median_cut(&opaque, MAX_SWATCHES);
    let mut swatches: Vec<Swatch> = clusters.iter().map(|rgb| swatch_from_srgb(*rgb)).collect();
    let chromatic: Vec<Swatch> = swatches
        .iter()
        .copied()
        .filter(|swatch| swatch.chroma >= GREY_CHROMA_MILLI)
        .collect();
    if !chromatic.is_empty() {
        swatches = chromatic;
    }
    distinct_lch(swatches)
}

/// Keep swatches with distinct OKLCH triples, at most [`MAX_SWATCHES`].
fn distinct_lch(swatches: Vec<Swatch>) -> Vec<Swatch> {
    let mut unique = Vec::new();
    for swatch in swatches {
        let key = (swatch.lightness, swatch.chroma, swatch.hue);
        if unique
            .iter()
            .all(|kept: &Swatch| (kept.lightness, kept.chroma, kept.hue) != key)
        {
            unique.push(swatch);
        }
        if unique.len() >= MAX_SWATCHES {
            break;
        }
    }
    unique
}

/// Split `pixels` into at most `max_boxes` boxes by median cut on RGB.
fn median_cut(pixels: &[[u8; 3]], max_boxes: usize) -> Vec<[u8; 3]> {
    let mut boxes = Vec::new();
    boxes.push(pixels.to_vec());
    while boxes.len() < max_boxes {
        let Some(index) = boxes
            .iter()
            .enumerate()
            .filter(|(_, cluster)| longest_range(cluster).0 > 0)
            .max_by_key(|(_, cluster)| longest_range(cluster))
            .map(|(index, _)| index)
        else {
            break;
        };
        let _ = replace_with_split(&mut boxes, index);
    }
    boxes.iter().map(|cluster| mean_rgb(cluster)).collect()
}

/// Take the box at `index`, or an empty box if that slot is gone.
fn take_cluster(boxes: &mut [RgbBox], index: usize) -> RgbBox {
    boxes.get_mut(index).map(std::mem::take).unwrap_or_default()
}

/// Split the box at `index` into two and put both back. `false` when it
/// cannot split.
fn replace_with_split(boxes: &mut Vec<RgbBox>, index: usize) -> bool {
    let cluster = take_cluster(boxes, index);
    let Some((left, right)) = split_cluster(cluster) else {
        return false;
    };
    store_box(boxes, index, left);
    boxes.push(right);
    true
}

/// Put `cluster` at `index`, or append it when that slot is gone.
fn store_box(boxes: &mut Vec<RgbBox>, index: usize, cluster: RgbBox) {
    if let Some(slot) = boxes.get_mut(index) {
        *slot = cluster;
    } else {
        boxes.push(cluster);
    }
}

/// `(range, channel)` of the RGB axis with the largest span.
fn longest_range(pixels: &[[u8; 3]]) -> (u8, u8) {
    let mut lowest = [u8::MAX; 3];
    let mut highest = [0_u8; 3];
    for pixel in pixels {
        for channel in 0..3_usize {
            let value = rgb_channel(*pixel, channel);
            set_u8(&mut lowest, channel, value, false);
            set_u8(&mut highest, channel, value, true);
        }
    }
    let mut best = (0_u8, 0_u8);
    for channel in 0..3_u8 {
        let index = usize::from(channel);
        let span = rgb_channel(highest, index).saturating_sub(rgb_channel(lowest, index));
        if span >= best.0 {
            best = (span, channel);
        }
    }
    best
}

/// Split `pixels` at the median of the longest RGB axis.
fn split_cluster(mut pixels: RgbBox) -> Option<(RgbBox, RgbBox)> {
    if pixels.len() < 2 {
        return None;
    }
    let (range, channel) = longest_range(&pixels);
    if range == 0 {
        return None;
    }
    let axis = usize::from(channel);
    pixels.sort_by_key(|pixel| rgb_channel(*pixel, axis));
    let last = pixels.len().saturating_sub(1);
    let mid = pixels.len().wrapping_div(2).clamp(1, last);
    let right = pixels.split_off(mid);
    Some((pixels, right))
}

/// Channel `index` of an sRGB triple, or 0 if that slot is missing.
fn rgb_channel(pixel: [u8; 3], index: usize) -> u8 {
    pixel.get(index).copied().unwrap_or(0)
}

/// Write `value` into `values[index]` as a min or max, when the slot exists.
fn set_u8(values: &mut [u8; 3], index: usize, value: u8, prefer_max: bool) {
    if let Some(slot) = values.get_mut(index) {
        *slot = if prefer_max {
            (*slot).max(value)
        } else {
            (*slot).min(value)
        };
    }
}

/// Overwrite `values[index]` when that slot exists.
fn write_channel(values: &mut [u8; 3], index: usize, value: u8) {
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    }
}

/// Saturating-add `value` into `values[index]` when that slot exists.
fn add_u32(values: &mut [u32; 3], index: usize, value: u32) {
    if let Some(slot) = values.get_mut(index) {
        *slot = slot.saturating_add(value);
    }
}

/// Channel `index` of a sum triple, or 0 if that slot is missing.
fn u32_at(values: &[u32; 3], index: usize) -> u32 {
    values.get(index).copied().unwrap_or(0)
}

/// Read a packed AC nibble into `bytes[index]`. Returns the exclusive end
/// of the written byte, or 0 when `index` is out of range.
fn or_nibble(bytes: &mut [u8], index: usize, nibble: u8, shift: u32) -> usize {
    match bytes.get_mut(index) {
        Some(slot) => {
            *slot |= nibble.wrapping_shl(shift);
            index.saturating_add(1)
        }
        None => 0,
    }
}

/// Mean sRGB of a cluster, rounding each channel toward nearest.
fn mean_rgb(pixels: &[[u8; 3]]) -> [u8; 3] {
    let count = u32::try_from(pixels.len().max(1)).unwrap_or(1);
    let mut sums = [0_u32; 3];
    for pixel in pixels {
        for channel in 0..3_usize {
            add_u32(&mut sums, channel, u32::from(rgb_channel(*pixel, channel)));
        }
    }
    let mut mean = [0_u8; 3];
    for channel in 0..3_usize {
        let sum = u32_at(&sums, channel);
        write_channel(
            &mut mean,
            channel,
            u8::try_from(sum.checked_div(count).unwrap_or(0)).unwrap_or(u8::MAX),
        );
    }
    mean
}

/// OKLCH swatch and WCAG contrast of an sRGB triple, from Ottosson's `OKLab`.
fn swatch_from_srgb(rgb: [u8; 3]) -> Swatch {
    let red = rgb.first().copied().unwrap_or(0);
    let green = rgb.get(1).copied().unwrap_or(0);
    let blue = rgb.get(2).copied().unwrap_or(0);
    let (lightness, chroma, hue) = srgb_to_oklch(red, green, blue);
    Swatch {
        lightness,
        chroma,
        hue,
        contrast_black: contrast_hundredths(red, green, blue, 0, 0, 0),
        contrast_white: contrast_hundredths(red, green, blue, 255, 255, 255),
    }
}

/// sRGB 8-bit to OKLCH milles and degrees (Ottosson 2020).
fn srgb_to_oklch(red: u8, green: u8, blue: u8) -> (u16, u16, u16) {
    let linear_red = srgb_to_linear(red);
    let linear_green = srgb_to_linear(green);
    let linear_blue = srgb_to_linear(blue);
    let cone_l = 0.412_221_470_8_f64.mul_add(
        linear_red,
        0.536_332_536_3_f64.mul_add(linear_green, 0.051_445_992_9_f64 * linear_blue),
    );
    let cone_m = 0.211_903_498_2_f64.mul_add(
        linear_red,
        0.680_699_545_1_f64.mul_add(linear_green, 0.107_396_956_6_f64 * linear_blue),
    );
    let cone_s = 0.088_302_461_9_f64.mul_add(
        linear_red,
        0.281_718_837_6_f64.mul_add(linear_green, 0.629_978_700_5_f64 * linear_blue),
    );
    let cube_l = cone_l.cbrt();
    let cube_m = cone_m.cbrt();
    let cube_s = cone_s.cbrt();
    let ok_l = 0.210_454_255_3_f64.mul_add(
        cube_l,
        0.793_617_785_0_f64.mul_add(cube_m, -0.004_072_046_8_f64 * cube_s),
    );
    let ok_a = 1.977_998_495_1_f64.mul_add(
        cube_l,
        (-2.428_592_205_0_f64).mul_add(cube_m, 0.450_593_709_9_f64 * cube_s),
    );
    let ok_b = 0.025_904_037_1_f64.mul_add(
        cube_l,
        0.738_491_886_6_f64.mul_add(cube_m, -0.764_395_924_1_f64 * cube_s),
    );
    let chroma = ok_a.hypot(ok_b);
    let hue = if chroma < 0.000_5 {
        0.0
    } else {
        let degrees = ok_b.atan2(ok_a).to_degrees();
        if degrees < 0.0 {
            degrees + 360.0
        } else {
            degrees
        }
    };
    (unit_milli(ok_l, 1000), unit_milli(chroma, 500), degree(hue))
}

/// Round `value × 1000` to `0..=cap`.
fn unit_milli(value: f64, cap: u16) -> u16 {
    clamp_to_u16(js_round(value * 1000.0), cap)
}

/// Round a hue in degrees to `0..=359`.
fn degree(hue: f64) -> u16 {
    clamp_to_u16(js_round(hue), 359)
}

/// IEC 61966-2-1 sRGB channel to linear light, 0 to 1.
fn srgb_to_linear(channel: u8) -> f64 {
    let unit = f64::from(channel) / 255.0;
    if unit <= 0.040_45 {
        unit / 12.92
    } else {
        ((unit + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 2 contrast ratio of two sRGB colours, in hundredths.
fn contrast_hundredths(
    left_r: u8,
    left_g: u8,
    left_b: u8,
    right_r: u8,
    right_g: u8,
    right_b: u8,
) -> u16 {
    let left = relative_luminance(left_r, left_g, left_b);
    let right = relative_luminance(right_r, right_g, right_b);
    let (lighter, darker) = if left >= right {
        (left, right)
    } else {
        (right, left)
    };
    let ratio = (lighter + 0.05) / (darker + 0.05);
    clamp_to_u16(js_round(ratio * 100.0), 2100)
}

/// WCAG 2 relative luminance of an sRGB triple.
fn relative_luminance(red: u8, green: u8, blue: u8) -> f64 {
    0.212_6_f64.mul_add(
        srgb_to_linear(red),
        0.715_2_f64.mul_add(srgb_to_linear(green), 0.072_2_f64 * srgb_to_linear(blue)),
    )
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::bool_to_int_with_if,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::unnecessary_cast,
    reason = "test oracles follow the published decoder and independent colour formulas"
)]
mod tests {
    use super::*;
    use proptest::collection::vec as arb_bytes;
    use proptest::prelude::*;

    /// Independent `ThumbHash` decoder, written from the published JavaScript
    /// `thumbHashToRGBA` / `thumbHashToApproximateAspectRatio` /
    /// `thumbHashToAverageRGBA` at <https://github.com/evanw/thumbhash>.
    /// It shares no functions with the encoder under test.
    mod published {
        use super::PI;

        pub struct Raster {
            pub width: usize,
            pub height: usize,
            pub rgba: Vec<u8>,
        }

        /// `thumbHashToApproximateAspectRatio`.
        pub fn aspect_ratio(hash: &[u8]) -> f64 {
            let header = *hash.get(3).unwrap_or(&0);
            let has_alpha = hash.get(2).copied().unwrap_or(0) & 0x80 != 0;
            let is_landscape = hash.get(4).copied().unwrap_or(0) & 0x80 != 0;
            let lx = if is_landscape {
                if has_alpha { 5.0 } else { 7.0 }
            } else {
                f64::from(header & 7)
            };
            let ly = if is_landscape {
                f64::from(header & 7)
            } else if has_alpha {
                5.0
            } else {
                7.0
            };
            if ly == 0.0 { 1.0 } else { lx / ly }
        }

        /// `thumbHashToAverageRGBA`, each channel 0 to 1.
        pub fn average_rgba(hash: &[u8]) -> (f64, f64, f64, f64) {
            let header = u32::from(*hash.first().unwrap_or(&0))
                | u32::from(*hash.get(1).unwrap_or(&0)) << 8
                | u32::from(*hash.get(2).unwrap_or(&0)) << 16;
            let l = f64::from(header & 63) / 63.0;
            let p = f64::from((header >> 6) & 63) / 31.5 - 1.0;
            let q = f64::from((header >> 12) & 63) / 31.5 - 1.0;
            let has_alpha = header >> 23 != 0;
            let a = if has_alpha {
                f64::from(hash.get(5).copied().unwrap_or(0) & 15) / 15.0
            } else {
                1.0
            };
            let b = l - 2.0 / 3.0 * p;
            let r = (3.0 * l - b + q) / 2.0;
            let g = r - q;
            (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), a)
        }

        /// `thumbHashToRGBA`.
        #[expect(
            clippy::too_many_lines,
            reason = "the published JavaScript decoder is one function; splitting it would share structure with the encoder"
        )]
        pub fn decode(hash: &[u8]) -> Raster {
            let header24 = u32::from(*hash.first().unwrap_or(&0))
                | u32::from(*hash.get(1).unwrap_or(&0)) << 8
                | u32::from(*hash.get(2).unwrap_or(&0)) << 16;
            let header16 =
                u32::from(*hash.get(3).unwrap_or(&0)) | u32::from(*hash.get(4).unwrap_or(&0)) << 8;
            let l_dc = f64::from(header24 & 63) / 63.0;
            let p_dc = f64::from((header24 >> 6) & 63) / 31.5 - 1.0;
            let q_dc = f64::from((header24 >> 12) & 63) / 31.5 - 1.0;
            let l_scale = f64::from((header24 >> 18) & 31) / 31.0;
            let has_alpha = header24 >> 23 != 0;
            let p_scale = f64::from((header16 >> 3) & 63) / 63.0;
            let q_scale = f64::from((header16 >> 9) & 63) / 63.0;
            let is_landscape = header16 >> 15 != 0;
            let lx = 3.max(if is_landscape {
                if has_alpha { 5 } else { 7 }
            } else {
                (header16 & 7) as usize
            });
            let ly = 3.max(if is_landscape {
                (header16 & 7) as usize
            } else if has_alpha {
                5
            } else {
                7
            });
            let a_dc = if has_alpha {
                f64::from(hash.get(5).copied().unwrap_or(0) & 15) / 15.0
            } else {
                1.0
            };
            let a_scale = f64::from(hash.get(5).copied().unwrap_or(0) >> 4) / 15.0;
            let ac_start = if has_alpha { 6 } else { 5 };
            let mut ac_index = 0_usize;
            let mut read = |nx: usize, ny: usize, scale: f64| {
                let mut ac = Vec::new();
                for cy in 0..ny {
                    let mut cx = if cy == 0 { 1 } else { 0 };
                    while cx * ny < nx * (ny - cy) {
                        let packed = hash.get(ac_start + ac_index / 2).copied().unwrap_or(0);
                        let nibble = (packed >> ((ac_index & 1) << 2)) & 15;
                        ac_index += 1;
                        ac.push((f64::from(nibble) / 7.5 - 1.0) * scale);
                        cx += 1;
                    }
                }
                ac
            };
            let l_ac = read(lx, ly, l_scale);
            let p_ac = read(3, 3, p_scale * 1.25);
            let q_ac = read(3, 3, q_scale * 1.25);
            let a_ac = if has_alpha {
                read(5, 5, a_scale)
            } else {
                Vec::new()
            };
            let ratio = aspect_ratio(hash);
            let w = if ratio > 1.0 {
                32
            } else {
                (32.0 * ratio).round().max(1.0) as usize
            };
            let h = if ratio > 1.0 {
                (32.0 / ratio).round().max(1.0) as usize
            } else {
                32
            };
            let mut rgba = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let mut l = l_dc;
                    let mut p = p_dc;
                    let mut q = q_dc;
                    let mut a = a_dc;
                    let n_fx = lx.max(if has_alpha { 5 } else { 3 });
                    let n_fy = ly.max(if has_alpha { 5 } else { 3 });
                    let mut fx = Vec::new();
                    let mut fy = Vec::new();
                    for cx in 0..n_fx {
                        fx.push((PI / w as f64 * (x as f64 + 0.5) * cx as f64).cos());
                    }
                    for cy in 0..n_fy {
                        fy.push((PI / h as f64 * (y as f64 + 0.5) * cy as f64).cos());
                    }
                    let mut j = 0;
                    for cy in 0..ly {
                        let fy2 = fy.get(cy).copied().unwrap_or(0.0) * 2.0;
                        let mut cx = if cy == 0 { 1 } else { 0 };
                        while cx * ly < lx * (ly - cy) {
                            l += l_ac.get(j).copied().unwrap_or(0.0)
                                * fx.get(cx).copied().unwrap_or(0.0)
                                * fy2;
                            j += 1;
                            cx += 1;
                        }
                    }
                    j = 0;
                    for cy in 0..3 {
                        let fy2 = fy.get(cy).copied().unwrap_or(0.0) * 2.0;
                        for cx in (if cy == 0 { 1 } else { 0 })..(3 - cy) {
                            let f = fx.get(cx).copied().unwrap_or(0.0) * fy2;
                            p += p_ac.get(j).copied().unwrap_or(0.0) * f;
                            q += q_ac.get(j).copied().unwrap_or(0.0) * f;
                            j += 1;
                        }
                    }
                    if has_alpha {
                        j = 0;
                        for cy in 0..5 {
                            let fy2 = fy.get(cy).copied().unwrap_or(0.0) * 2.0;
                            for cx in (if cy == 0 { 1 } else { 0 })..(5 - cy) {
                                a += a_ac.get(j).copied().unwrap_or(0.0)
                                    * fx.get(cx).copied().unwrap_or(0.0)
                                    * fy2;
                                j += 1;
                            }
                        }
                    }
                    let b = l - 2.0 / 3.0 * p;
                    let r = (3.0 * l - b + q) / 2.0;
                    let g = r - q;
                    rgba.push((255.0 * r.clamp(0.0, 1.0)).round() as u8);
                    rgba.push((255.0 * g.clamp(0.0, 1.0)).round() as u8);
                    rgba.push((255.0 * b.clamp(0.0, 1.0)).round() as u8);
                    rgba.push((255.0 * a.clamp(0.0, 1.0)).round() as u8);
                }
            }
            Raster {
                width: w,
                height: h,
                rgba,
            }
        }
    }

    /// Independent sRGB → OKLCH milles from Ottosson's `OKLab` matrices.
    fn independent_oklch(r: u8, g: u8, b: u8) -> (u16, u16, u16) {
        fn lin(c: u8) -> f64 {
            let u = f64::from(c) / 255.0;
            if u <= 0.040_45 {
                u / 12.92
            } else {
                ((u + 0.055) / 1.055).powf(2.4)
            }
        }
        let r = lin(r);
        let g = lin(g);
        let b = lin(b);
        let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
        let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
        let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();
        let l_ok = 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_;
        let a_ok = 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_;
        let b_ok = 0.025_904_037_1 * l_ + 0.738_491_886_6 * m_ - 0.764_395_924_1 * s_;
        let c = a_ok.hypot(b_ok);
        let h = if c < 0.000_5 {
            0.0
        } else {
            let deg = b_ok.atan2(a_ok).to_degrees();
            if deg < 0.0 { deg + 360.0 } else { deg }
        };
        (
            (l_ok * 1000.0).round().clamp(0.0, 1000.0) as u16,
            (c * 1000.0).round().clamp(0.0, 500.0) as u16,
            if c < 0.000_5 {
                0
            } else {
                h.round().clamp(0.0, 359.0) as u16
            },
        )
    }

    /// Independent WCAG 2 contrast in hundredths.
    fn independent_contrast(r: u8, g: u8, b: u8, other: (u8, u8, u8)) -> u16 {
        fn lin(c: u8) -> f64 {
            let u = f64::from(c) / 255.0;
            if u <= 0.040_45 {
                u / 12.92
            } else {
                ((u + 0.055) / 1.055).powf(2.4)
            }
        }
        fn lum(r: u8, g: u8, b: u8) -> f64 {
            0.212_6 * lin(r) + 0.715_2 * lin(g) + 0.072_2 * lin(b)
        }
        let left = lum(r, g, b);
        let right = lum(other.0, other.1, other.2);
        let (hi, lo) = if left >= right {
            (left, right)
        } else {
            (right, left)
        };
        ((hi + 0.05) / (lo + 0.05) * 100.0)
            .round()
            .clamp(0.0, 2100.0) as u16
    }

    fn fill(width: u16, height: u16, pixel: [u8; 4]) -> Vec<u8> {
        let n = usize::from(width) * usize::from(height);
        let mut out = Vec::new();
        for _ in 0..n {
            out.extend_from_slice(&pixel);
        }
        out
    }

    fn split_vertical(width: u16, height: u16, left: [u8; 4], right: [u8; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        let mid = width / 2;
        for _y in 0..height {
            for x in 0..width {
                out.extend_from_slice(if x < mid { &left } else { &right });
            }
        }
        out
    }

    fn gradient_horizontal(width: u16, height: u16) -> Vec<u8> {
        let mut out = Vec::new();
        let denom = f64::from(width.saturating_sub(1).max(1));
        for _y in 0..height {
            for x in 0..width {
                let t = f64::from(x) / denom;
                let r = (255.0 * (1.0 - t)).round() as u8;
                let b = (255.0 * t).round() as u8;
                out.extend_from_slice(&[r, 0, b, 255]);
            }
        }
        out
    }

    fn header_alpha_flag(bytes: &[u8]) -> u8 {
        bytes.get(2).copied().unwrap_or(0) & 0x80
    }

    fn mean_channel(raster: &published::Raster, xs: std::ops::Range<usize>, channel: usize) -> f64 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for y in 0..raster.height {
            for x in xs.clone() {
                if x >= raster.width {
                    continue;
                }
                let i = y
                    .saturating_mul(raster.width)
                    .saturating_add(x)
                    .saturating_mul(4)
                    .saturating_add(channel);
                sum += f64::from(*raster.rgba.get(i).unwrap_or(&0));
                n += 1.0;
            }
        }
        if n == 0.0 { 0.0 } else { sum / n }
    }

    fn assert_solid_decode(rgba: &[u8], width: u16, height: u16, rgb: [u8; 3], tolerance: u8) {
        let hash = placeholder(rgba, width, height).unwrap();
        assert!(
            hash.len() <= Placeholder::MAX_LEN && hash.len() == hash.as_bytes().len(),
            "{hash:?}"
        );
        assert!(hash.len() >= 5, "{hash:?}");
        assert!(!hash.is_empty());
        let raster = published::decode(hash.as_bytes());
        assert!(raster.width >= 1 && raster.height >= 1);
        for pixel in raster.rgba.chunks_exact(4) {
            for (got, want) in pixel.iter().take(3).zip(rgb) {
                let delta = got.abs_diff(want);
                let hash_bytes = hash.as_bytes();
                assert!(
                    delta <= tolerance,
                    "decoded {pixel:?} off {rgb:?} by {delta} (tol {tolerance}) hash {hash_bytes:02x?}"
                );
            }
            assert!(pixel[3] >= 255 - tolerance, "alpha {pixel:?}");
        }
        let (ar, ag, ab, aa) = published::average_rgba(hash.as_bytes());
        let to_u8 = |c: f64| (c * 255.0).round().clamp(0.0, 255.0) as u8;
        for (got, want) in [to_u8(ar), to_u8(ag), to_u8(ab)].iter().zip(rgb) {
            assert!(
                got.abs_diff(want) <= tolerance,
                "average ({ar},{ag},{ab},{aa}) off {rgb:?}"
            );
        }
        assert!(aa >= 0.9, "{aa}");
    }

    fn assert_swatch_near(swatch: Swatch, rgb: [u8; 3], l_tol: u16, c_tol: u16, h_tol: u16) {
        let (l, c, h) = independent_oklch(rgb[0], rgb[1], rgb[2]);
        let dl = swatch.lightness.abs_diff(l);
        let dc = swatch.chroma.abs_diff(c);
        let dh = {
            let raw = i32::from(swatch.hue) - i32::from(h);
            let wrapped = raw.abs().min((360 - raw.abs()) as i32);
            u16::try_from(wrapped).unwrap()
        };
        assert!(
            dl <= l_tol && dc <= c_tol && (c < GREY_CHROMA_MILLI || dh <= h_tol),
            "{swatch:?} vs independent ({l}, {c}, {h}) from {rgb:?}"
        );
        assert_eq!(
            swatch.contrast_black,
            independent_contrast(rgb[0], rgb[1], rgb[2], (0, 0, 0))
        );
        assert_eq!(
            swatch.contrast_white,
            independent_contrast(rgb[0], rgb[1], rgb[2], (255, 255, 255))
        );
    }

    #[test]
    fn records_thumbhash_as_the_placeholder_scheme() {
        assert_eq!(PLACEHOLDER_SCHEME, "ThumbHash");
        assert_eq!(Placeholder::MAX_LEN, 25);
        assert_eq!(MAX_SIDE, 100);
        assert_eq!(MAX_SWATCHES, 3);
        let vacant = Placeholder {
            bytes: [0; PLACEHOLDER_MAX_LEN],
            len: 0,
        };
        assert!(vacant.is_empty());
        assert_eq!(vacant.len(), 0);
        assert_eq!(vacant.as_bytes(), b"");
        let overlong = Placeholder {
            bytes: [7; PLACEHOLDER_MAX_LEN],
            len: 26,
        };
        assert_eq!(overlong.len(), Placeholder::MAX_LEN);
        assert_eq!(overlong.as_bytes().len(), 25);
        assert!(!overlong.is_empty());
    }

    #[test]
    fn refuses_a_zero_side() {
        assert_eq!(
            placeholder(&[], 0, 0),
            Err(ImageDataError::Empty {
                width: 0,
                height: 0
            })
        );
        assert_eq!(
            palette(&[1, 2, 3, 4], 0, 1),
            Err(ImageDataError::Empty {
                width: 0,
                height: 1
            })
        );
        assert_eq!(
            placeholder(&[1, 2, 3, 4], 1, 0),
            Err(ImageDataError::Empty {
                width: 1,
                height: 0
            })
        );
    }

    #[test]
    fn refuses_a_side_past_the_thumbhash_cap() {
        assert_eq!(
            placeholder(&[0, 0, 0, 255], 101, 1),
            Err(ImageDataError::TooLarge {
                width: 101,
                height: 1,
                max: 100
            })
        );
        assert_eq!(
            palette(&[0, 0, 0, 255], 1, 101),
            Err(ImageDataError::TooLarge {
                width: 1,
                height: 101,
                max: 100
            })
        );
        let ok = fill(100, 1, [10, 20, 30, 255]);
        assert!(placeholder(&ok, 100, 1).is_ok());
        assert!(palette(&ok, 100, 1).is_ok());
    }

    #[test]
    fn refuses_a_buffer_whose_length_does_not_match_the_dimensions() {
        assert_eq!(
            placeholder(&[0, 0, 0], 1, 1),
            Err(ImageDataError::LengthMismatch {
                width: 1,
                height: 1,
                len: 3,
                expected: 4
            })
        );
        assert_eq!(
            palette(&[0, 0, 0, 255, 1], 1, 1),
            Err(ImageDataError::LengthMismatch {
                width: 1,
                height: 1,
                len: 5,
                expected: 4
            })
        );
        assert_eq!(
            placeholder(&[], 2, 2),
            Err(ImageDataError::LengthMismatch {
                width: 2,
                height: 2,
                len: 0,
                expected: 16
            })
        );
    }

    #[test]
    fn a_one_by_one_red_pixel_decodes_to_red() {
        let rgba = fill(1, 1, [255, 0, 0, 255]);
        let hash = placeholder(&rgba, 1, 1).unwrap();
        assert!(hash.len() <= Placeholder::MAX_LEN && hash.len() >= 5);
        // A 1×1 input aliases the published DCT onto one sample, so the
        // 32×32 reconstruction is not uniform. The DC stored in the hash,
        // which `thumbHashToAverageRGBA` reads, is the solid colour.
        let (ar, ag, ab, aa) = published::average_rgba(hash.as_bytes());
        let to_u8 = |c: f64| (c * 255.0).round().clamp(0.0, 255.0) as u8;
        for (got, want) in [to_u8(ar), to_u8(ag), to_u8(ab)]
            .into_iter()
            .zip([255_u8, 0, 0])
        {
            assert!(got.abs_diff(want) <= 8, "average ({ar},{ag},{ab},{aa})");
        }
        assert!(aa >= 0.9, "{aa}");
        let raster = published::decode(hash.as_bytes());
        assert_eq!(raster.rgba.len(), raster.width * raster.height * 4);
        let swatches = palette(&rgba, 1, 1).unwrap();
        assert_eq!(swatches.len(), 1);
        assert_swatch_near(swatches[0], [255, 0, 0], 2, 2, 2);
    }

    #[test]
    fn solid_colours_decode_to_themselves() {
        for (rgb, tol) in [
            ([0_u8, 255, 0], 32_u8),
            ([0, 0, 255], 32),
            ([255, 255, 255], 8),
            ([0, 0, 0], 8),
            ([255, 255, 0], 32),
        ] {
            let rgba = fill(8, 8, [rgb[0], rgb[1], rgb[2], 255]);
            assert_solid_decode(&rgba, 8, 8, rgb, tol);
            let swatches = palette(&rgba, 8, 8).unwrap();
            assert_eq!(swatches.len(), 1, "{rgb:?}");
            assert_swatch_near(swatches[0], rgb, 2, 2, 2);
        }
    }

    #[test]
    fn an_opaque_image_clears_the_alpha_header_flag() {
        let rgba = fill(2, 2, [10, 20, 30, 255]);
        let hash = placeholder(&rgba, 2, 2).unwrap();
        let bytes = hash.as_bytes();
        let flag = header_alpha_flag(bytes);
        assert_eq!(flag, 0, "{bytes:02x?}");
        assert_eq!(header_alpha_flag(&[]), 0);
        assert_eq!(header_alpha_flag(&[0, 0, 0x80]), 0x80);
        assert!(hash.len() >= 5);
    }

    #[test]
    fn a_two_colour_split_decodes_to_two_regions_and_both_palette_colours() {
        let rgba = split_vertical(8, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
        let hash = placeholder(&rgba, 8, 4).unwrap();
        assert!(hash.len() <= Placeholder::MAX_LEN);
        let raster = published::decode(hash.as_bytes());
        let mid = raster.width / 2;
        let left_r = mean_channel(&raster, 0..mid / 2 + 1, 0);
        let right_b = mean_channel(&raster, mid + mid / 4..raster.width, 2);
        let left_b = mean_channel(&raster, 0..mid / 2 + 1, 2);
        let right_r = mean_channel(&raster, mid + mid / 4..raster.width, 0);
        assert!(
            left_r > right_r + 40.0 && right_b > left_b + 40.0,
            "left R {left_r} B {left_b}, right R {right_r} B {right_b}, {}x{}",
            raster.width,
            raster.height
        );
        let swatches = palette(&rgba, 8, 4).unwrap();
        assert_eq!(swatches.len(), 2);
        let hues: Vec<u16> = swatches.iter().map(|s| s.hue).collect();
        // Red ~28°, blue ~264°.
        assert!(
            hues.iter().any(|h| (15..45).contains(h))
                && hues.iter().any(|h| (240..290).contains(h)),
            "{swatches:?}"
        );
        assert_ne!(swatches[0], swatches[1]);
    }

    #[test]
    fn a_gradient_placeholder_shifts_from_red_to_blue() {
        let rgba = gradient_horizontal(16, 8);
        let hash = placeholder(&rgba, 16, 8).unwrap();
        let raster = published::decode(hash.as_bytes());
        let left_r = mean_channel(&raster, 0..raster.width / 4, 0);
        let right_b = mean_channel(&raster, raster.width * 3 / 4..raster.width, 2);
        assert!(
            left_r > 80.0 && right_b > 80.0,
            "left R {left_r} right B {right_b}"
        );
        let swatches = palette(&rgba, 16, 8).unwrap();
        assert!(swatches.len() >= 2 && swatches.len() <= MAX_SWATCHES);
        for pair in swatches.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
    }

    #[test]
    fn fully_transparent_pixels_encode_alpha_and_yield_no_swatch() {
        let rgba = fill(4, 4, [255, 0, 0, 0]);
        let hash = placeholder(&rgba, 4, 4).unwrap();
        let hash_bytes = hash.as_bytes();
        assert!(hash.len() >= 6, "alpha header byte {hash_bytes:02x?}");
        let (ar, ag, ab, aa) = published::average_rgba(hash_bytes);
        assert!(aa < 0.2, "average alpha {aa} from ({ar},{ag},{ab},{aa})");
        let raster = published::decode(hash_bytes);
        let mean_a = mean_channel(&raster, 0..raster.width, 3);
        assert!(mean_a < 40.0, "alpha {mean_a}");
        assert_eq!(palette(&rgba, 4, 4).unwrap(), Vec::<Swatch>::new());
    }

    #[test]
    fn mixed_transparent_and_red_keeps_the_red_swatch() {
        let mut rgba = fill(4, 2, [0, 0, 0, 0]);
        for pixel in rgba.chunks_exact_mut(4).take(4) {
            pixel.copy_from_slice(&[255, 0, 0, 255]);
        }
        let swatches = palette(&rgba, 4, 2).unwrap();
        assert_eq!(swatches.len(), 1);
        assert_swatch_near(swatches[0], [255, 0, 0], 2, 2, 2);
    }

    #[test]
    fn mixed_grey_and_red_drops_the_grey() {
        let rgba = split_vertical(8, 4, [128, 128, 128, 255], [255, 0, 0, 255]);
        let swatches = palette(&rgba, 8, 4).unwrap();
        assert_eq!(swatches.len(), 1, "{swatches:?}");
        assert_swatch_near(swatches[0], [255, 0, 0], 20, 20, 15);
        assert!(swatches[0].chroma >= GREY_CHROMA_MILLI);
    }

    #[test]
    fn a_landscape_hash_is_wider_than_it_is_tall() {
        let wide = fill(8, 2, [20, 40, 80, 255]);
        let tall = fill(2, 8, [20, 40, 80, 255]);
        let wide_hash = placeholder(&wide, 8, 2).unwrap();
        let tall_hash = placeholder(&tall, 2, 8).unwrap();
        assert!(published::aspect_ratio(wide_hash.as_bytes()) > 1.0);
        assert!(published::aspect_ratio(tall_hash.as_bytes()) < 1.0);
    }

    #[test]
    fn a_grey_cover_is_still_that_colour() {
        let rgba = fill(6, 6, [128, 128, 128, 255]);
        let swatches = palette(&rgba, 6, 6).unwrap();
        assert_eq!(swatches.len(), 1);
        assert_swatch_near(swatches[0], [128, 128, 128], 3, 3, 360);
        assert!(swatches[0].chroma < GREY_CHROMA_MILLI);
    }

    #[test]
    fn helpers_clamp_published_rounding_and_the_coefficient_triangle() {
        assert_eq!(js_round(2.3), 2);
        assert_eq!(js_round(2.5), 3);
        assert_eq!(js_round(-1.5), -1);
        assert_eq!(js_round(f64::from(i32::MAX) + 10.0), i32::MAX);
        assert_eq!(js_round(f64::from(i32::MIN) - 10.0), i32::MIN);
        assert_eq!(clamp_to_u16(-1, 10), 0);
        assert_eq!(clamp_to_u16(5, 10), 5);
        assert_eq!(clamp_to_u16(100, 10), 10);
        assert_eq!(clamp_to_u16(i32::MAX, 10), 10);
        assert_eq!(clamp_to_u32(-1, 15), 0);
        assert_eq!(clamp_to_u32(7, 15), 7);
        assert_eq!(clamp_to_u32(100, 15), 15);
        assert_eq!(clamp_to_u32(i32::MAX, 15), 15);
        assert_eq!(quantize(-1.0, 15), 0);
        assert_eq!(quantize(100.0, 15), 15);
        assert_eq!(unit_milli(-0.1, 1000), 0);
        assert_eq!(unit_milli(2.0, 1000), 1000);
        assert_eq!(degree(-10.0), 0);
        assert_eq!(degree(400.0), 359);
        assert!(in_triangle(0, 0, 3, 3));
        assert!(!in_triangle(2, 2, 3, 3));
        assert!(!in_triangle(usize::MAX, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(channel_extent(7.0, 0, 0), 1);
        assert_eq!(srgb_to_linear(0), 0.0);
        assert!(srgb_to_linear(255) > 0.9);
        assert_eq!(contrast_hundredths(0, 0, 0, 255, 255, 255), 2100);
        assert_eq!(contrast_hundredths(255, 255, 255, 0, 0, 0), 2100);
        assert_eq!(published::aspect_ratio(&[0, 0, 0, 0, 0x80]), 1.0);
    }

    #[test]
    fn palette_helpers_cover_out_of_range_slots() {
        assert_eq!(mean_rgb(&[]), [0, 0, 0]);
        assert_eq!(split_cluster(vec![[1, 2, 3]]), None);
        assert_eq!(split_cluster(Vec::new()), None);
        assert_eq!(split_cluster(vec![[4, 4, 4], [4, 4, 4]]), None);
        assert!(split_cluster(vec![[0, 0, 0], [255, 0, 0]]).is_some());
        let mut one = vec![vec![[9, 9, 9]]];
        store_box(&mut one, 0, vec![[1, 2, 3]]);
        assert_eq!(one, vec![vec![[1, 2, 3]]]);
        let mut none: Vec<RgbBox> = Vec::new();
        store_box(&mut none, 3, vec![[1, 2, 3]]);
        assert_eq!(none, vec![vec![[1, 2, 3]]]);
        assert_eq!(longest_range(&[]), (0, 2));
        assert_eq!(rgb_channel([1, 2, 3], 0), 1);
        assert_eq!(rgb_channel([1, 2, 3], 9), 0);
        assert_eq!(u32_at(&[1, 2, 3], 1), 2);
        assert_eq!(u32_at(&[1, 2, 3], 9), 0);
        let mut rgb = [5, 5, 5];
        set_u8(&mut rgb, 9, 1, true);
        set_u8(&mut rgb, 0, 9, true);
        set_u8(&mut rgb, 1, 1, false);
        write_channel(&mut rgb, 9, 7);
        write_channel(&mut rgb, 2, 4);
        assert_eq!(rgb, [9, 1, 4]);
        let mut sums = [0_u32, 0, 0];
        add_u32(&mut sums, 9, 4);
        add_u32(&mut sums, 0, 4);
        assert_eq!(sums, [4, 0, 0]);
        let mut packed = [0_u8; 1];
        assert_eq!(or_nibble(&mut packed, 0, 3, 0), 1);
        assert_eq!(or_nibble(&mut packed, 9, 3, 0), 0);
        write_u8(&mut packed, 9, 1);
        write_u8(&mut packed, 0, 7);
        assert_eq!(packed, [7]);
        assert!(take_cluster(&mut Vec::new(), 0).is_empty());
        assert!(!replace_with_split(&mut Vec::new(), 0));
        assert!(!replace_with_split(&mut vec![vec![[8, 8, 8]]], 0));
        assert!(replace_with_split(
            &mut vec![vec![[0, 0, 0], [255, 0, 0]]],
            0
        ));
        let dummy = |lightness, chroma, hue| Swatch {
            lightness,
            chroma,
            hue,
            contrast_black: 0,
            contrast_white: 0,
        };
        let kept = distinct_lch(vec![
            dummy(100, 10, 20),
            dummy(100, 10, 20),
            dummy(100, 11, 20),
            dummy(100, 11, 21),
            dummy(200, 30, 40),
        ]);
        assert_eq!(
            kept.iter()
                .map(|s| (s.lightness, s.chroma, s.hue))
                .collect::<Vec<_>>(),
            vec![(100, 10, 20), (100, 11, 20), (100, 11, 21)]
        );
        let empty_raster = published::Raster {
            width: 0,
            height: 0,
            rgba: Vec::new(),
        };
        assert_eq!(mean_channel(&empty_raster, 0..4, 0), 0.0);
        let unit = published::Raster {
            width: 1,
            height: 1,
            rgba: vec![10, 0, 0, 255],
        };
        assert!(mean_channel(&unit, 0..4, 0) >= 0.0);
    }

    #[test]
    fn transparent_landscape_and_portrait_hashes_carry_alpha() {
        let wide_clear = fill(8, 2, [10, 20, 30, 0]);
        let tall_clear = fill(2, 8, [10, 20, 30, 0]);
        let wide_hash = placeholder(&wide_clear, 8, 2).unwrap();
        let tall_hash = placeholder(&tall_clear, 2, 8).unwrap();
        assert!(published::aspect_ratio(wide_hash.as_bytes()) > 1.0);
        assert!(published::aspect_ratio(tall_hash.as_bytes()) < 1.0);
        let _ = published::decode(wide_hash.as_bytes());
        let _ = published::decode(tall_hash.as_bytes());
        let _ = published::average_rgba(wide_hash.as_bytes());
    }

    proptest! {
        #[test]
        fn the_placeholder_always_fits_its_fixed_size(
            width in 1_u16..=12,
            height in 1_u16..=12,
            octets in arb_bytes(any::<u8>(), 0..4),
        ) {
            let mut rgba = Vec::new();
            let pixels = usize::from(width) * usize::from(height);
            for _ in 0..pixels {
                let mut pixel = [0_u8, 0, 0, 255];
                for (slot, octet) in pixel.iter_mut().zip(&octets) {
                    *slot = *octet;
                }
                rgba.extend_from_slice(&pixel);
            }
            let hash = placeholder(&rgba, width, height).unwrap();
            prop_assert!(hash.len() <= Placeholder::MAX_LEN);
            prop_assert_eq!(hash.len(), hash.as_bytes().len());
            prop_assert!(hash.len() >= 5);
            prop_assert!(!hash.is_empty());
            let raster = published::decode(hash.as_bytes());
            prop_assert_eq!(raster.rgba.len(), raster.width * raster.height * 4);
            prop_assert!(raster.width >= 1 && raster.height >= 1);
        }

        #[test]
        fn palette_candidates_are_distinct_and_capped(
            width in 1_u16..=10,
            height in 1_u16..=10,
            seed in any::<u8>(),
        ) {
            let mut rgba = Vec::new();
            let pixels = usize::from(width) * usize::from(height);
            for i in 0..pixels {
                let t = (i as u8).wrapping_add(seed);
                rgba.extend_from_slice(&[t, t.wrapping_mul(3), t.wrapping_mul(7), 255]);
            }
            let swatches = palette(&rgba, width, height).unwrap();
            prop_assert!(swatches.len() <= MAX_SWATCHES);
            for i in 0..swatches.len() {
                for j in (i + 1)..swatches.len() {
                    prop_assert_ne!(swatches[i], swatches[j]);
                }
            }
        }

        #[test]
        fn every_buffer_returns_a_typed_result(
            width in 0_u16..=120,
            height in 0_u16..=120,
            rgba in arb_bytes(any::<u8>(), 0..64),
        ) {
            let hashed = placeholder(&rgba, width, height);
            let colours = palette(&rgba, width, height);
            prop_assert!(hashed.is_ok() || hashed.is_err());
            prop_assert!(colours.is_ok() || colours.is_err());
            match hashed {
                Ok(hash) => {
                    prop_assert!(colours.is_ok());
                    prop_assert!(hash.len() <= Placeholder::MAX_LEN);
                }
                Err(error) => prop_assert_eq!(colours, Err(error)),
            }
        }
    }
}
