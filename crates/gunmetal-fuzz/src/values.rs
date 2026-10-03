//! The harness for the text parsers of the typed values in
//! `gunmetal_core::values` (SEC-MED-014).

use gunmetal_core::time::CivilDate;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::values::{GainDb, Isrc, Mbid, NumberOf, PartialDate, PeakRatio, ValueError};

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
}

/// Feeds `data` to the parser of every typed value read from text.
///
/// # Panics
///
/// Panics when an accepted value breaks its documented form or range: an
/// identifier that is not the input in its one written form, a number or
/// total outside 1 to 9,999 or a number above its total, a date that does
/// not exist or has a day without a month, or a gain or peak that is not
/// finite or is outside its range.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let mbid = Mbid::parse(Untrusted::new(&text)).map(|mbid| mbid.to_string());
    let isrc = Isrc::parse(Untrusted::new(&text)).map(|isrc| isrc.as_str().to_owned());
    let number =
        NumberOf::parse(Untrusted::new(&text)).map(|number| (number.number(), number.total()));
    let date_parts = PartialDate::parse(Untrusted::new(&text))
        .map(|date| (date.year(), date.month(), date.day()));
    let gain = GainDb::parse(Untrusted::new(&text)).map(GainDb::db);
    let peak = PeakRatio::parse(Untrusted::new(&text)).map(PeakRatio::ratio);
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
            })
            && date_parts.as_ref().ok().is_none_or(|&(year, month, day)| {
                (1..=9999).contains(&year)
                    && (month.is_some() || day.is_none())
                    && month.is_none_or(|month| (1..=12).contains(&month))
                    && day.is_none_or(|day| {
                        CivilDate::new(year, month.unwrap_or_default(), day).is_ok()
                    })
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
    }
}
