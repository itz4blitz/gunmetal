//! Artwork placeholder and palette, from a small RGBA buffer the scan
//! worker has already decoded and shrunk (decoding images is WP-079).
//!
//! The placeholder scheme is **`ThumbHash`** (Evan Wallace, 2023),
//! described at <https://evanw.github.io/thumbhash/>. [`placeholder`]
//! follows the published JavaScript encoder `rgbaToThumbHash` in
//! `js/thumbhash.js` of <https://github.com/evanw/thumbhash> step for step,
//! so its output matches that encoder's byte for byte: a DCT of the image in
//! a luma and two chroma channels, plus alpha when any pixel is not opaque,
//! packed with the aspect ratio into at most [`Placeholder::MAX_LEN`] octets.
//! `ThumbHash` was chosen over `BlurHash` because it keeps alpha and the
//! aspect ratio and needs no parameters.
//!
//! [`palette`] returns up to three candidates for the artwork tint in
//! `docs/ui/design-language.md`, each in OKLCH with its WCAG contrast
//! against black and white. The server checks every number again before it
//! stores one (SEC-MED-023), and the client again before it draws one.
//!
//! The input is at most [`MAX_SIDE`] pixels a side, so the work each call
//! does has a fixed ceiling whatever the pixels hold. Nothing here parses
//! an encoded format: every octet is a pixel channel, read once in order,
//! so the step budget parsers charge (SEC-MED-007) has nothing to bound.

use std::cmp::Reverse;
use std::f64::consts::PI;

/// Longest side either function accepts. The published encoder refuses
/// larger images: encoding them is slow and gains nothing.
pub const MAX_SIDE: u16 = 100;

/// At most this many palette candidates.
pub const MAX_SWATCHES: usize = 3;

/// The placeholder scheme [`placeholder`] encodes.
pub const PLACEHOLDER_SCHEME: &str = "ThumbHash";

/// OKLCH chroma, in thousandths, below which a palette cluster counts as
/// near-grey. Near-grey clusters are candidates only when nothing else is.
const NEAR_GREY_CHROMA: u16 = 40;

/// A `ThumbHash` of an artwork image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placeholder {
    bytes: [u8; Self::MAX_LEN],
    len: usize,
}

impl Placeholder {
    /// The longest `ThumbHash` the published layout can produce: 5 header
    /// octets, an alpha octet, and 38 coefficient nibbles.
    pub const MAX_LEN: usize = 25;

    /// The hash, 5 to [`Self::MAX_LEN`] octets.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&self.bytes)
    }

    /// Copies up to [`Self::MAX_LEN`] octets of `hash`.
    fn new(hash: &[u8]) -> Self {
        let mut bytes = [0; Self::MAX_LEN];
        for (slot, octet) in bytes.iter_mut().zip(hash) {
            *slot = *octet;
        }
        Self {
            bytes,
            len: hash.len().min(Self::MAX_LEN),
        }
    }
}

/// One palette candidate: an OKLCH colour, with the WCAG 2 contrast of
/// that colour against black and against white.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    /// OKLCH lightness in thousandths, 0 to 1000.
    pub lightness: u16,
    /// OKLCH chroma in thousandths, 0 to 500 (sRGB colours stay below 330).
    pub chroma: u16,
    /// OKLCH hue in whole degrees, 0 to 359; 0 when `chroma` is 0.
    pub hue: u16,
    /// WCAG 2 contrast ratio against black, in hundredths (2100 is 21:1).
    pub contrast_black: u16,
    /// WCAG 2 contrast ratio against white, in hundredths.
    pub contrast_white: u16,
}

/// Why [`placeholder`] or [`palette`] refused a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDataError {
    /// The width or the height is 0.
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
        /// [`MAX_SIDE`].
        max: u16,
    },
    /// The buffer is not `width × height × 4` octets long.
    LengthMismatch {
        /// The width given.
        width: u16,
        /// The height given.
        height: u16,
        /// Octets the buffer holds.
        len: usize,
        /// Octets `width × height` pixels need.
        expected: usize,
    },
}

/// Encodes `rgba` as a `ThumbHash`. The buffer holds `width × height`
/// pixels row by row, four octets each (red, green, blue, alpha), with
/// colour not premultiplied by alpha.
///
/// # Errors
///
/// [`ImageDataError::Empty`] when a side is 0, [`ImageDataError::TooLarge`]
/// when a side is longer than [`MAX_SIDE`], and
/// [`ImageDataError::LengthMismatch`] when the buffer is the wrong length.
pub fn placeholder(rgba: &[u8], width: u16, height: u16) -> Result<Placeholder, ImageDataError> {
    let pixels = checked_pixels(rgba, width, height)?;
    Ok(Placeholder::new(&thumbhash(&pixels, width, height)))
}

/// Up to [`MAX_SWATCHES`] palette candidates, the most common colour first.
///
/// Fully transparent pixels are left out, and an image of nothing else has
/// no candidates. The rest are split into three clusters by median cut
/// (repeatedly halving the cluster with the widest red, green or blue range
/// at its median along that channel; ties go to the later cluster and the
/// later channel). Each cluster's mean colour is a candidate, largest
/// cluster first. Near-grey candidates are dropped when any other remains,
/// and a candidate whose rounded OKLCH value repeats an earlier one is
/// dropped.
///
/// # Errors
///
/// The same refusals as [`placeholder`].
pub fn palette(rgba: &[u8], width: u16, height: u16) -> Result<Vec<Swatch>, ImageDataError> {
    let visible: Vec<[u8; 3]> = checked_pixels(rgba, width, height)?
        .into_iter()
        .filter(|&[_, _, _, alpha]| alpha != 0)
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect();
    let mut clusters = median_cut(visible);
    clusters.retain(|cluster| !cluster.is_empty());
    clusters.sort_by_key(|cluster| Reverse(cluster.len()));
    let candidates: Vec<Swatch> = clusters
        .iter()
        .map(|cluster| swatch(mean(cluster)))
        .collect();
    let coloured: Vec<Swatch> = candidates
        .iter()
        .copied()
        .filter(|candidate| candidate.chroma >= NEAR_GREY_CHROMA)
        .collect();
    let chosen = if coloured.is_empty() {
        candidates
    } else {
        coloured
    };
    let mut distinct: Vec<Swatch> = Vec::new();
    for candidate in chosen {
        if !distinct.iter().any(|kept| same_colour(*kept, candidate)) {
            distinct.push(candidate);
        }
    }
    Ok(distinct)
}

/// Checks the dimensions and the buffer length, then splits the buffer
/// into pixels.
fn checked_pixels(rgba: &[u8], width: u16, height: u16) -> Result<Vec<[u8; 4]>, ImageDataError> {
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
    let expected = usize::from(width)
        .saturating_mul(usize::from(height))
        .saturating_mul(4);
    if rgba.len() != expected {
        return Err(ImageDataError::LengthMismatch {
            width,
            height,
            len: rgba.len(),
            expected,
        });
    }
    Ok(rgba
        .chunks_exact(4)
        .map(|pixel| <[u8; 4]>::try_from(pixel).unwrap_or_default())
        .collect())
}

/// One channel's DCT: the DC term, the AC terms scaled into 0 to 1, and
/// the scale that undoes that.
struct Channel {
    dc: f64,
    ac: Vec<f64>,
    scale: f64,
}

/// `rgbaToThumbHash`, statement for statement where the arithmetic is
/// concerned, so that rounding matches the published encoder.
fn thumbhash(pixels: &[[u8; 4]], width: u16, height: u16) -> Vec<u8> {
    let (sum_red, sum_green, sum_blue, sum_alpha) = pixels.iter().fold(
        (0.0, 0.0, 0.0, 0.0),
        |(red, green, blue, alpha), &[r, g, b, a]| {
            let weight = f64::from(a) / 255.0;
            (
                red + weight / 255.0 * f64::from(r),
                green + weight / 255.0 * f64::from(g),
                blue + weight / 255.0 * f64::from(b),
                alpha + weight,
            )
        },
    );
    // The published encoder divides only when the alpha sum is not 0; the
    // colour sums are 0 whenever it is, so dividing by the smallest
    // positive number instead gives the same averages.
    let total_alpha = sum_alpha.max(f64::MIN_POSITIVE);
    let average = [
        sum_red / total_alpha,
        sum_green / total_alpha,
        sum_blue / total_alpha,
    ];
    let has_alpha = sum_alpha < f64::from(width) * f64::from(height);
    let limit = if has_alpha { 5.0 } else { 7.0 };
    let longest = f64::from(width.max(height));
    let lx = quantize(limit * f64::from(width) / longest, 7).max(1);
    let ly = quantize(limit * f64::from(height) / longest, 7).max(1);
    let planes = lpqa(pixels, average);
    let image = (width, height);
    let luma = dct(&planes.luma, image, lx.max(3), ly.max(3));
    let yellow_blue = dct(&planes.yellow_blue, image, 3, 3);
    let red_green = dct(&planes.red_green, image, 3, 3);
    let landscape = width > height;
    let header24 = pack([
        (quantize(63.0 * luma.dc, 63), 0),
        (quantize(31.5 + 31.5 * yellow_blue.dc, 63), 6),
        (quantize(31.5 + 31.5 * red_green.dc, 63), 12),
        (quantize(31.0 * luma.scale, 31), 18),
        (u32::from(has_alpha), 23),
    ]);
    let header16 = pack([
        (if landscape { ly } else { lx }, 0),
        (quantize(63.0 * yellow_blue.scale, 63), 3),
        (quantize(63.0 * red_green.scale, 63), 9),
        (u32::from(landscape), 15),
    ]);
    let [b0, b1, b2, _] = header24.to_le_bytes();
    let [b3, b4, _, _] = header16.to_le_bytes();
    let mut hash = vec![b0, b1, b2, b3, b4];
    let mut terms: Vec<f64> = luma.ac;
    terms.extend(yellow_blue.ac);
    terms.extend(red_green.ac);
    if has_alpha {
        let alpha = dct(&planes.alpha, image, 5, 5);
        hash.push(low_octet(pack([
            (quantize(15.0 * alpha.dc, 15), 0),
            (quantize(15.0 * alpha.scale, 15), 4),
        ])));
        terms.extend(alpha.ac);
    }
    for pair in terms.chunks(2) {
        let nibbles = pair
            .iter()
            .zip([0, 4])
            .map(|(term, shift)| (quantize(15.0 * term, 15), shift));
        hash.push(low_octet(pack(nibbles)));
    }
    hash
}

/// Luma, yellow–blue, red–green and alpha planes.
struct Planes {
    luma: Vec<f64>,
    yellow_blue: Vec<f64>,
    red_green: Vec<f64>,
    alpha: Vec<f64>,
}

/// Composites each pixel over the average colour and splits it into planes.
fn lpqa(pixels: &[[u8; 4]], [average_red, average_green, average_blue]: [f64; 3]) -> Planes {
    let mut planes = Planes {
        luma: Vec::new(),
        yellow_blue: Vec::new(),
        red_green: Vec::new(),
        alpha: Vec::new(),
    };
    for &[r, g, b, a] in pixels {
        let alpha = f64::from(a) / 255.0;
        let red = average_red * (1.0 - alpha) + alpha / 255.0 * f64::from(r);
        let green = average_green * (1.0 - alpha) + alpha / 255.0 * f64::from(g);
        let blue = average_blue * (1.0 - alpha) + alpha / 255.0 * f64::from(b);
        planes.luma.push((red + green + blue) / 3.0);
        planes.yellow_blue.push(f64::midpoint(red, green) - blue);
        planes.red_green.push(red - green);
        planes.alpha.push(alpha);
    }
    planes
}

/// The DCT coefficients `(cx, cy)` with `cx·ny < nx·(ny − cy)`, row by
/// row, as the published layout stores them.
fn dct(samples: &[f64], (width, height): (u16, u16), nx: u32, ny: u32) -> Channel {
    let (w, h) = (f64::from(width), f64::from(height));
    let mut channel = Channel {
        dc: 0.0,
        ac: Vec::new(),
        scale: 0.0,
    };
    for cy in 0..ny {
        for cx in (0..nx).take_while(|&cx| in_triangle(cx, cy, nx, ny)) {
            let fx: Vec<f64> = (0..width)
                .map(|x| (PI / w * f64::from(cx) * (f64::from(x) + 0.5)).cos())
                .collect();
            let mut f = 0.0;
            for (y, row) in (0..height).zip(samples.chunks_exact(usize::from(width.max(1)))) {
                let fy = (PI / h * f64::from(cy) * (f64::from(y) + 0.5)).cos();
                for (sample, basis) in row.iter().zip(&fx) {
                    f += sample * basis * fy;
                }
            }
            f /= w * h;
            if (cx, cy) == (0, 0) {
                channel.dc = f;
            } else {
                channel.ac.push(f);
                channel.scale = channel.scale.max(f.abs());
            }
        }
    }
    // A flat channel has a scale of 0, which makes every term NaN here. The
    // published encoder skips this step then and packs each term as 0;
    // `quantize` packs NaN as 0, so the result is the same.
    for term in &mut channel.ac {
        *term = 0.5 + 0.5 / channel.scale * *term;
    }
    channel
}

/// Whether coefficient `(cx, cy)` is one an `nx` × `ny` channel stores.
fn in_triangle(cx: u32, cy: u32, nx: u32, ny: u32) -> bool {
    f64::from(cx) * f64::from(ny) < f64::from(nx) * (f64::from(ny) - f64::from(cy))
}

/// Adds each value shifted left by its offset. The fields never overlap, so
/// this is the bitwise OR the published encoder writes.
fn pack(fields: impl IntoIterator<Item = (u32, u32)>) -> u32 {
    fields.into_iter().fold(0, |packed, (value, shift)| {
        packed.wrapping_add(value.wrapping_shl(shift))
    })
}

/// The low eight bits of `value`.
fn low_octet(value: u32) -> u8 {
    let [low, ..] = value.to_le_bytes();
    low
}

/// JavaScript's `Math.round` (half rounds up), clamped to `0..=max`. NaN
/// becomes 0, as the published encoder's bitwise operators make it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a whole number in 0..=max or NaN, and `as` turns NaN into 0"
)]
fn quantize(value: f64, max: u32) -> u32 {
    (value + 0.5).floor().clamp(0.0, f64::from(max)) as u32
}

/// Median cut into at most [`MAX_SWATCHES`] clusters.
fn median_cut(pixels: Vec<[u8; 3]>) -> Vec<Vec<[u8; 3]>> {
    let mut clusters = vec![pixels];
    for _ in 1..MAX_SWATCHES {
        clusters.sort_by_key(|cluster| widest(cluster).0);
        let widest_cluster = clusters.pop();
        clusters.extend(widest_cluster.into_iter().flat_map(split));
    }
    clusters
}

/// Halves `cluster` at its median along its widest channel, or returns it
/// whole when every pixel is the same colour.
fn split(mut cluster: Vec<[u8; 3]>) -> Vec<Vec<[u8; 3]>> {
    let (range, channel) = widest(&cluster);
    if range == 0 {
        return vec![cluster];
    }
    cluster.sort_by_key(|pixel| component(*pixel, channel));
    let upper = cluster.split_off(cluster.len().wrapping_div(2));
    vec![cluster, upper]
}

/// The range of `cluster` along its widest channel, and that channel.
fn widest(cluster: &[[u8; 3]]) -> (u8, usize) {
    (0..3)
        .map(|channel| {
            let values = cluster.iter().map(|pixel| component(*pixel, channel));
            let low = values.clone().min().unwrap_or(0);
            let high = values.max().unwrap_or(0);
            (high.saturating_sub(low), channel)
        })
        .max_by_key(|&(range, _)| range)
        .unwrap_or((0, 0))
}

/// Channel `channel` (0 red, 1 green, 2 blue) of `pixel`.
fn component(pixel: [u8; 3], channel: usize) -> u8 {
    pixel.get(channel).copied().unwrap_or(0)
}

/// The mean red, green and blue of `cluster` in linear light, from 0 to 1.
fn mean(cluster: &[[u8; 3]]) -> [f64; 3] {
    let (red, green, blue, count) = cluster.iter().fold(
        (0.0, 0.0, 0.0, 0.0),
        |(red, green, blue, count), &[r, g, b]| {
            (
                red + linear(r),
                green + linear(g),
                blue + linear(b),
                count + 1.0,
            )
        },
    );
    [red / count, green / count, blue / count]
}

/// Whether two swatches round to the same OKLCH colour.
fn same_colour(left: Swatch, right: Swatch) -> bool {
    (left.lightness, left.chroma, left.hue) == (right.lightness, right.chroma, right.hue)
}

/// The OKLCH swatch of a colour in linear-light sRGB, using Björn
/// Ottosson's `OKLab` matrices (2020, as CSS Color 4 uses them) and WCAG 2's
/// relative luminance.
fn swatch([red, green, blue]: [f64; 3]) -> Swatch {
    let long = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
    let medium = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
    let short = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();
    let lightness = 0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short;
    let a = 1.977_998_495_1 * long - 2.428_592_205_0 * medium + 0.450_593_709_9 * short;
    let b = 0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766_0 * short;
    let chroma = thousandths(a.hypot(b), 500);
    let hue = if chroma == 0 {
        0
    } else {
        whole(b.atan2(a).to_degrees().rem_euclid(360.0), 360).rem_euclid(360)
    };
    let luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
    Swatch {
        lightness: thousandths(lightness, 1000),
        chroma,
        hue,
        contrast_black: whole(100.0 * (luminance + 0.05) / 0.05, 2100),
        contrast_white: whole(100.0 * 1.05 / (luminance + 0.05), 2100),
    }
}

/// An sRGB channel in linear light, from 0 to 1 (IEC 61966-2-1). The
/// curve's straight segment ends at 0.04045, between 10/255 and 11/255.
fn linear(channel: u8) -> f64 {
    let unit = f64::from(channel) / 255.0;
    if channel <= 10 {
        unit / 12.92
    } else {
        ((unit + 0.055) / 1.055).powf(2.4)
    }
}

/// `value` in thousandths, rounded and clamped to `0..=max`.
fn thousandths(value: f64, max: u16) -> u16 {
    whole(value * 1000.0, max)
}

/// `value` rounded and clamped to `0..=max`.
fn whole(value: f64, max: u16) -> u16 {
    u16::try_from(quantize(value, u32::from(max))).unwrap_or(max)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::type_complexity,
    reason = "the oracles work on images of at most 100 × 100 pixels and hashes of at most 25 octets, compare values the published formulas give exactly, and keep the published decoder's names"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// An independent `ThumbHash` decoder, ported from `thumbHashToRGBA`,
    /// `thumbHashToAverageRGBA` and `thumbHashToApproximateAspectRatio` in
    /// `js/thumbhash.js` of <https://github.com/evanw/thumbhash> (commit
    /// a652ce6). It shares no code with the encoder under test.
    mod published {
        use std::f64::consts::PI;

        pub struct Raster {
            pub width: usize,
            pub height: usize,
            pub rgba: Vec<u8>,
        }

        fn header24(hash: &[u8]) -> u32 {
            u32::from(hash[0]) | u32::from(hash[1]) << 8 | u32::from(hash[2]) << 16
        }

        fn header16(hash: &[u8]) -> u32 {
            u32::from(hash[3]) | u32::from(hash[4]) << 8
        }

        /// `lx` and `ly` as `thumbHashToApproximateAspectRatio` reads them.
        fn extents(hash: &[u8]) -> (u32, u32) {
            let stored = u32::from(hash[3] & 7);
            let fixed = if hash[2] & 0x80 != 0 { 5 } else { 7 };
            if hash[4] & 0x80 != 0 {
                (fixed, stored)
            } else {
                (stored, fixed)
            }
        }

        pub fn aspect_ratio(hash: &[u8]) -> f64 {
            let (lx, ly) = extents(hash);
            f64::from(lx) / f64::from(ly)
        }

        fn has_alpha(hash: &[u8]) -> bool {
            header24(hash) >> 23 != 0
        }

        /// The DC terms `[l, p, q, a]`.
        fn dc(hash: &[u8]) -> [f64; 4] {
            let header = header24(hash);
            let a = if has_alpha(hash) {
                f64::from(hash[5] & 15) / 15.0
            } else {
                1.0
            };
            [
                f64::from(header & 63) / 63.0,
                f64::from((header >> 6) & 63) / 31.5 - 1.0,
                f64::from((header >> 12) & 63) / 31.5 - 1.0,
                a,
            ]
        }

        fn to_rgb(l: f64, p: f64, q: f64) -> [f64; 3] {
            let b = l - 2.0 / 3.0 * p;
            let r = (3.0 * l - b + q) / 2.0;
            let g = r - q;
            [r, g, b]
        }

        /// `thumbHashToAverageRGBA`.
        pub fn average_rgba(hash: &[u8]) -> [f64; 4] {
            let [l, p, q, a] = dc(hash);
            let [r, g, b] = to_rgb(l, p, q);
            [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), a]
        }

        /// `thumbHashToRGBA`, rounding each channel to the nearest octet.
        pub fn decode(hash: &[u8]) -> Raster {
            let h24 = header24(hash);
            let h16 = header16(hash);
            let alpha = has_alpha(hash);
            let (lx, ly) = extents(hash);
            let l_scale = f64::from((h24 >> 18) & 31) / 31.0;
            let p_scale = f64::from((h16 >> 3) & 63) / 63.0;
            let q_scale = f64::from((h16 >> 9) & 63) / 63.0;
            let ac_start = if alpha { 6 } else { 5 };
            let a_scale = f64::from(hash[5] >> 4) / 15.0;
            let mut nibbles = hash[ac_start..]
                .iter()
                .flat_map(|octet| [octet & 15, octet >> 4]);
            let mut channel = |nx: usize, ny: usize, scale: f64| {
                let mut terms = Vec::new();
                for cy in 0..ny {
                    for cx in 0..nx {
                        if (cx, cy) != (0, 0) && cx * ny < nx * (ny - cy) {
                            let nibble = nibbles.next().unwrap();
                            terms.push((cx, cy, (f64::from(nibble) / 7.5 - 1.0) * scale));
                        }
                    }
                }
                terms
            };
            let l_ac = channel(lx.max(3) as usize, ly.max(3) as usize, l_scale);
            let p_ac = channel(3, 3, p_scale * 1.25);
            let q_ac = channel(3, 3, q_scale * 1.25);
            let a_ac = if alpha {
                channel(5, 5, a_scale)
            } else {
                Vec::new()
            };
            let ratio = aspect_ratio(hash);
            let (w, h) = if ratio > 1.0 {
                (32.0, (32.0 / ratio).round())
            } else {
                ((32.0 * ratio).round(), 32.0)
            };
            let [l_dc, p_dc, q_dc, a_dc] = dc(hash);
            let mut rgba = Vec::new();
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let basis = |cx: usize, cy: usize| {
                        (PI / w * (x as f64 + 0.5) * cx as f64).cos()
                            * (PI / h * (y as f64 + 0.5) * cy as f64).cos()
                            * 2.0
                    };
                    let sum = |start: f64, terms: &[(usize, usize, f64)]| {
                        terms
                            .iter()
                            .fold(start, |acc, &(cx, cy, f)| acc + f * basis(cx, cy))
                    };
                    let [r, g, b] = to_rgb(sum(l_dc, &l_ac), sum(p_dc, &p_ac), sum(q_dc, &q_ac));
                    for value in [r, g, b, sum(a_dc, &a_ac)] {
                        rgba.push((255.0 * value.clamp(0.0, 1.0)).round() as u8);
                    }
                }
            }
            Raster {
                width: w as usize,
                height: h as usize,
                rgba,
            }
        }
    }

    /// How far, in octets, each decoded channel of a flat image may stray
    /// from the original. The header keeps the luma and both chroma DC terms
    /// in 6 bits each, so a flat colour comes back within a few octets.
    const FLAT_TOLERANCE: u8 = 6;

    fn image(width: u16, height: u16, pixel: impl Fn(usize, usize) -> [u8; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        for y in 0..usize::from(height) {
            for x in 0..usize::from(width) {
                out.extend_from_slice(&pixel(x, y));
            }
        }
        out
    }

    fn fill(width: u16, height: u16, pixel: [u8; 4]) -> Vec<u8> {
        image(width, height, |_, _| pixel)
    }

    /// The same pattern the published encoder was run on (`noise` in the
    /// generator): channel values from the pixel's row-major index.
    fn noise(width: u16, height: u16, opaque: bool) -> Vec<u8> {
        image(width, height, |x, y| {
            let i = y * usize::from(width) + x;
            let alpha = if opaque { 255 } else { (i * 53 + 29) % 256 };
            [
                (i * 37 % 256) as u8,
                ((i * 91 + 11) % 256) as u8,
                ((i * 13 + 7) % 256) as u8,
                alpha as u8,
            ]
        })
    }

    fn halves(width: u16, height: u16, left: [u8; 4], right: [u8; 4]) -> Vec<u8> {
        let mid = usize::from(width / 2);
        image(width, height, |x, _| if x < mid { left } else { right })
    }

    /// Red falling and blue rising across the columns: `255 - 17x`, `17x`.
    fn gradient() -> Vec<u8> {
        image(16, 8, |x, _| [(255 - 17 * x) as u8, 0, (17 * x) as u8, 255])
    }

    fn hash(rgba: &[u8], width: u16, height: u16) -> Vec<u8> {
        placeholder(rgba, width, height)
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    /// Mean of `channel` over the decoded columns `columns`.
    fn region_mean(
        raster: &published::Raster,
        columns: std::ops::Range<usize>,
        channel: usize,
    ) -> f64 {
        let mut sum = 0.0;
        let mut count = 0.0;
        for y in 0..raster.height {
            for x in columns.clone() {
                sum += f64::from(raster.rgba[(y * raster.width + x) * 4 + channel]);
                count += 1.0;
            }
        }
        sum / count
    }

    fn octet(unit: f64) -> u8 {
        (unit * 255.0).round() as u8
    }

    const fn swatch(
        lightness: u16,
        chroma: u16,
        hue: u16,
        contrast_black: u16,
        contrast_white: u16,
    ) -> Swatch {
        Swatch {
            lightness,
            chroma,
            hue,
            contrast_black,
            contrast_white,
        }
    }

    // OKLCH values match CSS Color 4's conversion of the sRGB primaries
    // (red is oklch(62.8% 0.2577 29.23), blue oklch(45.2% 0.3132 264.05),
    // lime oklch(86.6% 0.2948 142.5)); contrast is WCAG 2's ratio against
    // black and white, in hundredths.
    const RED: Swatch = swatch(628, 258, 29, 525, 400);
    const LIME: Swatch = swatch(866, 295, 142, 1530, 137);
    const BLUE: Swatch = swatch(452, 313, 264, 244, 859);
    const WHITE: Swatch = swatch(1000, 0, 0, 2100, 100);
    const BLACK: Swatch = swatch(0, 0, 0, 100, 2100);
    const YELLOW: Swatch = swatch(968, 211, 110, 1956, 107);
    const GREY: Swatch = swatch(600, 0, 0, 532, 395);

    #[test]
    fn records_thumbhash_and_its_limits() {
        assert_eq!(PLACEHOLDER_SCHEME, "ThumbHash");
        assert_eq!(Placeholder::MAX_LEN, 25);
        assert_eq!(MAX_SIDE, 100);
        assert_eq!(MAX_SWATCHES, 3);
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
            placeholder(&fill(101, 1, [0, 0, 0, 255]), 101, 1),
            Err(ImageDataError::TooLarge {
                width: 101,
                height: 1,
                max: 100
            })
        );
        assert_eq!(
            palette(&fill(1, 101, [0, 0, 0, 255]), 1, 101),
            Err(ImageDataError::TooLarge {
                width: 1,
                height: 101,
                max: 100
            })
        );
        assert_eq!(
            palette(&fill(100, 1, [255, 0, 0, 255]), 100, 1),
            Ok(vec![RED])
        );
        assert_eq!(
            palette(&fill(1, 100, [255, 0, 0, 255]), 1, 100),
            Ok(vec![RED])
        );
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
            placeholder(&[0; 12], 2, 2),
            Err(ImageDataError::LengthMismatch {
                width: 2,
                height: 2,
                len: 12,
                expected: 16
            })
        );
    }

    /// Hashes of `noise` images, produced by the published JavaScript
    /// encoder `rgbaToThumbHash` (evanw/thumbhash commit a652ce6, run under
    /// Node). Every coefficient of these images is far from zero, so the
    /// bytes do not depend on the last bit of `cos`.
    #[test]
    fn matches_the_published_encoder_byte_for_byte() {
        let cases: [(u16, u16, bool, &[u8]); 5] = [
            (
                10,
                10,
                true,
                &[
                    0x1f, 0xf8, 0x05, 0x1f, 0x04, 0x00, 0x29, 0x68, 0x53, 0x6b, 0x68, 0x64, 0x65,
                    0x78, 0x97, 0x27, 0x48, 0x75, 0xf2, 0x8c, 0x6b, 0x27, 0xb3, 0x00,
                ],
            ),
            (
                7,
                5,
                false,
                &[
                    0x1e, 0xe8, 0x85, 0x14, 0x8e, 0x17, 0x90, 0x5a, 0x85, 0x68, 0x76, 0x39, 0x58,
                    0xaf, 0xf6, 0x50, 0x99, 0x67, 0x72, 0x76, 0x37, 0xa3, 0x77, 0x73, 0x00,
                ],
            ),
            (
                100,
                100,
                false,
                &[
                    0xe0, 0xf7, 0x81, 0x05, 0x00, 0x07, 0x48, 0x08, 0x75, 0x53, 0xb5, 0x54, 0x63,
                    0x90, 0xc9, 0xf9, 0x57, 0x6c, 0x5a, 0x3c, 0xfa, 0xe5, 0xb6, 0x95, 0x6b,
                ],
            ),
            (
                100,
                1,
                true,
                &[
                    0x1f, 0xf8, 0x3d, 0x09, 0x82, 0x77, 0x77, 0x77, 0x88, 0x88, 0x08, 0x88, 0x1f,
                    0x88, 0x17, 0x80, 0xd8,
                ],
            ),
            (
                1,
                100,
                true,
                &[
                    0x1f, 0xf8, 0x3d, 0x09, 0x02, 0x08, 0x87, 0x78, 0x88, 0x87, 0x87, 0x77, 0x78,
                    0x8f, 0x81, 0x1d, 0x08,
                ],
            ),
        ];
        for (width, height, opaque, expected) in cases {
            assert_eq!(hash(&noise(width, height, opaque), width, height), expected);
        }
    }

    /// The header and length the published encoder writes for flat and
    /// two-tone images. Their AC nibbles come from rounding noise in `cos`
    /// (a flat channel's scale is 0), so only the header is compared.
    #[test]
    fn writes_the_published_header_for_flat_and_structured_images() {
        let cases: [(Vec<u8>, u16, u16, &[u8], usize); 9] = [
            (
                fill(8, 8, [255, 0, 0, 255]),
                8,
                8,
                &[0xd5, 0xfb, 0x03, 0x07, 0x00],
                24,
            ),
            (
                halves(8, 4, [255, 0, 0, 255], [0, 0, 255, 255]),
                8,
                4,
                &[0x15, 0xf6, 0x02, 0xf4, 0xa8],
                19,
            ),
            (gradient(), 16, 8, &[0x15, 0xf6, 0x02, 0xa4, 0x9c], 19),
            (
                fill(1, 1, [255, 0, 0, 255]),
                1,
                1,
                &[0xd5, 0xfb, 0x2b, 0x07, 0x7f],
                24,
            ),
            (
                fill(8, 2, [20, 40, 80, 255]),
                8,
                2,
                &[0x4c, 0xd6, 0x01, 0x02, 0x80],
                17,
            ),
            (
                fill(2, 8, [20, 40, 80, 255]),
                2,
                8,
                &[0x4c, 0xd6, 0x01, 0x02, 0x00],
                17,
            ),
            (
                fill(4, 4, [255, 0, 0, 0]),
                4,
                4,
                &[0x00, 0x08, 0x82, 0x05, 0x00, 0x00],
                25,
            ),
            (
                fill(8, 2, [10, 20, 30, 0]),
                8,
                2,
                &[0x00, 0x08, 0x82, 0x01, 0x80, 0x00],
                23,
            ),
            (
                fill(2, 8, [10, 20, 30, 0]),
                2,
                8,
                &[0x00, 0x08, 0x82, 0x01, 0x00, 0x00],
                23,
            ),
        ];
        for (rgba, width, height, header, len) in cases {
            let bytes = hash(&rgba, width, height);
            assert_eq!((&bytes[..header.len()], bytes.len()), (header, len));
        }
    }

    #[test]
    fn a_landscape_or_portrait_hash_keeps_its_aspect_ratio() {
        let ratios = [
            (fill(8, 2, [20, 40, 80, 255]), 8, 2, 3.5),
            (fill(2, 8, [20, 40, 80, 255]), 2, 8, 2.0 / 7.0),
            (fill(8, 2, [20, 40, 80, 0]), 8, 2, 5.0),
            (fill(2, 8, [20, 40, 80, 0]), 2, 8, 0.2),
            (fill(5, 5, [20, 40, 80, 255]), 5, 5, 1.0),
        ];
        for (rgba, width, height, ratio) in ratios {
            assert_eq!(published::aspect_ratio(&hash(&rgba, width, height)), ratio);
        }
        let wide = published::decode(&hash(&fill(8, 2, [20, 40, 80, 0]), 8, 2));
        assert_eq!((wide.width, wide.height), (32, 6));
        let tall = published::decode(&hash(&fill(2, 8, [20, 40, 80, 255]), 2, 8));
        assert_eq!((tall.width, tall.height), (9, 32));
    }

    #[test]
    fn a_solid_colour_decodes_to_that_colour() {
        for rgb in [
            [255_u8, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 255],
            [0, 0, 0],
            [255, 255, 0],
            [128, 128, 128],
            [20, 40, 80],
        ] {
            let [r, g, b] = rgb;
            let bytes = hash(&fill(8, 8, [r, g, b, 255]), 8, 8);
            let raster = published::decode(&bytes);
            assert_eq!(raster.rgba.len(), 32 * 32 * 4);
            for pixel in raster.rgba.chunks_exact(4) {
                for (got, want) in pixel.iter().zip([r, g, b, 255]) {
                    assert!(got.abs_diff(want) <= FLAT_TOLERANCE);
                }
            }
        }
    }

    /// A 1×1 image aliases the DCT onto one sample: the published decoder
    /// draws a pattern around it, and the average it reports is the colour.
    #[test]
    fn a_one_pixel_image_averages_to_its_colour() {
        let bytes = hash(&[255, 0, 0, 255], 1, 1);
        let [r, g, b, a] = published::average_rgba(&bytes);
        for (got, want) in [octet(r), octet(g), octet(b), octet(a)]
            .into_iter()
            .zip([255, 0, 0, 255])
        {
            assert!(got.abs_diff(want) <= FLAT_TOLERANCE);
        }
        assert_eq!(palette(&[255, 0, 0, 255], 1, 1), Ok(vec![RED]));
    }

    #[test]
    fn a_two_colour_split_decodes_to_two_regions() {
        let raster = published::decode(&hash(
            &halves(8, 4, [255, 0, 0, 255], [0, 0, 255, 255]),
            8,
            4,
        ));
        let quarter = raster.width / 4;
        let left = 0..quarter;
        let right = raster.width - quarter..raster.width;
        assert!(region_mean(&raster, left.clone(), 0) > 200.0);
        assert!(region_mean(&raster, left, 2) < 55.0);
        assert!(region_mean(&raster, right.clone(), 0) < 55.0);
        assert!(region_mean(&raster, right, 2) > 200.0);
    }

    #[test]
    fn a_gradient_decodes_to_a_gradient() {
        let raster = published::decode(&hash(&gradient(), 16, 8));
        let quarter = raster.width / 4;
        let columns: Vec<_> = (0..4).map(|n| n * quarter..(n + 1) * quarter).collect();
        let reds: Vec<f64> = columns
            .iter()
            .map(|c| region_mean(&raster, c.clone(), 0))
            .collect();
        let blues: Vec<f64> = columns
            .iter()
            .map(|c| region_mean(&raster, c.clone(), 2))
            .collect();
        for pair in reds.windows(2) {
            assert!(pair[0] > pair[1] + 20.0);
        }
        for pair in blues.windows(2) {
            assert!(pair[0] + 20.0 < pair[1]);
        }
    }

    #[test]
    fn fully_transparent_pixels_decode_transparent_and_give_no_swatch() {
        let rgba = fill(4, 4, [255, 0, 0, 0]);
        let bytes = hash(&rgba, 4, 4);
        assert_eq!(published::average_rgba(&bytes)[3], 0.0);
        let raster = published::decode(&bytes);
        assert!(raster.rgba.chunks_exact(4).all(|pixel| pixel[3] == 0));
        assert_eq!(palette(&rgba, 4, 4), Ok(Vec::new()));
    }

    #[test]
    fn the_palette_of_a_solid_image_is_that_colour() {
        for (rgb, expected) in [
            ([255_u8, 0, 0], RED),
            ([0, 255, 0], LIME),
            ([0, 0, 255], BLUE),
            ([255, 255, 255], WHITE),
            ([0, 0, 0], BLACK),
            ([255, 255, 0], YELLOW),
            ([128, 128, 128], GREY),
        ] {
            let [r, g, b] = rgb;
            assert_eq!(
                palette(&fill(6, 6, [r, g, b, 255]), 6, 6),
                Ok(vec![expected])
            );
        }
    }

    #[test]
    fn the_palette_of_a_two_colour_split_is_both_colours() {
        let rgba = halves(8, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
        assert_eq!(palette(&rgba, 8, 4), Ok(vec![RED, BLUE]));
    }

    /// Median cut on the gradient, worked by hand: the first cut falls
    /// between columns 7 and 8 (red and blue span the same range, and a tie
    /// goes to the later channel, blue), the second splits the later of the
    /// two equally wide halves between columns 11 and 12. Each candidate is
    /// its columns' mean in linear light, the largest cluster first.
    #[test]
    fn the_palette_of_a_gradient_is_three_median_cut_means() {
        assert_eq!(
            palette(&gradient(), 16, 8),
            Ok(vec![
                swatch(532, 212, 13, 356, 589),
                swatch(400, 211, 302, 202, 1038),
                swatch(426, 285, 268, 221, 952),
            ])
        );
    }

    #[test]
    fn the_largest_cluster_comes_first() {
        let red = [255, 0, 0, 255];
        let lime = [0, 255, 0, 255];
        let blue = [0, 0, 255, 255];
        let rgba = image(5, 2, |x, y| match (y, x) {
            (0, _) => red,
            (_, 0 | 1) => lime,
            _ => blue,
        });
        assert_eq!(palette(&rgba, 5, 2), Ok(vec![RED, BLUE, LIME]));
    }

    #[test]
    fn a_near_grey_cluster_gives_way_to_a_coloured_one() {
        let rgba = halves(8, 4, [128, 128, 128, 255], [255, 0, 0, 255]);
        assert_eq!(palette(&rgba, 8, 4), Ok(vec![RED]));
    }

    /// (88, 60, 60) has OKLCH chroma 0.0397 and (91, 63, 63) 0.0394: they
    /// round to 40 and 39 thousandths, either side of the near-grey line.
    #[test]
    fn the_near_grey_line_is_a_chroma_of_forty_thousandths() {
        let rgba = halves(4, 2, [91, 63, 63, 255], [88, 60, 60, 255]);
        assert_eq!(
            palette(&rgba, 4, 2),
            Ok(vec![swatch(388, 40, 19, 213, 988)])
        );
    }

    /// (0, 30, 215) and (1, 30, 215) are two clusters with the same OKLCH
    /// value once rounded, so only the first is a candidate.
    #[test]
    fn clusters_with_the_same_colour_collapse_into_one_candidate() {
        let rgba = halves(4, 2, [0, 30, 215, 255], [1, 30, 215, 255]);
        assert_eq!(
            palette(&rgba, 4, 2),
            Ok(vec![swatch(415, 264, 264, 217, 969)])
        );
    }

    #[test]
    fn only_fully_transparent_pixels_are_left_out_of_the_palette() {
        let hidden = halves(4, 2, [255, 0, 0, 255], [0, 0, 255, 0]);
        assert_eq!(palette(&hidden, 4, 2), Ok(vec![RED]));
        let faint = halves(4, 2, [255, 0, 0, 255], [0, 0, 255, 1]);
        assert_eq!(palette(&faint, 4, 2), Ok(vec![RED, BLUE]));
    }

    /// sRGB's transfer curve is a straight line up to 0.04045, so a channel
    /// of 10 (0.0392) is on the line and 11 (0.0431) on the power curve. On
    /// the power curve, (10, 0, 30) would have a hue of 295 degrees.
    #[test]
    fn a_channel_of_ten_is_on_the_straight_part_of_the_srgb_curve() {
        assert_eq!(
            palette(&[10, 0, 30, 255], 1, 1),
            Ok(vec![swatch(126, 69, 296, 103, 2036)])
        );
    }

    #[test]
    fn quantize_rounds_half_up_and_clamps_like_the_published_encoder() {
        assert_eq!(quantize(2.4, 15), 2);
        assert_eq!(quantize(2.5, 15), 3);
        assert_eq!(quantize(-0.5, 15), 0);
        assert_eq!(quantize(-7.0, 15), 0);
        assert_eq!(quantize(15.4, 15), 15);
        assert_eq!(quantize(99.0, 15), 15);
        assert_eq!(quantize(f64::NAN, 15), 0);
    }

    /// Coefficients the published layout stores for an `nx` × `ny` channel,
    /// not counting the DC term.
    fn ac_count(nx: usize, ny: usize) -> usize {
        let mut count = 0;
        for cy in 0..ny {
            for cx in 0..nx {
                if cx * ny < nx * (ny - cy) {
                    count += 1;
                }
            }
        }
        count - 1
    }

    /// The hash length the published layout gives a `width` × `height`
    /// image, with or without alpha.
    fn published_len(width: u16, height: u16, alpha: bool) -> usize {
        let limit = if alpha { 5.0 } else { 7.0 };
        let longest = f64::from(width.max(height));
        let lx = ((limit * f64::from(width) / longest).round() as usize).max(1);
        let ly = ((limit * f64::from(height) / longest).round() as usize).max(1);
        let alpha_terms = if alpha { ac_count(5, 5) } else { 0 };
        let nibbles = ac_count(lx.max(3), ly.max(3)) + 2 * ac_count(3, 3) + alpha_terms;
        5 + usize::from(alpha) + nibbles.div_ceil(2)
    }

    fn sized_image(max_side: u16) -> impl Strategy<Value = (u16, u16, Vec<u8>)> {
        (1..=max_side, 1..=max_side).prop_flat_map(|(width, height)| {
            let len = usize::from(width) * usize::from(height) * 4;
            (Just(width), Just(height), vec(any::<u8>(), len))
        })
    }

    /// What `placeholder` and `palette` must refuse, worked out from the
    /// documented rules alone.
    fn expected_refusal(rgba: &[u8], width: u16, height: u16) -> Option<ImageDataError> {
        let expected = usize::from(width) * usize::from(height) * 4;
        if width == 0 || height == 0 {
            Some(ImageDataError::Empty { width, height })
        } else if width > 100 || height > 100 {
            Some(ImageDataError::TooLarge {
                width,
                height,
                max: 100,
            })
        } else if rgba.len() == expected {
            None
        } else {
            Some(ImageDataError::LengthMismatch {
                width,
                height,
                len: rgba.len(),
                expected,
            })
        }
    }

    proptest! {
        #[test]
        fn the_placeholder_always_fits_the_published_layout(
            (width, height, mut rgba) in sized_image(24),
            opaque in any::<bool>(),
        ) {
            if opaque {
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel[3] = 255;
                }
            }
            let alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 255);
            let bytes = hash(&rgba, width, height);
            prop_assert_eq!(bytes.len(), published_len(width, height, alpha));
            prop_assert!(bytes.len() <= Placeholder::MAX_LEN);
            prop_assert_eq!(bytes[2] & 0x80 != 0, alpha);
            prop_assert_eq!(bytes[4] & 0x80 != 0, width > height);
        }

        #[test]
        fn palette_candidates_are_distinct_and_at_most_three(
            (width, height, rgba) in sized_image(16),
        ) {
            let swatches = palette(&rgba, width, height).unwrap();
            prop_assert!(swatches.len() <= MAX_SWATCHES);
            for (i, left) in swatches.iter().enumerate() {
                for right in &swatches[i + 1..] {
                    prop_assert_ne!(
                        (left.lightness, left.chroma, left.hue),
                        (right.lightness, right.chroma, right.hue)
                    );
                }
            }
        }

        #[test]
        fn transparent_pixels_do_not_change_the_palette(
            (width, height, rgba) in sized_image(16),
            clear in vec(any::<[u8; 3]>(), 1..=16),
        ) {
            // The same image with a fully transparent row appended below.
            let mut taller = rgba.clone();
            for column in 0..usize::from(width) {
                let [r, g, b] = clear[column % clear.len()];
                taller.extend_from_slice(&[r, g, b, 0]);
            }
            prop_assert_eq!(
                palette(&taller, width, height + 1),
                palette(&rgba, width, height)
            );
        }

        #[test]
        fn every_buffer_gets_a_result_or_the_documented_refusal(
            (width, height, rgba) in prop_oneof![
                (0_u16..=120, 0_u16..=120, vec(any::<u8>(), 0..64)),
                sized_image(4),
            ],
        ) {
            let refusal = expected_refusal(&rgba, width, height);
            prop_assert_eq!(placeholder(&rgba, width, height).err(), refusal);
            prop_assert_eq!(palette(&rgba, width, height).err(), refusal);
        }
    }
}
