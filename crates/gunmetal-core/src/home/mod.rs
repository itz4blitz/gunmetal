//! Home rows: what a person's start page shows, evaluated on the device
//! from the synced library and that person's own activity, so drawing Home
//! needs no server call (DIS-001, DIS-002, DIS-004).
//!
//! R1 ships four rows, in the order of [`DEFAULT_LAYOUT`]: Continue
//! listening, Recently played, Recently added and Loved songs.
//! [`evaluate_row`] turns a [`RowSpec`] into a [`Row`]: its cards, each an
//! item's public identifier, the reason the row shows them (API-HOME-04),
//! or the empty state to draw when it has none.
//!
//! Dismissing a card is WP-141 (R1.1), a person's top tracks by an artist
//! WP-147 (R1.1), arranging rows and pins WP-154 (R1.2), and "because you
//! played" and rule-backed rows R1.3.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod evaluate;
#[cfg(test)]
mod fixtures;
pub mod input;
#[cfg(test)]
mod properties;
pub mod row;

pub use evaluate::{ARRIVAL_SPAN_MS, CONTINUE_WINDOW_MS, evaluate_row};
pub use input::{LibraryTrack, LibraryView, Listening, ListeningSource, MyEvents};
pub use row::{
    Card, DEFAULT_LAYOUT, DEFAULT_ROW_LIMIT, EmptyState, Reason, Row, RowContent, RowSource,
    RowSpec,
};
