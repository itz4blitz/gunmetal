//! What kind of release an album is, and its content advisory
//! (API-CAT-02, API-CAT-03).

use super::coded::coded;

coded! {
    /// The primary type of a release, as `MusicBrainz` defines it.
    PrimaryType: u8 {
        /// An album.
        Album = 1,
        /// A single.
        Single = 2,
        /// An EP.
        Ep = 3,
        /// A broadcast, such as a radio show.
        Broadcast = 4,
        /// Anything else.
        Other = 5,
    }
}

coded! {
    /// A secondary type of a release, as `MusicBrainz` defines it. A release
    /// may have several.
    SecondaryType: u8 {
        /// A compilation.
        Compilation = 1,
        /// A soundtrack.
        Soundtrack = 2,
        /// Spoken word.
        Spokenword = 3,
        /// An interview.
        Interview = 4,
        /// An audiobook.
        Audiobook = 5,
        /// An audio drama.
        AudioDrama = 6,
        /// A live recording.
        Live = 7,
        /// A remix release.
        Remix = 8,
        /// A DJ mix.
        DjMix = 9,
        /// A mixtape or street release.
        Mixtape = 10,
        /// A demo.
        Demo = 11,
        /// A field recording.
        FieldRecording = 12,
    }
}

/// A release's type: one primary type when tagged, and any secondary types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ReleaseType {
    /// The primary type, when tagged.
    pub primary: Option<PrimaryType>,
    /// The secondary types, in the order tagged.
    pub secondary: Vec<SecondaryType>,
}

coded! {
    /// A content advisory from a tag, such as MP4's `rtng` (MUS-047).
    Advisory: u8 {
        /// Explicit content.
        Explicit = 1,
        /// A clean version of a release that has explicit content.
        Clean = 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_types_have_these_codes() {
        let types: Vec<(PrimaryType, u8)> =
            PrimaryType::ALL.iter().map(|t| (*t, t.code())).collect();
        assert_eq!(
            types,
            [
                (PrimaryType::Album, 1),
                (PrimaryType::Single, 2),
                (PrimaryType::Ep, 3),
                (PrimaryType::Broadcast, 4),
                (PrimaryType::Other, 5),
            ]
        );
        let read: Vec<Option<PrimaryType>> = (0..=6).map(PrimaryType::from_code).collect();
        let mut expected = vec![None];
        expected.extend(PrimaryType::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn secondary_types_have_these_codes() {
        let types: Vec<(SecondaryType, u8)> =
            SecondaryType::ALL.iter().map(|t| (*t, t.code())).collect();
        assert_eq!(
            types,
            [
                (SecondaryType::Compilation, 1),
                (SecondaryType::Soundtrack, 2),
                (SecondaryType::Spokenword, 3),
                (SecondaryType::Interview, 4),
                (SecondaryType::Audiobook, 5),
                (SecondaryType::AudioDrama, 6),
                (SecondaryType::Live, 7),
                (SecondaryType::Remix, 8),
                (SecondaryType::DjMix, 9),
                (SecondaryType::Mixtape, 10),
                (SecondaryType::Demo, 11),
                (SecondaryType::FieldRecording, 12),
            ]
        );
        let read: Vec<Option<SecondaryType>> = (0..=13).map(SecondaryType::from_code).collect();
        let mut expected = vec![None];
        expected.extend(SecondaryType::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn advisories_have_these_codes() {
        let advisories: Vec<(Advisory, u8)> =
            Advisory::ALL.iter().map(|a| (*a, a.code())).collect();
        assert_eq!(advisories, [(Advisory::Explicit, 1), (Advisory::Clean, 2)]);
        let read: Vec<Option<Advisory>> = (0..=3).map(Advisory::from_code).collect();
        assert_eq!(
            read,
            [None, Some(Advisory::Explicit), Some(Advisory::Clean), None]
        );
    }
}
