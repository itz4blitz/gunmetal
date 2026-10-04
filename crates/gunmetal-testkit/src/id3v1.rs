//! A builder for `ID3v1` and `ID3v1.1` tags: the 128 octets at the very end of
//! an MP3 file.
//!
//! The layout is the one id3.org gives for `ID3v1`: `TAG`, then a title, an
//! artist and an album of 30 octets each, a year of 4, a comment of 30 and
//! one genre octet. `ID3v1.1` takes the comment's last two octets for a zero
//! and a track number. Unused octets of a text field are padded, with zeros
//! or with spaces, as writers in the wild do both.

/// What fills the unused end of each text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Padding {
    /// Zero octets, as the specification shows.
    Zeros,
    /// Spaces, as some writers pad instead.
    Spaces,
}

/// An `ID3v1` tag to write. Every text field starts empty, there is no track
/// number, and the genre octet is zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id3v1 {
    title: Vec<u8>,
    artist: Vec<u8>,
    album: Vec<u8>,
    year: Vec<u8>,
    comment: Vec<u8>,
    track: Option<u8>,
    genre: u8,
    padding: Padding,
}

impl Default for Id3v1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Id3v1 {
    /// An empty tag padded with zeros.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: Vec::new(),
            artist: Vec::new(),
            album: Vec::new(),
            year: Vec::new(),
            comment: Vec::new(),
            track: None,
            genre: 0,
            padding: Padding::Zeros,
        }
    }

    /// Sets the title, at most 30 octets.
    #[must_use]
    pub fn title(mut self, text: &[u8]) -> Self {
        text.clone_into(&mut self.title);
        self
    }

    /// Sets the artist, at most 30 octets.
    #[must_use]
    pub fn artist(mut self, text: &[u8]) -> Self {
        text.clone_into(&mut self.artist);
        self
    }

    /// Sets the album, at most 30 octets.
    #[must_use]
    pub fn album(mut self, text: &[u8]) -> Self {
        text.clone_into(&mut self.album);
        self
    }

    /// Sets the year, at most 4 octets.
    #[must_use]
    pub fn year(mut self, text: &[u8]) -> Self {
        text.clone_into(&mut self.year);
        self
    }

    /// Sets the comment: at most 30 octets, or 28 with a track number.
    #[must_use]
    pub fn comment(mut self, text: &[u8]) -> Self {
        text.clone_into(&mut self.comment);
        self
    }

    /// Writes an ID3v1.1 track number: a zero, then `number`, in the
    /// comment's last two octets.
    #[must_use]
    pub const fn track(mut self, number: u8) -> Self {
        self.track = Some(number);
        self
    }

    /// Sets the genre octet.
    #[must_use]
    pub const fn genre(mut self, genre: u8) -> Self {
        self.genre = genre;
        self
    }

    /// Sets what pads the unused end of each text field.
    #[must_use]
    pub const fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// The 128 octets of the tag.
    ///
    /// # Panics
    ///
    /// Panics when a field is longer than its space.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut tag = b"TAG".to_vec();
        self.field(&mut tag, "title", &self.title, 30);
        self.field(&mut tag, "artist", &self.artist, 30);
        self.field(&mut tag, "album", &self.album, 30);
        self.field(&mut tag, "year", &self.year, 4);
        if let Some(number) = self.track {
            self.field(&mut tag, "comment", &self.comment, 28);
            tag.extend([0, number]);
        } else {
            self.field(&mut tag, "comment", &self.comment, 30);
        }
        tag.push(self.genre);
        tag
    }

    /// Appends `text` to `tag`, padded to `width` octets.
    fn field(&self, tag: &mut Vec<u8>, name: &str, text: &[u8], width: usize) {
        assert!(
            text.len() <= width,
            "the {name} takes at most {width} octets, not {}",
            text.len()
        );
        let pad = match self.padding {
            Padding::Zeros => 0,
            Padding::Spaces => b' ',
        };
        tag.extend(text);
        tag.resize(tag.len() + width - text.len(), pad);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tag id3.org's layout gives for these values, written out field
    /// by field: `TAG`, title, artist, album (30 octets each), year (4),
    /// comment (30) and genre (1).
    #[test]
    fn writes_the_layout_the_specification_gives() {
        let tag = Id3v1::new()
            .title(b"Title")
            .artist(b"Artist")
            .album(b"Album")
            .year(b"2003")
            .comment(b"Comment")
            .genre(17)
            .build();
        let expected = [
            &b"TAG"[..],
            b"Title",
            &[0; 25],
            b"Artist",
            &[0; 24],
            b"Album",
            &[0; 25],
            b"2003",
            b"Comment",
            &[0; 23],
            &[17],
        ]
        .concat();
        assert_eq!(tag, expected);
        assert_eq!(tag.len(), 128);
    }

    #[test]
    fn pads_with_spaces_when_asked() {
        let tag = Id3v1::new()
            .title(b"T")
            .artist(b"Ar")
            .album(b"Alb")
            .year(b"99")
            .comment(b"C")
            .padding(Padding::Spaces)
            .build();
        let expected = [
            &b"TAG"[..],
            b"T",
            &[b' '; 29],
            b"Ar",
            &[b' '; 28],
            b"Alb",
            &[b' '; 27],
            b"99  ",
            b"C",
            &[b' '; 29],
            &[0],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    /// ID3v1.1: "If the 29th byte of the comment is 0 (null), then the
    /// 30th byte is the track number."
    #[test]
    fn writes_a_track_number_in_the_last_two_octets_of_the_comment() {
        let tag = Id3v1::new()
            .comment(b"Comment")
            .track(7)
            .padding(Padding::Spaces)
            .build();
        let expected = [
            &b"TAG"[..],
            &[b' '; 94],
            b"Comment",
            &[b' '; 21],
            &[0, 7],
            &[0],
        ]
        .concat();
        assert_eq!(tag, expected);
    }

    #[test]
    fn fills_every_field_to_its_end() {
        let tag = Id3v1::new()
            .title(&[b'a'; 30])
            .artist(&[b'b'; 30])
            .album(&[b'c'; 30])
            .year(b"1999")
            .comment(&[b'd'; 30])
            .genre(255)
            .build();
        let expected = [
            &b"TAG"[..],
            &[b'a'; 30],
            &[b'b'; 30],
            &[b'c'; 30],
            b"1999",
            &[b'd'; 30],
            &[255],
        ]
        .concat();
        assert_eq!(tag, expected);
        let numbered = Id3v1::new().comment(&[b'e'; 28]).track(255).build();
        assert_eq!(&numbered[97..], [&[b'e'; 28][..], &[0, 255, 0]].concat());
    }

    #[test]
    fn starts_empty_with_zero_padding() {
        assert_eq!(Id3v1::default(), Id3v1::new());
        assert_eq!(Id3v1::new().build(), [&b"TAG"[..], &[0; 125]].concat());
    }

    #[test]
    #[should_panic(expected = "the title takes at most 30 octets, not 31")]
    fn refuses_a_title_longer_than_its_field() {
        let _ = Id3v1::new().title(&[b'a'; 31]).build();
    }

    #[test]
    #[should_panic(expected = "the artist takes at most 30 octets, not 31")]
    fn refuses_an_artist_longer_than_its_field() {
        let _ = Id3v1::new().artist(&[b'a'; 31]).build();
    }

    #[test]
    #[should_panic(expected = "the album takes at most 30 octets, not 31")]
    fn refuses_an_album_longer_than_its_field() {
        let _ = Id3v1::new().album(&[b'a'; 31]).build();
    }

    #[test]
    #[should_panic(expected = "the year takes at most 4 octets, not 5")]
    fn refuses_a_year_longer_than_its_field() {
        let _ = Id3v1::new().year(b"20031").build();
    }

    #[test]
    #[should_panic(expected = "the comment takes at most 30 octets, not 31")]
    fn refuses_a_comment_longer_than_its_field() {
        let _ = Id3v1::new().comment(&[b'a'; 31]).build();
    }

    #[test]
    #[should_panic(expected = "the comment takes at most 28 octets, not 29")]
    fn refuses_a_comment_that_leaves_no_room_for_the_track() {
        let _ = Id3v1::new().comment(&[b'a'; 29]).track(1).build();
    }
}
