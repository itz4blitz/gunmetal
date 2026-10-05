//! Builders for the search tests, written without the code they test.

use super::doc::{DocKind, DocRef, SearchDoc};

/// The identifier text of document `number` of kind `kind`: the kind's
/// prefix, then `number` in decimal, padded to 26 symbols.
pub(super) fn id_text(kind: DocKind, number: u32) -> String {
    let prefix = match kind {
        DocKind::Artist => "art",
        DocKind::Album => "alb",
        DocKind::Track => "trk",
        DocKind::Playlist => "pls",
    };
    format!("{prefix}_{number:026}")
}

/// The reference to document `number` of kind `kind`.
pub(super) fn doc_ref(kind: DocKind, number: u32) -> DocRef {
    DocRef::from_text(&id_text(kind, number)).unwrap()
}

/// A document with a title and nothing else.
pub(super) fn titled(kind: DocKind, number: u32, title: &str) -> SearchDoc {
    SearchDoc {
        doc: doc_ref(kind, number),
        title: title.to_owned(),
        artist: String::new(),
        album: String::new(),
        credits: Vec::new(),
        genres: Vec::new(),
        labels: Vec::new(),
        plays: 0,
    }
}

/// `words`, each as an owned string.
pub(super) fn owned(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}
