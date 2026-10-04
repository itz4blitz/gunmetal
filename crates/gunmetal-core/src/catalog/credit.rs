//! Credited people and their roles (API-CAT-01).
//!
//! A [`Credit`] is one name as a tag gave it, with the role the tag gave
//! it. Splitting a display credit such as "A feat. B" into credits is the
//! credit splitter's job (WP-053); people with typed roles across items are
//! R1.3 (WP-161).

use crate::values::Mbid;

use super::coded::coded;
use super::error::CatalogError;

coded! {
    /// What a credited person did on a recording.
    Role: u8 {
        /// The recording's artist.
        Artist = 1,
        /// The release's artist.
        AlbumArtist = 2,
        /// A featured artist.
        Featured = 3,
        /// The composer.
        Composer = 4,
        /// The conductor.
        Conductor = 5,
        /// The lyricist.
        Lyricist = 6,
        /// The producer.
        Producer = 7,
        /// The remixer.
        Remixer = 8,
        /// A performer, with the instrument or voice as the detail.
        Performer = 9,
        /// The arranger.
        Arranger = 10,
        /// An engineer.
        Engineer = 11,
        /// The mixing engineer.
        Mixer = 12,
        /// The DJ who mixed a DJ mix.
        DjMixer = 13,
    }
}

/// One credited name, exactly as tagged, with its role.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Credit {
    name: String,
    role: Role,
    detail: Option<String>,
    mbid: Option<Mbid>,
}

impl Credit {
    /// `name` credited as `role`, with a detail such as the instrument of a
    /// performer and the person's `MusicBrainz` identifier when tagged.
    ///
    /// # Errors
    ///
    /// [`CatalogError::BlankName`] for a name that is empty or only white
    /// space.
    pub fn new(
        name: String,
        role: Role,
        detail: Option<String>,
        mbid: Option<Mbid>,
    ) -> Result<Self, CatalogError> {
        if name.trim().is_empty() {
            return Err(CatalogError::BlankName);
        }
        Ok(Self {
            name,
            role,
            detail,
            mbid,
        })
    }

    /// The name, exactly as tagged.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// The detail, such as an instrument, when tagged.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// The person's `MusicBrainz` identifier, when tagged.
    #[must_use]
    pub const fn mbid(&self) -> Option<Mbid> {
        self.mbid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::untrusted::Untrusted;

    #[test]
    fn roles_have_these_codes() {
        let roles: Vec<(Role, u8)> = Role::ALL.iter().map(|r| (*r, r.code())).collect();
        assert_eq!(
            roles,
            [
                (Role::Artist, 1),
                (Role::AlbumArtist, 2),
                (Role::Featured, 3),
                (Role::Composer, 4),
                (Role::Conductor, 5),
                (Role::Lyricist, 6),
                (Role::Producer, 7),
                (Role::Remixer, 8),
                (Role::Performer, 9),
                (Role::Arranger, 10),
                (Role::Engineer, 11),
                (Role::Mixer, 12),
                (Role::DjMixer, 13),
            ]
        );
        let read: Vec<Option<Role>> = (0..=14).map(Role::from_code).collect();
        let mut expected = vec![None];
        expected.extend(Role::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn keeps_a_whole_credit() {
        let mbid = Mbid::parse(Untrusted::new("f81d4fae-7dec-11d0-a765-00a0c91e6bf6")).unwrap();
        let credit = Credit::new(
            String::from(" Jaco Pastorius "),
            Role::Performer,
            Some(String::from("fretless bass")),
            Some(mbid),
        )
        .unwrap();
        assert_eq!(
            (credit.name(), credit.role(), credit.detail(), credit.mbid()),
            (
                " Jaco Pastorius ",
                Role::Performer,
                Some("fretless bass"),
                Some(mbid)
            )
        );
    }

    #[test]
    fn keeps_a_credit_without_detail_or_identifier() {
        let credit =
            Credit::new(String::from("Nadia Boulanger"), Role::Conductor, None, None).unwrap();
        assert_eq!(
            (credit.name(), credit.role(), credit.detail(), credit.mbid()),
            ("Nadia Boulanger", Role::Conductor, None, None)
        );
    }

    #[test]
    fn refuses_an_empty_or_blank_name() {
        for name in ["", " ", "\t \n"] {
            assert_eq!(
                Credit::new(String::from(name), Role::Composer, None, None),
                Err(CatalogError::BlankName)
            );
        }
    }
}
