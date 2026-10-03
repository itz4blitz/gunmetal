//! Where a track sits on its release: track and disc numbers and totals.

use crate::values::NumberOf;

use super::error::CatalogError;

/// One part of a [`TrackPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionPart {
    /// The track number.
    Track,
    /// The number of tracks.
    TrackTotal,
    /// The disc number.
    Disc,
    /// The number of discs.
    DiscTotal,
}

/// A track's number and its disc's number, each with its total when known.
///
/// Every part is from 1 to 9,999 ([`NumberOf::MAX`]); a number of 0, such
/// as disc 0, is refused. Unlike [`NumberOf`], a number above its total is
/// kept, because taggers often get the total wrong and the number is still
/// useful, and it is flagged so the scan can record it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TrackPosition {
    track: Option<u16>,
    track_total: Option<u16>,
    disc: Option<u16>,
    disc_total: Option<u16>,
}

impl TrackPosition {
    /// Track `track` of `track_total` on disc `disc` of `disc_total`.
    ///
    /// # Errors
    ///
    /// [`CatalogError::OutOfRange`] for the first part, in that order, that
    /// is outside 1 to [`NumberOf::MAX`].
    pub fn new(
        track: Option<u16>,
        track_total: Option<u16>,
        disc: Option<u16>,
        disc_total: Option<u16>,
    ) -> Result<Self, CatalogError> {
        Ok(Self {
            track: part(PositionPart::Track, track)?,
            track_total: part(PositionPart::TrackTotal, track_total)?,
            disc: part(PositionPart::Disc, disc)?,
            disc_total: part(PositionPart::DiscTotal, disc_total)?,
        })
    }

    /// The track number, when known.
    #[must_use]
    pub const fn track(self) -> Option<u16> {
        self.track
    }

    /// The number of tracks, when known.
    #[must_use]
    pub const fn track_total(self) -> Option<u16> {
        self.track_total
    }

    /// The disc number, when known.
    #[must_use]
    pub const fn disc(self) -> Option<u16> {
        self.disc
    }

    /// The number of discs, when known.
    #[must_use]
    pub const fn disc_total(self) -> Option<u16> {
        self.disc_total
    }

    /// Whether the track number is above the number of tracks.
    #[must_use]
    pub fn track_above_total(self) -> bool {
        above(self.track, self.track_total)
    }

    /// Whether the disc number is above the number of discs.
    #[must_use]
    pub fn disc_above_total(self) -> bool {
        above(self.disc, self.disc_total)
    }
}

/// `value` when it is missing or from 1 to [`NumberOf::MAX`].
fn part(part: PositionPart, value: Option<u16>) -> Result<Option<u16>, CatalogError> {
    match value {
        Some(number) if !(1..=NumberOf::MAX).contains(&number) => Err(CatalogError::OutOfRange {
            part,
            value: number,
        }),
        _ => Ok(value),
    }
}

/// Whether both are known and `number` is above `total`.
fn above(number: Option<u16>, total: Option<u16>) -> bool {
    matches!((number, total), (Some(number), Some(total)) if number > total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out_of_range(part: PositionPart, value: u16) -> Result<TrackPosition, CatalogError> {
        Err(CatalogError::OutOfRange { part, value })
    }

    #[test]
    fn keeps_every_part() {
        let position = TrackPosition::new(Some(3), Some(12), Some(2), Some(4)).unwrap();
        assert_eq!(
            (
                position.track(),
                position.track_total(),
                position.disc(),
                position.disc_total(),
                position.track_above_total(),
                position.disc_above_total(),
            ),
            (Some(3), Some(12), Some(2), Some(4), false, false)
        );
    }

    #[test]
    fn every_part_may_be_missing() {
        let position = TrackPosition::new(None, None, None, None).unwrap();
        assert_eq!(position, TrackPosition::default());
        assert_eq!(
            (
                position.track(),
                position.track_total(),
                position.disc(),
                position.disc_total(),
                position.track_above_total(),
                position.disc_above_total(),
            ),
            (None, None, None, None, false, false)
        );
    }

    #[test]
    fn refuses_disc_zero() {
        assert_eq!(
            TrackPosition::new(Some(1), None, Some(0), Some(2)),
            out_of_range(PositionPart::Disc, 0)
        );
    }

    #[test]
    fn refuses_zero_and_above_the_maximum_in_every_part() {
        let cases = [
            (
                TrackPosition::new(Some(0), None, None, None),
                out_of_range(PositionPart::Track, 0),
            ),
            (
                TrackPosition::new(Some(10_000), None, None, None),
                out_of_range(PositionPart::Track, 10_000),
            ),
            (
                TrackPosition::new(None, Some(0), None, None),
                out_of_range(PositionPart::TrackTotal, 0),
            ),
            (
                TrackPosition::new(None, Some(u16::MAX), None, None),
                out_of_range(PositionPart::TrackTotal, u16::MAX),
            ),
            (
                TrackPosition::new(None, None, Some(10_000), None),
                out_of_range(PositionPart::Disc, 10_000),
            ),
            (
                TrackPosition::new(None, None, None, Some(0)),
                out_of_range(PositionPart::DiscTotal, 0),
            ),
            (
                TrackPosition::new(None, None, None, Some(10_000)),
                out_of_range(PositionPart::DiscTotal, 10_000),
            ),
        ];
        for (given, expected) in cases {
            assert_eq!(given, expected);
        }
    }

    #[test]
    fn reports_the_first_bad_part() {
        assert_eq!(
            TrackPosition::new(Some(0), Some(0), Some(0), Some(0)),
            out_of_range(PositionPart::Track, 0)
        );
    }

    #[test]
    fn accepts_one_and_the_maximum() {
        let low = TrackPosition::new(Some(1), Some(1), Some(1), Some(1)).unwrap();
        let high = TrackPosition::new(Some(9_999), Some(9_999), Some(9_999), Some(9_999)).unwrap();
        assert_eq!(
            [
                (low.track(), low.track_total(), low.disc(), low.disc_total()),
                (
                    high.track(),
                    high.track_total(),
                    high.disc(),
                    high.disc_total()
                ),
            ],
            [
                (Some(1), Some(1), Some(1), Some(1)),
                (Some(9_999), Some(9_999), Some(9_999), Some(9_999)),
            ]
        );
    }

    #[test]
    fn keeps_a_track_number_above_its_total_and_flags_it() {
        let position = TrackPosition::new(Some(13), Some(12), None, None).unwrap();
        assert_eq!(
            (
                position.track(),
                position.track_total(),
                position.track_above_total(),
                position.disc_above_total(),
            ),
            (Some(13), Some(12), true, false)
        );
    }

    #[test]
    fn keeps_a_disc_number_above_its_total_and_flags_it() {
        let position = TrackPosition::new(None, None, Some(3), Some(2)).unwrap();
        assert_eq!(
            (
                position.disc(),
                position.disc_total(),
                position.track_above_total(),
                position.disc_above_total(),
            ),
            (Some(3), Some(2), false, true)
        );
    }

    #[test]
    fn a_number_equal_to_its_total_is_not_flagged() {
        let position = TrackPosition::new(Some(12), Some(12), Some(2), Some(2)).unwrap();
        assert_eq!(
            (position.track_above_total(), position.disc_above_total()),
            (false, false)
        );
    }

    #[test]
    fn a_number_without_a_total_is_not_flagged() {
        let position = TrackPosition::new(Some(12), None, Some(2), None).unwrap();
        assert_eq!(
            (position.track_above_total(), position.disc_above_total()),
            (false, false)
        );
    }
}
