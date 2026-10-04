//! The harness for the text parsers of the typed values in
//! `gunmetal_core::values` (SEC-MED-014).

use std::num::NonZeroU32;

use gunmetal_core::time::CivilDate;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::values::{
    BitDepth, Channels, Duration, GainDb, Isrc, Mbid, NumberOf, PartialDate, PeakRatio, SampleRate,
    ValueError,
};

/// The ticks a second [`Whole::ticks`] counts in: the sample rate of a
/// compact disc.
pub const TICKS_PER_SECOND: NonZeroU32 = NonZeroU32::new(44_100).unwrap();

/// What the constructors that take a whole number reported for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Whole {
    /// [`SampleRate::new`], in hertz.
    pub sample_rate: Result<u32, ValueError>,
    /// [`Channels::new`].
    pub channels: Result<u32, ValueError>,
    /// [`BitDepth::new`], in bits.
    pub bit_depth: Result<u32, ValueError>,
    /// [`Duration::from_millis`], in milliseconds.
    pub duration: Result<u64, ValueError>,
    /// [`Duration::from_ticks`] at [`TICKS_PER_SECOND`], in milliseconds.
    pub ticks: Result<u64, ValueError>,
}

/// What the constructors that take a floating-point number reported for
/// one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Float {
    /// [`GainDb::new`], in decibels.
    pub gain: Result<f32, ValueError>,
    /// [`PeakRatio::new`], as a ratio of full scale.
    pub peak: Result<f32, ValueError>,
}

/// What each value parser reported for one input, read as UTF-8 with each
/// invalid sequence replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// [`Mbid::parse`], written back in its hyphenated lower-case form.
    pub mbid: Result<String, ValueError>,
    /// [`Isrc::parse`], as the code it holds.
    pub isrc: Result<String, ValueError>,
    /// [`NumberOf::parse`]: the number and the total.
    pub number: Result<(u16, Option<u16>), ValueError>,
    /// [`PartialDate::parse`]: the year, month and day.
    pub date: Result<(u16, Option<u8>, Option<u8>), ValueError>,
    /// [`GainDb::parse`], in decibels.
    pub gain: Result<f32, ValueError>,
    /// [`PeakRatio::parse`], as a ratio of full scale.
    pub peak: Result<f32, ValueError>,
    /// The constructors that take a whole number, when the input is one in
    /// decimal. A number past `u32::MAX` is given to the 32-bit ones as
    /// `u32::MAX`.
    pub whole: Option<Whole>,
    /// The constructors that take a floating-point number, when the input
    /// is one as Rust writes them, `NaN` and `inf` included.
    pub float: Option<Float>,
    /// [`GainDb::from_q7_8`] in decibels, when the input is a signed 16-bit
    /// number in decimal.
    pub r128: Option<f32>,
}

/// Feeds `data` to the parser of every typed value read from text, and to
/// the constructors that take a number when it spells one.
///
/// # Panics
///
/// Panics when an accepted value breaks its documented form or range: an
/// identifier that is not the input in its one written form, a number or
/// total outside 1 to 9,999 or a number above its total, a date that does
/// not exist or has a day without a month, or a gain or peak that is not
/// finite or is outside its range; when a number or date read from text
/// cannot be built again from its parts; or when a sample rate, channel
/// count, bit depth or duration is accepted outside its range or is not
/// the number given.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let whole = whole(&text);
    let float = float(&text);
    let r128 = r128(&text);
    let mbid = Mbid::parse(Untrusted::new(&text)).map(|mbid| mbid.to_string());
    let isrc = Isrc::parse(Untrusted::new(&text)).map(|isrc| isrc.as_str().to_owned());
    let number =
        NumberOf::parse(Untrusted::new(&text)).map(|number| (number.number(), number.total()));
    let date_parts = PartialDate::parse(Untrusted::new(&text))
        .map(|date| (date.year(), date.month(), date.day()));
    let gain = db(GainDb::parse(Untrusted::new(&text)));
    let peak = ratio(PeakRatio::parse(Untrusted::new(&text)));
    assert!(
        mbid.as_ref()
            .ok()
            .is_none_or(|written| *written == text.to_ascii_lowercase())
            && isrc.as_ref().ok().is_none_or(|code| {
                *code == text.replace('-', "").to_ascii_uppercase()
                    && code.len() == 12
                    && code
                        .bytes()
                        .all(|octet| octet.is_ascii_uppercase() || octet.is_ascii_digit())
            })
            && number.as_ref().ok().is_none_or(|&(number, total)| {
                (1..=9999).contains(&number)
                    && total.is_none_or(|total| (number..=9999).contains(&total))
                    && NumberOf::new(u64::from(number), total.map(u64::from))
                        .map(|again| (again.number(), again.total()))
                        == Ok((number, total))
            })
            && date_parts.as_ref().ok().is_none_or(|&(year, month, day)| {
                (1..=9999).contains(&year)
                    && (month.is_some() || day.is_none())
                    && month.is_none_or(|month| (1..=12).contains(&month))
                    && day.is_none_or(|day| {
                        CivilDate::new(year, month.unwrap_or_default(), day).is_ok()
                    })
                    && PartialDate::new(year, month, day)
                        .map(|again| (again.year(), again.month(), again.day()))
                        == Ok((year, month, day))
            })
            && gain
                .as_ref()
                .ok()
                .is_none_or(|db| db.is_finite() && (-128.0..=128.0).contains(db))
            && peak
                .as_ref()
                .ok()
                .is_none_or(|ratio| ratio.is_finite() && (0.0..=16.0).contains(ratio)),
        "{text:?} gave {mbid:?}, {isrc:?}, {number:?}, {date_parts:?}, {gain:?}, {peak:?}"
    );
    Outcome {
        mbid,
        isrc,
        number,
        date: date_parts,
        gain,
        peak,
        whole,
        float,
        r128,
    }
}

/// Gives `text`, when it is a whole number in decimal, to every constructor
/// that takes one.
///
/// # Panics
///
/// Panics when a sample rate, channel count, bit depth or duration is
/// accepted outside its range, refused inside it, or is not the number
/// given; ticks are counted in milliseconds, rounded down.
fn whole(text: &str) -> Option<Whole> {
    let number = text.parse::<u64>().ok()?;
    let narrow = u32::try_from(number).unwrap_or(u32::MAX);
    let whole = Whole {
        sample_rate: SampleRate::new(narrow).map(|rate| rate.hz().get()),
        channels: Channels::new(narrow).map(|count| count.get().get()),
        bit_depth: BitDepth::new(narrow).map(|depth| depth.get().get()),
        duration: millis(Duration::from_millis(number)),
        ticks: millis(Duration::from_ticks(number, TICKS_PER_SECOND)),
    };
    assert!(
        whole.sample_rate.is_ok_and(|hz| u64::from(hz) == number)
            == (1..=768_000).contains(&number)
            && whole.channels.is_ok_and(|count| u64::from(count) == number)
                == (1..=255).contains(&number)
            && whole.bit_depth.is_ok_and(|bits| u64::from(bits) == number)
                == (1..=64).contains(&number)
            && whole.duration.is_ok_and(|length| length == number) == (number <= 2_592_000_000)
            && whole
                .ticks
                .is_ok_and(|length| { u128::from(length) == u128::from(number) * 1_000 / 44_100 })
                == (u128::from(number) * 1_000 / 44_100 <= 2_592_000_000),
        "{number} gave {whole:?}"
    );
    Some(whole)
}

/// Gives `text`, when it is a floating-point number as Rust writes them,
/// to the constructors that take one.
///
/// # Panics
///
/// Panics when a gain or peak is accepted outside its range or refused
/// inside it; a number that is not finite is in no range.
fn float(text: &str) -> Option<Float> {
    let number = text.parse::<f32>().ok()?;
    let float = Float {
        gain: db(GainDb::new(number)),
        peak: ratio(PeakRatio::new(number)),
    };
    assert!(
        float
            .gain
            .is_ok_and(|gain| gain.to_bits() == number.to_bits())
            == (-128.0..=128.0).contains(&number)
            && float
                .peak
                .is_ok_and(|peak| peak.to_bits() == number.to_bits())
                == (0.0..=16.0).contains(&number),
        "{number} gave {float:?}"
    );
    Some(float)
}

/// Gives `text`, when it is a signed 16-bit number in decimal, to
/// [`GainDb::from_q7_8`].
///
/// # Panics
///
/// Panics when the gain is not the number divided by 256, which is always
/// from -128 to just under 128 decibels.
fn r128(text: &str) -> Option<f32> {
    let raw = text.parse::<i16>().ok()?;
    let gain = GainDb::from_q7_8(raw).db();
    assert!(
        (-128.0..128.0).contains(&gain) && (gain * 256.0).to_bits() == f32::from(raw).to_bits(),
        "{raw} gave {gain}"
    );
    Some(gain)
}

/// The decibels of an accepted gain.
fn db(gain: Result<GainDb, ValueError>) -> Result<f32, ValueError> {
    Ok(gain?.db())
}

/// The ratio of an accepted peak.
fn ratio(peak: Result<PeakRatio, ValueError>) -> Result<f32, ValueError> {
    Ok(peak?.ratio())
}

/// The milliseconds of an accepted duration.
fn millis(length: Result<Duration, ValueError>) -> Result<u64, ValueError> {
    Ok(length?.millis())
}
