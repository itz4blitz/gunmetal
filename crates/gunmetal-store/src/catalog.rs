//! The catalogue's tables in the cache (WP-067): the synced-library
//! records, each track's credits and the extras kept beside a track.
//!
//! [`apply_batch`] is the only writer. It touches a row only when the batch
//! holds something different from what is stored, and it returns one
//! [`CatalogChange`] for each record that really changed, so applying the
//! same batch again writes nothing and returns nothing (LIB-016). The
//! caller writes the returned changes to the change log in the same
//! transaction (WP-102).
//!
//! Every reader takes the [`Permit`] the policy returned and filters in SQL
//! by the libraries it holds (SEC-IAM-070, SEC-API-010). There is no reader
//! without one. A row in a library the permit does not hold is never
//! returned, so a hidden record answers exactly as one that does not exist
//! (SEC-API-011). Every row type implements [`HasLibrary`], so the
//! authorisation layer can check a row again (WP-065).
//!
//! Every statement is static text with bound values (SEC-API-066), so a
//! title or a genre full of SQL is stored and read back as it is.
//!
//! The record tables hold the core's records field by field, under the
//! names of the core's field table, so a change to a record's shape changes
//! this part's SQL and the cache rebuilds. What a record holds is decided
//! by the packages that derive it (WP-075, WP-076). The store does not read
//! an extra at all: it keeps the octets the owning package encoded.
//!
//! The release-group table arrives with the release-group record in R1.1
//! (WP-146), and the reader of a track's credits arrives with the credits
//! panel that needs it.

use gunmetal_core::authz::{HasLibrary, Permit};
use gunmetal_core::catalog::{
    AlbumId, AlbumRecord, ArtistId, ArtistRecord, CatalogChange, Credit, LibraryId, RecordId,
    TrackId, TrackRecord,
};
use gunmetal_core::id::PublicId;
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_fs::sqlite::{DbError, Query, Value};

use crate::readers::Reader;
use crate::store::StoreError;
use crate::writer::Transaction;

/// A column of the catalogue. Every one is library data: what the library
/// holds, never what a person did with it.
const fn column(table: &'static str, name: &'static str) -> Column {
    Column {
        table,
        name,
        class: DataClass::Library,
    }
}

/// The catalogue's schema part, which the server registers when it opens
/// the cache.
pub const SCHEMA: SchemaPart = SchemaPart {
    name: "catalog",
    sql: include_str!("catalog.sql"),
    columns: &[
        column("tracks", "id"),
        column("tracks", "kind"),
        column("tracks", "library"),
        column("tracks", "title"),
        column("tracks", "title_sort"),
        column("tracks", "artist_credit"),
        column("tracks", "artists"),
        column("tracks", "album"),
        column("tracks", "track_number"),
        column("tracks", "track_total"),
        column("tracks", "disc_number"),
        column("tracks", "disc_total"),
        column("tracks", "disc_subtitle"),
        column("tracks", "date"),
        column("tracks", "original_date"),
        column("tracks", "genres"),
        column("tracks", "moods"),
        column("tracks", "styles"),
        column("tracks", "labels"),
        column("tracks", "grouping"),
        column("tracks", "advisory"),
        column("tracks", "isrc"),
        column("tracks", "recording_mbid"),
        column("tracks", "codec"),
        column("tracks", "container"),
        column("tracks", "sample_rate"),
        column("tracks", "bit_depth"),
        column("tracks", "channels"),
        column("tracks", "bitrate"),
        column("tracks", "duration"),
        column("tracks", "track_gain_scale"),
        column("tracks", "track_gain"),
        column("tracks", "track_peak"),
        column("tracks", "album_gain_scale"),
        column("tracks", "album_gain"),
        column("tracks", "album_peak"),
        column("tracks", "trim_delay"),
        column("tracks", "trim_padding"),
        column("tracks", "lyrics_timing"),
        column("tracks", "availability"),
        column("tracks", "added"),
        column("tracks", "lyrics_origin"),
        column("albums", "id"),
        column("albums", "library"),
        column("albums", "title"),
        column("albums", "title_sort"),
        column("albums", "artist_credit"),
        column("albums", "artists"),
        column("albums", "date"),
        column("albums", "original_date"),
        column("albums", "primary_type"),
        column("albums", "secondary_types"),
        column("albums", "compilation"),
        column("albums", "genres"),
        column("albums", "labels"),
        column("albums", "track_count"),
        column("albums", "disc_count"),
        column("albums", "duration"),
        column("albums", "has_artwork"),
        column("albums", "release_mbid"),
        column("albums", "release_group_mbid"),
        column("albums", "added"),
        column("artists", "id"),
        column("artists", "library"),
        column("artists", "name"),
        column("artists", "name_sort"),
        column("artists", "mbid"),
        column("artists", "album_count"),
        column("artists", "track_count"),
        column("artists", "genres"),
        column("artists", "has_artwork"),
        column("credits", "track"),
        column("credits", "seq"),
        column("credits", "name"),
        column("credits", "role"),
        column("credits", "detail"),
        column("credits", "mbid"),
        column("extras", "track"),
        column("extras", "kind"),
        column("extras", "body"),
    ],
};

/// What the store keeps beside a track's record. Each is the octets the
/// package that owns it encoded; the store compares and returns them and
/// never reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraKind {
    /// What the scan recorded about the track's file, such as its path
    /// beneath the library root and its fingerprint.
    File,
    /// The track's artwork record.
    Artwork,
    /// The track's lyrics.
    Lyrics,
    /// The file's seek index, which a device fetches when the track enters
    /// its queue.
    SeekIndex,
    /// The file's frame index, which the packaging route checks a segment
    /// number against.
    FrameIndex,
}

impl ExtraKind {
    /// The code the kind is stored under, which never changes and is never
    /// reused.
    const fn code(self) -> i64 {
        match self {
            Self::File => 1,
            Self::Artwork => 2,
            Self::Lyrics => 3,
            Self::SeekIndex => 4,
            Self::FrameIndex => 5,
        }
    }
}

/// The credits of one track, whole and in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackCredits {
    /// The track.
    pub track: TrackId,
    /// Every credit the track has. An empty list removes them all.
    pub credits: Vec<Credit>,
}

/// One extra of one track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackExtra {
    /// The track.
    pub track: TrackId,
    /// Which extra.
    pub kind: ExtraKind,
    /// The octets to keep, or `None` to remove the extra.
    pub body: Option<Vec<u8>>,
}

/// What one commit of a scan writes to the catalogue.
///
/// [`apply_batch`] applies the lists in the order of the fields. A record
/// in a list replaces the stored record with the same identifier. The
/// caller gives a track's record with or before its credits and extras: the
/// store keeps credits and extras for a track it has no record of, but no
/// reader returns them until the record arrives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CatalogBatch {
    /// Artist records to add or change.
    pub artists: Vec<ArtistRecord>,
    /// Album records to add or change.
    pub albums: Vec<AlbumRecord>,
    /// Track records to add or change.
    pub tracks: Vec<TrackRecord>,
    /// Tracks whose credits are replaced. A track not named keeps the
    /// credits it has.
    pub credits: Vec<TrackCredits>,
    /// Extras to store or remove. An extra not named stays as it is.
    pub extras: Vec<TrackExtra>,
    /// Records to remove. Removing a track removes its credits and its
    /// extras with it, and nothing in any other table.
    pub removed: Vec<RecordId>,
}

/// A track a reader returned.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackRow(pub TrackRecord);

/// An album a reader returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumRow(pub AlbumRecord);

/// An artist a reader returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistRow(pub ArtistRecord);

/// An extra a reader returned, with the library of its track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraRow {
    /// The track the extra belongs to.
    pub track: TrackId,
    /// The library the track is in.
    pub library: LibraryId,
    /// Which extra.
    pub kind: ExtraKind,
    /// The octets the owning package stored.
    pub body: Vec<u8>,
}

impl HasLibrary for TrackRow {
    fn library(&self) -> PublicId {
        self.0.library.get()
    }
}

impl HasLibrary for AlbumRow {
    fn library(&self) -> PublicId {
        self.0.library.get()
    }
}

impl HasLibrary for ArtistRow {
    fn library(&self) -> PublicId {
        self.0.library.get()
    }
}

impl HasLibrary for ExtraRow {
    fn library(&self) -> PublicId {
        self.library.get()
    }
}

/// Why a reader failed.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    /// SQLite failed or refused the statement.
    Db(DbError),
    /// A stored row is not one this module wrote: a cell is missing, of the
    /// wrong type or outside its range. The read returns nothing rather
    /// than the rows it could read, and the cache should be rebuilt.
    Damaged,
}

/// A page of tracks in identifier order. The first value is the permit's
/// libraries, as [`visible`] writes them.
const TRACK_PAGE: &str = "SELECT * FROM tracks \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id > ?2 \
     ORDER BY id LIMIT ?3";

const TRACK_ONE: &str = "SELECT * FROM tracks \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id = ?2";

const ALBUM_PAGE: &str = "SELECT * FROM albums \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id > ?2 \
     ORDER BY id LIMIT ?3";

const ALBUM_ONE: &str = "SELECT * FROM albums \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id = ?2";

const ARTIST_PAGE: &str = "SELECT * FROM artists \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id > ?2 \
     ORDER BY id LIMIT ?3";

const ARTIST_ONE: &str = "SELECT * FROM artists \
     WHERE (?1 IS NULL OR library IN (SELECT value FROM json_each(?1))) AND id = ?2";

/// One extra with the library of its track, which is where an extra's
/// library comes from.
const EXTRA_ONE: &str = "SELECT extras.track, tracks.library, extras.body \
     FROM extras JOIN tracks ON tracks.id = extras.track \
     WHERE (?1 IS NULL OR tracks.library IN (SELECT value FROM json_each(?1))) \
     AND extras.track = ?2 AND extras.kind = ?3";

/// Applies `batch` inside the caller's transaction and returns what really
/// changed.
///
/// A row is written only when the batch holds something different from
/// what is stored. The changes come back in the order they were made:
/// artists, albums, then tracks, each as an upsert, then removals. A track
/// whose record, credits or extras changed is one upsert of that track,
/// however many of them changed. A record named in `removed` that was not
/// stored is no change.
///
/// # Errors
///
/// Returns [`StoreError::Db`] when SQLite fails or refuses a statement. The
/// caller's transaction then rolls back, so nothing of the batch is kept.
pub fn apply_batch(
    _tx: &Transaction<'_>,
    _batch: &CatalogBatch,
) -> Result<Vec<CatalogChange>, StoreError> {
    Ok(Vec::new())
}

/// The statement `text` with `cells` bound to its parameters in order.
fn bound(text: &'static str, cells: impl IntoIterator<Item = Value>) -> Query {
    cells.into_iter().fold(Query::new(text), Query::bind)
}

fn id_cell(id: PublicId) -> Value {
    Value::Text(id.to_string())
}

/// The libraries `permit` holds, as the one value every reader's statement
/// filters by: `NULL` when it holds every library, and otherwise a JSON
/// array of their identifiers, which the statement reads with `json_each`.
/// An identifier holds only digits, lower-case letters and an underscore,
/// so it needs no escaping.
fn visible(permit: &Permit) -> Value {
    permit
        .libraries()
        .restriction()
        .map_or(Value::Null, |libraries| {
            let quoted: Vec<String> = libraries
                .iter()
                .map(|library| ["\"", library.to_string().as_str(), "\""].concat())
                .collect();
            Value::Text(["[", quoted.join(",").as_str(), "]"].concat())
        })
}

/// The statement `text` for up to `limit` rows `permit` may see whose
/// identifiers come after `after`. Every identifier comes after the empty
/// text.
fn page(text: &'static str, permit: &Permit, after: Option<PublicId>, limit: u32) -> Query {
    bound(
        text,
        [
            visible(permit),
            after.map_or_else(|| Value::Text(String::new()), id_cell),
            Value::Integer(i64::from(limit)),
        ],
    )
}

/// The statement `text` for the row with identifier `id`, if `permit` may
/// see it.
fn one(text: &'static str, permit: &Permit, id: PublicId) -> Query {
    bound(text, [visible(permit), id_cell(id)])
}

/// Runs `query` and reads every row with `read`. One row that cannot be
/// read fails the whole read.
fn rows<T>(
    reader: &Reader<'_>,
    query: &Query,
    read: impl Fn(&[Value]) -> Option<T>,
) -> Result<Vec<T>, ReadError> {
    reader
        .query(query)
        .map_err(ReadError::Db)
        .and_then(|found| {
            found
                .iter()
                .map(|row| read(&row.0))
                .collect::<Option<Vec<T>>>()
                .ok_or(ReadError::Damaged)
        })
}

/// The row of a read by identifier, which finds at most one.
fn only<T>(found: Vec<T>) -> Option<T> {
    found.into_iter().next()
}

/// Up to `limit` tracks in the libraries `permit` holds, in identifier
/// order, starting after `after`, or from the first when `after` is
/// `None`.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when a stored track cannot be read.
pub fn tracks(
    reader: &Reader<'_>,
    permit: &Permit,
    after: Option<TrackId>,
    limit: u32,
) -> Result<Vec<TrackRow>, ReadError> {
    let query = page(TRACK_PAGE, permit, after.map(TrackId::get), limit);
    rows(reader, &query, track_row)
}

/// The track `id`, if it is stored and in a library `permit` holds. A
/// track in any other library answers as one that is not stored.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when the stored track cannot be read.
pub fn track(
    reader: &Reader<'_>,
    permit: &Permit,
    id: TrackId,
) -> Result<Option<TrackRow>, ReadError> {
    rows(reader, &one(TRACK_ONE, permit, id.get()), track_row).map(only)
}

/// Up to `limit` albums in the libraries `permit` holds, in identifier
/// order, starting after `after`, or from the first when `after` is
/// `None`.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when a stored album cannot be read.
pub fn albums(
    reader: &Reader<'_>,
    permit: &Permit,
    after: Option<AlbumId>,
    limit: u32,
) -> Result<Vec<AlbumRow>, ReadError> {
    let query = page(ALBUM_PAGE, permit, after.map(AlbumId::get), limit);
    rows(reader, &query, album_row)
}

/// The album `id`, if it is stored and in a library `permit` holds. An
/// album in any other library answers as one that is not stored.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when the stored album cannot be read.
pub fn album(
    reader: &Reader<'_>,
    permit: &Permit,
    id: AlbumId,
) -> Result<Option<AlbumRow>, ReadError> {
    rows(reader, &one(ALBUM_ONE, permit, id.get()), album_row).map(only)
}

/// Up to `limit` artists in the libraries `permit` holds, in identifier
/// order, starting after `after`, or from the first when `after` is
/// `None`.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when a stored artist cannot be read.
pub fn artists(
    reader: &Reader<'_>,
    permit: &Permit,
    after: Option<ArtistId>,
    limit: u32,
) -> Result<Vec<ArtistRow>, ReadError> {
    let query = page(ARTIST_PAGE, permit, after.map(ArtistId::get), limit);
    rows(reader, &query, artist_row)
}

/// The artist `id`, if it is stored and in a library `permit` holds. An
/// artist in any other library answers as one that is not stored.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when the stored artist cannot be read.
pub fn artist(
    reader: &Reader<'_>,
    permit: &Permit,
    id: ArtistId,
) -> Result<Option<ArtistRow>, ReadError> {
    rows(reader, &one(ARTIST_ONE, permit, id.get()), artist_row).map(only)
}

/// The extra of kind `kind` kept beside the track `track`, if there is one
/// and the track is in a library `permit` holds. The extra of a track in
/// any other library, or of a track with no record, answers as one that is
/// not stored.
///
/// # Errors
///
/// Returns [`ReadError::Db`] when SQLite fails and [`ReadError::Damaged`]
/// when the stored row cannot be read.
pub fn extra(
    reader: &Reader<'_>,
    permit: &Permit,
    track: TrackId,
    kind: ExtraKind,
) -> Result<Option<ExtraRow>, ReadError> {
    let query = bound(
        EXTRA_ONE,
        [
            visible(permit),
            id_cell(track.get()),
            Value::Integer(kind.code()),
        ],
    );
    rows(reader, &query, |cells| extra_row(cells, kind)).map(only)
}

/// Reads a track from the cells of its row.
fn track_row(_cells: &[Value]) -> Option<TrackRow> {
    None
}

/// Reads an album from the cells of its row.
fn album_row(_cells: &[Value]) -> Option<AlbumRow> {
    None
}

/// Reads an artist from the cells of its row.
fn artist_row(_cells: &[Value]) -> Option<ArtistRow> {
    None
}

/// Reads an extra of kind `kind` from its track, the track's library and
/// its octets.
fn extra_row(_cells: &[Value], _kind: ExtraKind) -> Option<ExtraRow> {
    None
}

/// What code outside this crate must not be able to do with the catalogue.
#[cfg(doctest)]
mod compile_fail {
    /// Control: a reader is called with the permit the policy returned.
    ///
    /// ```
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_core::catalog::TrackId;
    /// use gunmetal_store::catalog::{ReadError, TrackRow, track};
    /// use gunmetal_store::readers::Reader;
    ///
    /// fn open(
    ///     reader: &Reader<'_>,
    ///     permit: &Permit,
    ///     id: TrackId,
    /// ) -> Result<Option<TrackRow>, ReadError> {
    ///     track(reader, permit, id)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-IAM-070
    ///
    /// No reader returns a row without a permit to filter by.
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_core::catalog::TrackId;
    /// use gunmetal_store::catalog::{ReadError, TrackRow, track};
    /// use gunmetal_store::readers::Reader;
    ///
    /// fn open(reader: &Reader<'_>, id: TrackId) -> Result<Option<TrackRow>, ReadError> {
    ///     track(reader, id)
    /// }
    /// ```
    struct NoReaderWithoutAPermit;
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Debug;
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    use std::thread;
    use std::time::{Duration as Wait, Instant};

    use gunmetal_core::authz::{
        Action, Capability, CapabilitySet, Context as Request, DeviceClass, Elevation, HasLibrary,
        Network, Permit, PrincipalFacts, PrincipalKind, Reach, RemoteAdmin, ResourceFacts,
        UserVerification, decide,
    };
    use gunmetal_core::catalog::{
        Advisory, AlbumId, AlbumRecord, ArtistId, ArtistRecord, AudioFormat, Availability, Bitrate,
        CatalogChange, CatalogField, ChangeOp, Codec, Container, Credit, Gain, GainScale, GainTags,
        ItemKind, LibraryId, LyricsOrigin, LyricsSource, LyricsTiming, PrimaryType, RecordId,
        RecordKind, ReleaseType, Role, SecondaryType, TechInfo, TrackId, TrackPosition,
        TrackRecord, Trim,
    };
    use gunmetal_core::client_context::PathClass;
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::schema::{Column, DataClass, SchemaPart};
    use gunmetal_core::time::Timestamp;
    use gunmetal_core::untrusted::Untrusted;
    use gunmetal_core::values::{
        BitDepth, Channels, Duration, GainDb, Isrc, Mbid, PartialDate, PeakRatio, SampleRate,
    };
    use gunmetal_fs::dataroot::{DataRoot, Policy};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::sqlite::{DbError, Query, Row, Value};
    use gunmetal_testkit::tempdir::TempDir;

    use crate::readers::Reader;
    use crate::store::{Generation, Store, StoreError};

    /// How long a test waits for the store before it fails. A hang is a
    /// failure, never a timeout.
    const BOUND: Wait = Wait::from_secs(10);

    const ALL_TRACKS: &str = "SELECT * FROM tracks ORDER BY id";
    const ALL_ALBUMS: &str = "SELECT * FROM albums ORDER BY id";
    const ALL_ARTISTS: &str = "SELECT * FROM artists ORDER BY id";
    const ALL_CREDITS: &str = "SELECT * FROM credits ORDER BY track, seq";
    const ALL_EXTRAS: &str = "SELECT * FROM extras ORDER BY track, kind";

    /// How many rows the writer's connection has inserted, changed or
    /// deleted since it was opened.
    const WRITES: &str = "SELECT total_changes()";

    const TRACK_1: &str = "trk_00000000000000000000000001";
    const TRACK_2: &str = "trk_00000000000000000000000002";
    const ALBUM_1: &str = "alb_00000000000000000000000001";
    const ALBUM_2: &str = "alb_00000000000000000000000002";
    const ARTIST_1: &str = "art_00000000000000000000000001";
    const ARTIST_2: &str = "art_00000000000000000000000002";
    const LIBRARY_1: &str = "lib_00000000000000000000000001";
    const LIBRARY_2: &str = "lib_00000000000000000000000002";

    /// A stand-in for a projection of the curation log: a table of another
    /// module's that names catalogue records.
    const OVERRIDES: SchemaPart = SchemaPart {
        name: "curation",
        sql: "CREATE TABLE curation_overrides (record TEXT NOT NULL, verdict TEXT NOT NULL) STRICT;",
        columns: &[
            Column {
                table: "curation_overrides",
                name: "record",
                class: DataClass::Activity,
            },
            Column {
                table: "curation_overrides",
                name: "verdict",
                class: DataClass::Activity,
            },
        ],
    };

    /// Texts a tag might hold to break out of a statement.
    const HOSTILE: [&str; 10] = [
        "'); DROP TABLE tracks; --",
        "\" OR 1=1 --",
        "' OR '1'='1",
        "%",
        "_",
        "admin%",
        "Intro'; DELETE FROM tracks WHERE '' = '",
        "/* */ ; ATTACH DATABASE '/tmp/x' AS x; --",
        "a\0b",
        "?1 ?2 :name @name $name",
    ];

    /// The cache in a temporary data directory.
    struct Cache {
        store: Store,
        // Dropped after the store, in this order.
        _root: DataRoot,
        _dir: TempDir,
    }

    /// A new cache built from `parts`.
    fn open(parts: &[SchemaPart]) -> Cache {
        let dir = TempDir::new("catalog").expect("a temporary directory");
        let host = HostFacts::probe(dir.path()).expect("probe the host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("the data root opens")
            .root;
        let store = Store::open(&root, parts, Generation([7; 16]))
            .expect("the store opens")
            .store;
        Cache {
            store,
            _root: root,
            _dir: dir,
        }
    }

    /// Drives `future` on this thread until it is ready, failing the test
    /// once [`BOUND`] has passed.
    fn wait<F: Future>(future: F) -> F::Output {
        let deadline = Instant::now() + BOUND;
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        loop {
            thread::yield_now();
            // The store must answer in time.
            assert!(Instant::now() < deadline);
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    /// Applies `batch` in one write.
    fn apply(cache: &Cache, batch: &CatalogBatch) -> Result<Vec<CatalogChange>, StoreError> {
        let batch = batch.clone();
        wait(cache.store.write(move |tx| apply_batch(tx, &batch)))
    }

    /// Runs one statement of the test's own on the writer and returns how
    /// many rows it changed.
    fn run(cache: &Cache, sql: &'static str) -> Result<usize, StoreError> {
        wait(
            cache
                .store
                .write(move |tx| tx.execute(&Query::new(sql)).map_err(StoreError::from)),
        )
    }

    /// Every row a statement of the test's own returns, read on the writer.
    fn select(cache: &Cache, sql: &'static str) -> Vec<Row> {
        wait(
            cache
                .store
                .write(move |tx| tx.query(&Query::new(sql)).map_err(StoreError::from)),
        )
        .expect("the statement runs")
    }

    /// Every row of every table of the catalogue.
    fn everything(cache: &Cache) -> [Vec<Row>; 5] {
        [ALL_TRACKS, ALL_ALBUMS, ALL_ARTISTS, ALL_CREDITS, ALL_EXTRAS].map(|sql| select(cache, sql))
    }

    /// Runs `work` on a pooled reader.
    fn read<R: Send + 'static>(
        cache: &Cache,
        work: impl FnOnce(&Reader<'_>) -> R + Send + 'static,
    ) -> R {
        wait(cache.store.read(work)).expect("the reader answers")
    }

    /// The permit the policy gives a principal of kind `kind` holding
    /// `capabilities` and granted the libraries numbered `libraries`, for
    /// browsing.
    fn principal(kind: PrincipalKind, capabilities: CapabilitySet, libraries: &[u8]) -> Permit {
        let facts = PrincipalFacts {
            kind,
            account: None,
            profile: None,
            capabilities,
            libraries: libraries.iter().map(|n| library(*n).get()).collect(),
            device: DeviceClass::Personal,
            elevation: Elevation::Ordinary,
            verification: UserVerification::Stale,
            reach: Reach::Anywhere,
            scope: None,
        };
        let request = Request {
            path: PathClass::Loopback,
            network: Network::Same,
            remote_admin: RemoteAdmin::Allowed,
        };
        decide(
            &facts,
            Action::BrowseLibrary,
            &ResourceFacts::Server,
            &request,
        )
        .expect("the policy allows browsing")
    }

    /// The permit of a member granted the libraries numbered `libraries`.
    fn member(libraries: &[u8]) -> Permit {
        principal(
            PrincipalKind::Member,
            CapabilitySet::of(&[Capability::LibraryRead]),
            libraries,
        )
    }

    /// The permit of the owner, who holds every library.
    fn owner() -> Permit {
        principal(PrincipalKind::Owner, CapabilitySet::EVERY, &[])
    }

    /// The identifier of kind `kind` whose symbols are `n` in decimal,
    /// padded with zeros.
    fn public(prefix: &str, kind: IdKind, n: u8) -> PublicId {
        PublicId::parse(&format!("{prefix}_{n:026}"), kind).expect("an identifier")
    }

    fn library(n: u8) -> LibraryId {
        LibraryId::new(public("lib", IdKind::Library, n)).expect("a library")
    }

    fn track_id(n: u8) -> TrackId {
        TrackId::new(public("trk", IdKind::Track, n)).expect("a track")
    }

    fn album_id(n: u8) -> AlbumId {
        AlbumId::new(public("alb", IdKind::Album, n)).expect("an album")
    }

    fn artist_id(n: u8) -> ArtistId {
        ArtistId::new(public("art", IdKind::Artist, n)).expect("an artist")
    }

    fn uuid(text: &str) -> Mbid {
        Mbid::parse(Untrusted::new(text)).expect("a UUID")
    }

    fn date(year: u16, month: Option<u8>, day: Option<u8>) -> PartialDate {
        PartialDate::new(year, month, day).expect("a date")
    }

    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).expect("a time")
    }

    fn lasting(millis: u64) -> Duration {
        Duration::from_millis(millis).expect("a duration")
    }

    fn owned(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    fn t(text: &str) -> Value {
        Value::Text(text.to_owned())
    }

    fn i(value: i64) -> Value {
        Value::Integer(value)
    }

    fn b(octets: &[u8]) -> Value {
        Value::Blob(octets.to_vec())
    }

    fn changed(record: RecordId) -> CatalogChange {
        CatalogChange {
            record,
            op: ChangeOp::Upsert,
        }
    }

    fn gone(record: RecordId) -> CatalogChange {
        CatalogChange {
            record,
            op: ChangeOp::Removal,
        }
    }

    /// A track with every field filled in, in library 1.
    fn full_track() -> TrackRecord {
        let format = AudioFormat {
            sample_rate: Some(SampleRate::new(44_100).expect("a sample rate")),
            bit_depth: Some(BitDepth::new(16).expect("a bit depth")),
            channels: Some(Channels::new(2).expect("a channel count")),
            bitrate: Some(Bitrate::new(701_000).expect("a bitrate")),
            duration: Some(lasting(337_000)),
        };
        TrackRecord {
            id: track_id(1),
            kind: ItemKind::Track,
            library: library(1),
            title: "First Light".to_owned(),
            title_sort: Some("first light".to_owned()),
            artist_credit: "The Examples feat. A. Guest".to_owned(),
            artists: vec![artist_id(1), artist_id(2)],
            album: Some(album_id(1)),
            position: TrackPosition::new(Some(3), Some(12), Some(1), Some(2)).expect("a position"),
            disc_subtitle: Some("Side A".to_owned()),
            date: Some(date(1977, Some(2), Some(4))),
            original_date: Some(date(1976, Some(11), None)),
            genres: owned(&["Jazz", "Modal"]),
            moods: owned(&["Calm"]),
            styles: owned(&["Cool"]),
            labels: owned(&["Example Records"]),
            grouping: owned(&["Sessions"]),
            advisory: Some(Advisory::Clean),
            isrc: vec![Isrc::parse(Untrusted::new("AAXXX2600001")).expect("a code")],
            recording_mbid: Some(uuid("11111111-2222-3333-4444-555555555555")),
            tech: TechInfo::new(Codec::Flac, Container::Flac, format).expect("a format"),
            gain: GainTags {
                track: Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain: GainDb::new(-6.5).expect("a gain"),
                    peak: Some(PeakRatio::new(0.5).expect("a peak")),
                }),
                album: Some(Gain {
                    scale: GainScale::R128,
                    gain: GainDb::new(-7.25).expect("a gain"),
                    peak: None,
                }),
            },
            trim: Some(Trim {
                delay: 576,
                padding: 1105,
            }),
            lyrics: Some(
                LyricsSource::new(LyricsOrigin::Id3Synced, LyricsTiming::Line).expect("a source"),
            ),
            availability: Availability::Playable,
            added: at(1_700_000_000_000),
        }
    }

    /// The row of [`full_track`], written out by hand. The floats are their
    /// IEEE 754 bit patterns, computed with Python's `struct`.
    fn full_track_cells() -> Vec<Value> {
        vec![
            t(TRACK_1),
            i(1),
            t(LIBRARY_1),
            t("First Light"),
            t("first light"),
            t("The Examples feat. A. Guest"),
            b(b"\0\0\0\0\0\0\0\x1eart_00000000000000000000000001\
                \0\0\0\0\0\0\0\x1eart_00000000000000000000000002"),
            t(ALBUM_1),
            i(3),
            i(12),
            i(1),
            i(2),
            t("Side A"),
            i(19_770_204),
            i(19_761_100),
            b(b"\0\0\0\0\0\0\0\x04Jazz\0\0\0\0\0\0\0\x05Modal"),
            b(b"\0\0\0\0\0\0\0\x04Calm"),
            b(b"\0\0\0\0\0\0\0\x04Cool"),
            b(b"\0\0\0\0\0\0\0\x0fExample Records"),
            b(b"\0\0\0\0\0\0\0\x08Sessions"),
            i(2),
            b(b"\0\0\0\0\0\0\0\x0cAAXXX2600001"),
            t("11111111-2222-3333-4444-555555555555"),
            i(3),
            i(2),
            i(44_100),
            i(16),
            i(2),
            i(701_000),
            i(337_000),
            i(1),
            i(3_234_856_960),
            i(1_056_964_608),
            i(2),
            i(3_236_429_824),
            Value::Null,
            i(576),
            i(1105),
            i(2),
            i(1),
            i(1_700_000_000_000),
            i(2),
        ]
    }

    /// A track with nothing optional, in library 2.
    fn bare_track() -> TrackRecord {
        TrackRecord {
            id: track_id(2),
            kind: ItemKind::Audiobook,
            library: library(2),
            title: "Untitled".to_owned(),
            title_sort: None,
            artist_credit: String::new(),
            artists: Vec::new(),
            album: None,
            position: TrackPosition::default(),
            disc_subtitle: None,
            date: None,
            original_date: None,
            genres: Vec::new(),
            moods: Vec::new(),
            styles: Vec::new(),
            labels: Vec::new(),
            grouping: Vec::new(),
            advisory: None,
            isrc: Vec::new(),
            recording_mbid: None,
            tech: TechInfo::new(Codec::Mp3, Container::Mpeg, AudioFormat::default())
                .expect("a format"),
            gain: GainTags::default(),
            trim: None,
            lyrics: None,
            availability: Availability::Missing,
            added: at(-1),
        }
    }

    /// The row of [`bare_track`], written out by hand.
    fn bare_track_cells() -> Vec<Value> {
        let mut cells = vec![
            t(TRACK_2),
            i(2),
            t(LIBRARY_2),
            t("Untitled"),
            Value::Null,
            t(""),
            b(b""),
        ];
        cells.extend(vec![Value::Null; 8]);
        cells.extend(vec![b(b""); 5]);
        cells.extend([Value::Null, b(b""), Value::Null, i(4), i(4)]);
        cells.extend(vec![Value::Null; 14]);
        cells.extend([i(4), i(-1), Value::Null]);
        cells
    }

    /// An album with every field filled in, in library 1.
    fn full_album() -> AlbumRecord {
        AlbumRecord {
            id: album_id(1),
            library: library(1),
            title: "Collected Examples".to_owned(),
            title_sort: Some("collected examples".to_owned()),
            artist_credit: "The Examples".to_owned(),
            artists: vec![artist_id(1)],
            date: Some(date(1977, Some(2), Some(4))),
            original_date: Some(date(1976, None, None)),
            release_type: ReleaseType {
                primary: Some(PrimaryType::Album),
                secondary: vec![SecondaryType::Live, SecondaryType::Compilation],
            },
            compilation: true,
            genres: owned(&["Jazz"]),
            labels: owned(&["Example Records"]),
            track_count: 12,
            disc_count: 2,
            duration: lasting(2_400_000),
            has_artwork: false,
            release_mbid: Some(uuid("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")),
            release_group_mbid: Some(uuid("01234567-89ab-cdef-0123-456789abcdef")),
            added: at(1_700_000_000_001),
        }
    }

    /// The row of [`full_album`], written out by hand.
    fn full_album_cells() -> Vec<Value> {
        vec![
            t(ALBUM_1),
            t(LIBRARY_1),
            t("Collected Examples"),
            t("collected examples"),
            t("The Examples"),
            b(b"\0\0\0\0\0\0\0\x1eart_00000000000000000000000001"),
            i(19_770_204),
            i(19_760_000),
            i(1),
            b(&[7, 1]),
            i(1),
            b(b"\0\0\0\0\0\0\0\x04Jazz"),
            b(b"\0\0\0\0\0\0\0\x0fExample Records"),
            i(12),
            i(2),
            i(2_400_000),
            i(0),
            t("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            t("01234567-89ab-cdef-0123-456789abcdef"),
            i(1_700_000_000_001),
        ]
    }

    /// An album with nothing optional, in library 2.
    fn bare_album() -> AlbumRecord {
        AlbumRecord {
            id: album_id(2),
            library: library(2),
            title: "Untitled".to_owned(),
            title_sort: None,
            artist_credit: String::new(),
            artists: Vec::new(),
            date: None,
            original_date: None,
            release_type: ReleaseType::default(),
            compilation: false,
            genres: Vec::new(),
            labels: Vec::new(),
            track_count: 0,
            disc_count: 0,
            duration: lasting(0),
            has_artwork: true,
            release_mbid: None,
            release_group_mbid: None,
            added: at(0),
        }
    }

    /// The row of [`bare_album`], written out by hand.
    fn bare_album_cells() -> Vec<Value> {
        vec![
            t(ALBUM_2),
            t(LIBRARY_2),
            t("Untitled"),
            Value::Null,
            t(""),
            b(b""),
            Value::Null,
            Value::Null,
            Value::Null,
            b(b""),
            i(0),
            b(b""),
            b(b""),
            i(0),
            i(0),
            i(0),
            i(1),
            Value::Null,
            Value::Null,
            i(0),
        ]
    }

    /// An artist with every field filled in, in library 1.
    fn full_artist() -> ArtistRecord {
        ArtistRecord {
            id: artist_id(1),
            library: library(1),
            name: "The Examples".to_owned(),
            name_sort: Some("Examples, The".to_owned()),
            mbid: Some(uuid("99999999-8888-7777-6666-555555555555")),
            album_count: 3,
            track_count: 31,
            genres: owned(&["Jazz", "Modal"]),
            has_artwork: true,
        }
    }

    /// The row of [`full_artist`], written out by hand.
    fn full_artist_cells() -> Vec<Value> {
        vec![
            t(ARTIST_1),
            t(LIBRARY_1),
            t("The Examples"),
            t("Examples, The"),
            t("99999999-8888-7777-6666-555555555555"),
            i(3),
            i(31),
            b(b"\0\0\0\0\0\0\0\x04Jazz\0\0\0\0\0\0\0\x05Modal"),
            i(1),
        ]
    }

    /// An artist with nothing optional, in library 2.
    fn bare_artist() -> ArtistRecord {
        ArtistRecord {
            id: artist_id(2),
            library: library(2),
            name: "A. Guest".to_owned(),
            name_sort: None,
            mbid: None,
            album_count: 0,
            track_count: 0,
            genres: Vec::new(),
            has_artwork: false,
        }
    }

    /// The row of [`bare_artist`], written out by hand.
    fn bare_artist_cells() -> Vec<Value> {
        vec![
            t(ARTIST_2),
            t(LIBRARY_2),
            t("A. Guest"),
            Value::Null,
            Value::Null,
            i(0),
            i(0),
            b(b""),
            i(0),
        ]
    }

    /// The two credits the first batch gives track 1.
    fn two_credits() -> Vec<Credit> {
        vec![
            Credit::new("A. Guest".to_owned(), Role::Featured, None, None).expect("a credit"),
            Credit::new(
                "B. Player".to_owned(),
                Role::Performer,
                Some("fretless bass".to_owned()),
                Some(uuid("12121212-3434-5656-7878-909090909090")),
            )
            .expect("a credit"),
        ]
    }

    fn credits_of(track: u8, credits: Vec<Credit>) -> TrackCredits {
        TrackCredits {
            track: track_id(track),
            credits,
        }
    }

    /// The extra of kind `kind` of the track numbered `track`, holding
    /// `body`.
    fn kept(track: u8, kind: ExtraKind, body: &[u8]) -> TrackExtra {
        TrackExtra {
            track: track_id(track),
            kind,
            body: Some(body.to_vec()),
        }
    }

    /// Two artists, two albums and two tracks, one of each in library 1
    /// and one in library 2, with two credits and one extra of every kind
    /// for track 1 and a seek index for track 2.
    fn first_batch() -> CatalogBatch {
        CatalogBatch {
            artists: vec![full_artist(), bare_artist()],
            albums: vec![full_album(), bare_album()],
            tracks: vec![full_track(), bare_track()],
            credits: vec![credits_of(1, two_credits())],
            extras: vec![
                kept(1, ExtraKind::File, b"file one"),
                kept(1, ExtraKind::Artwork, b"art"),
                kept(1, ExtraKind::Lyrics, b"la la"),
                kept(1, ExtraKind::SeekIndex, b"seek"),
                kept(1, ExtraKind::FrameIndex, b"frames"),
                kept(2, ExtraKind::SeekIndex, b"two"),
            ],
            removed: Vec::new(),
        }
    }

    /// What applying [`first_batch`] to an empty catalogue reports.
    fn first_changes() -> Vec<CatalogChange> {
        vec![
            changed(RecordId::Artist(artist_id(1))),
            changed(RecordId::Artist(artist_id(2))),
            changed(RecordId::Album(album_id(1))),
            changed(RecordId::Album(album_id(2))),
            changed(RecordId::Track(track_id(1))),
            changed(RecordId::Track(track_id(2))),
        ]
    }

    /// A cache holding [`first_batch`].
    fn filled(parts: &[SchemaPart]) -> Cache {
        let cache = open(parts);
        assert_eq!(apply(&cache, &first_batch()), Ok(first_changes()));
        cache
    }

    /// The seek index a reader returns for track 1.
    fn seek_index_of_track_1() -> ExtraRow {
        ExtraRow {
            track: track_id(1),
            library: library(1),
            kind: ExtraKind::SeekIndex,
            body: b"seek".to_vec(),
        }
    }

    #[test]
    fn a_first_batch_reports_each_record_once_and_stores_these_rows() {
        let cache = filled(&[SCHEMA]);
        assert_eq!(
            select(&cache, ALL_TRACKS),
            vec![Row(full_track_cells()), Row(bare_track_cells())]
        );
        assert_eq!(
            select(&cache, ALL_ALBUMS),
            vec![Row(full_album_cells()), Row(bare_album_cells())]
        );
        assert_eq!(
            select(&cache, ALL_ARTISTS),
            vec![Row(full_artist_cells()), Row(bare_artist_cells())]
        );
        assert_eq!(
            select(&cache, ALL_CREDITS),
            vec![
                Row(vec![
                    t(TRACK_1),
                    i(0),
                    t("A. Guest"),
                    i(3),
                    Value::Null,
                    Value::Null
                ]),
                Row(vec![
                    t(TRACK_1),
                    i(1),
                    t("B. Player"),
                    i(9),
                    t("fretless bass"),
                    t("12121212-3434-5656-7878-909090909090"),
                ]),
            ]
        );
        assert_eq!(
            select(&cache, ALL_EXTRAS),
            vec![
                Row(vec![t(TRACK_1), i(1), b(b"file one")]),
                Row(vec![t(TRACK_1), i(2), b(b"art")]),
                Row(vec![t(TRACK_1), i(3), b(b"la la")]),
                Row(vec![t(TRACK_1), i(4), b(b"seek")]),
                Row(vec![t(TRACK_1), i(5), b(b"frames")]),
                Row(vec![t(TRACK_2), i(4), b(b"two")]),
            ]
        );
    }

    /// The property of LIB-016 at the storage layer: a rescan that finds
    /// nothing new writes nothing.
    #[test]
    fn applying_the_same_batch_again_writes_nothing_and_reports_nothing() {
        let cache = open(&[SCHEMA]);
        let before = select(&cache, WRITES);
        assert_eq!(apply(&cache, &first_batch()), Ok(first_changes()));
        let rows = everything(&cache);
        let writes = select(&cache, WRITES);
        assert_ne!(writes, before);
        assert_eq!(apply(&cache, &first_batch()), Ok(Vec::new()));
        assert_eq!(everything(&cache), rows);
        assert_eq!(select(&cache, WRITES), writes);
        assert_eq!(
            rows.iter().map(Vec::len).collect::<Vec<_>>(),
            [2, 2, 2, 2, 6]
        );
    }

    #[test]
    fn a_changed_record_is_reported_and_an_unchanged_one_is_not() {
        let cache = filled(&[SCHEMA]);
        let renamed = ArtistRecord {
            name: "The Samples".to_owned(),
            ..full_artist()
        };
        let longer = AlbumRecord {
            track_count: 13,
            ..full_album()
        };
        let offline = TrackRecord {
            availability: Availability::Offline,
            ..full_track()
        };
        let batch = CatalogBatch {
            artists: vec![bare_artist(), renamed.clone()],
            albums: vec![longer.clone(), bare_album()],
            tracks: vec![bare_track(), offline.clone()],
            ..CatalogBatch::default()
        };
        assert_eq!(
            apply(&cache, &batch),
            Ok(vec![
                changed(RecordId::Artist(artist_id(1))),
                changed(RecordId::Album(album_id(1))),
                changed(RecordId::Track(track_id(1))),
            ])
        );
        assert_eq!(apply(&cache, &batch), Ok(Vec::new()));
        assert_eq!(
            read(&cache, |reader| artist(reader, &owner(), artist_id(1))),
            Ok(Some(ArtistRow(renamed)))
        );
        assert_eq!(
            read(&cache, |reader| album(reader, &owner(), album_id(1))),
            Ok(Some(AlbumRow(longer)))
        );
        assert_eq!(
            read(&cache, |reader| track(reader, &owner(), track_id(1))),
            Ok(Some(TrackRow(offline)))
        );
        // A record that was never stored is new, whichever list it is in.
        let new = CatalogBatch {
            artists: vec![ArtistRecord {
                id: artist_id(3),
                ..bare_artist()
            }],
            ..CatalogBatch::default()
        };
        assert_eq!(
            apply(&cache, &new),
            Ok(vec![changed(RecordId::Artist(artist_id(3)))])
        );
    }

    #[test]
    fn a_change_to_a_tracks_credits_is_a_change_to_the_track() {
        let cache = filled(&[SCHEMA]);
        let only_credits = |track, credits| CatalogBatch {
            credits: vec![credits_of(track, credits)],
            ..CatalogBatch::default()
        };
        let track_1 = Ok(vec![changed(RecordId::Track(track_id(1)))]);
        let mut reversed = two_credits();
        reversed.reverse();
        let mut shorter = two_credits();
        shorter.truncate(1);
        // The same credits are no change; a new order, a shorter list and
        // no list at all each are.
        assert_eq!(apply(&cache, &only_credits(1, two_credits())), Ok(vec![]));
        assert_eq!(apply(&cache, &only_credits(1, reversed.clone())), track_1);
        assert_eq!(apply(&cache, &only_credits(1, reversed)), Ok(vec![]));
        assert_eq!(apply(&cache, &only_credits(1, shorter.clone())), track_1);
        assert_eq!(
            select(&cache, ALL_CREDITS),
            vec![Row(vec![
                t(TRACK_1),
                i(0),
                t("A. Guest"),
                i(3),
                Value::Null,
                Value::Null
            ])]
        );
        assert_eq!(apply(&cache, &only_credits(1, Vec::new())), track_1);
        assert_eq!(apply(&cache, &only_credits(1, Vec::new())), Ok(vec![]));
        assert_eq!(select(&cache, ALL_CREDITS), Vec::<Row>::new());
        // A track that had none gains one.
        assert_eq!(
            apply(&cache, &only_credits(2, shorter)),
            Ok(vec![changed(RecordId::Track(track_id(2)))])
        );
    }

    #[test]
    fn a_change_to_a_tracks_extras_is_a_change_to_the_track() {
        let cache = filled(&[SCHEMA]);
        let only_extra = |extra| CatalogBatch {
            extras: vec![extra],
            ..CatalogBatch::default()
        };
        let track_1 = Ok(vec![changed(RecordId::Track(track_id(1)))]);
        let lyrics = || {
            read(&cache, |reader| {
                extra(reader, &owner(), track_id(1), ExtraKind::Lyrics)
            })
        };
        let no_lyrics = TrackExtra {
            track: track_id(1),
            kind: ExtraKind::Lyrics,
            body: None,
        };
        // The same octets are no change; other octets and no octets are.
        let same = only_extra(kept(1, ExtraKind::Lyrics, b"la la"));
        assert_eq!(apply(&cache, &same), Ok(vec![]));
        let other = only_extra(kept(1, ExtraKind::Lyrics, b"la la la"));
        assert_eq!(apply(&cache, &other), track_1);
        assert_eq!(apply(&cache, &other), Ok(vec![]));
        assert_eq!(
            lyrics(),
            Ok(Some(ExtraRow {
                track: track_id(1),
                library: library(1),
                kind: ExtraKind::Lyrics,
                body: b"la la la".to_vec(),
            }))
        );
        assert_eq!(apply(&cache, &only_extra(no_lyrics.clone())), track_1);
        assert_eq!(apply(&cache, &only_extra(no_lyrics)), Ok(vec![]));
        assert_eq!(lyrics(), Ok(None));
        // The track's other extras are as they were.
        assert_eq!(
            read(&cache, |reader| extra(
                reader,
                &owner(),
                track_id(1),
                ExtraKind::SeekIndex
            )),
            Ok(Some(seek_index_of_track_1()))
        );
    }

    #[test]
    fn a_track_whose_record_credits_and_extras_all_change_is_reported_once() {
        let cache = filled(&[SCHEMA]);
        let batch = CatalogBatch {
            artists: vec![ArtistRecord {
                track_count: 32,
                ..full_artist()
            }],
            tracks: vec![TrackRecord {
                title: "Second Light".to_owned(),
                ..bare_track()
            }],
            credits: vec![credits_of(2, two_credits())],
            extras: vec![
                kept(2, ExtraKind::SeekIndex, b"two again"),
                kept(2, ExtraKind::File, b"file two"),
            ],
            removed: vec![RecordId::Album(album_id(2))],
            ..CatalogBatch::default()
        };
        // Upserts come first, in the order of the batch's lists, then
        // removals.
        assert_eq!(
            apply(&cache, &batch),
            Ok(vec![
                changed(RecordId::Artist(artist_id(1))),
                changed(RecordId::Track(track_id(2))),
                gone(RecordId::Album(album_id(2))),
            ])
        );
    }

    #[test]
    fn a_removal_is_reported_for_a_record_that_was_stored_and_for_no_other() {
        let cache = filled(&[SCHEMA]);
        let batch = CatalogBatch {
            removed: vec![
                RecordId::Track(track_id(9)),
                RecordId::Artist(artist_id(2)),
                RecordId::Album(album_id(1)),
                RecordId::Track(track_id(2)),
            ],
            ..CatalogBatch::default()
        };
        assert_eq!(
            apply(&cache, &batch),
            Ok(vec![
                gone(RecordId::Artist(artist_id(2))),
                gone(RecordId::Album(album_id(1))),
                gone(RecordId::Track(track_id(2))),
            ])
        );
        assert_eq!(apply(&cache, &batch), Ok(Vec::new()));
        assert_eq!(select(&cache, ALL_TRACKS), vec![Row(full_track_cells())]);
        assert_eq!(select(&cache, ALL_ALBUMS), vec![Row(bare_album_cells())]);
        assert_eq!(select(&cache, ALL_ARTISTS), vec![Row(full_artist_cells())]);
    }

    #[test]
    fn removing_a_track_removes_its_credits_and_extras_and_nothing_of_another_module() {
        let cache = filled(&[SCHEMA, OVERRIDES]);
        let more = CatalogBatch {
            credits: vec![credits_of(2, two_credits())],
            ..CatalogBatch::default()
        };
        assert_eq!(
            apply(&cache, &more),
            Ok(vec![changed(RecordId::Track(track_id(2)))])
        );
        assert_eq!(
            run(
                &cache,
                "INSERT INTO curation_overrides VALUES \
                 ('trk_00000000000000000000000001', 'keep apart'), \
                 ('alb_00000000000000000000000001', 'merge'), \
                 ('art_00000000000000000000000001', 'alias')"
            ),
            Ok(3)
        );
        let batch = CatalogBatch {
            removed: vec![
                RecordId::Track(track_id(1)),
                RecordId::Album(album_id(1)),
                RecordId::Artist(artist_id(1)),
            ],
            ..CatalogBatch::default()
        };
        assert_eq!(
            apply(&cache, &batch),
            Ok(vec![
                gone(RecordId::Track(track_id(1))),
                gone(RecordId::Album(album_id(1))),
                gone(RecordId::Artist(artist_id(1))),
            ])
        );
        // Track 1's credits and extras went with it; track 2's stayed.
        assert_eq!(
            select(&cache, "SELECT track, seq FROM credits ORDER BY track, seq"),
            vec![Row(vec![t(TRACK_2), i(0)]), Row(vec![t(TRACK_2), i(1)])]
        );
        assert_eq!(
            select(&cache, ALL_EXTRAS),
            vec![Row(vec![t(TRACK_2), i(4), b(b"two")])]
        );
        // The other module's rows still name all three.
        assert_eq!(
            select(&cache, "SELECT * FROM curation_overrides ORDER BY record"),
            vec![
                Row(vec![t(ALBUM_1), t("merge")]),
                Row(vec![t(ARTIST_1), t("alias")]),
                Row(vec![t(TRACK_1), t("keep apart")]),
            ]
        );
    }

    #[test]
    fn each_kind_of_extra_is_kept_and_returned_on_its_own() {
        let cache = filled(&[SCHEMA]);
        for (kind, body) in [
            (ExtraKind::File, "file one"),
            (ExtraKind::Artwork, "art"),
            (ExtraKind::Lyrics, "la la"),
            (ExtraKind::SeekIndex, "seek"),
            (ExtraKind::FrameIndex, "frames"),
        ] {
            assert_eq!(
                read(&cache, move |reader| extra(
                    reader,
                    &owner(),
                    track_id(1),
                    kind
                )),
                Ok(Some(ExtraRow {
                    track: track_id(1),
                    library: library(1),
                    kind,
                    body: body.as_bytes().to_vec(),
                }))
            );
        }
        // Track 2 has a seek index and nothing else, and track 3 does not
        // exist.
        assert_eq!(
            read(&cache, |reader| extra(
                reader,
                &owner(),
                track_id(2),
                ExtraKind::File
            )),
            Ok(None)
        );
        assert_eq!(
            read(&cache, |reader| extra(
                reader,
                &owner(),
                track_id(3),
                ExtraKind::SeekIndex
            )),
            Ok(None)
        );
    }

    /// What one permit sees through every reader.
    #[derive(Debug, PartialEq)]
    struct Seen {
        /// A page of each kind of record.
        tracks: Result<Vec<TrackRow>, ReadError>,
        albums: Result<Vec<AlbumRow>, ReadError>,
        artists: Result<Vec<ArtistRow>, ReadError>,
        /// Records 1 and 2 of each kind, each asked for alone.
        track: Result<Vec<TrackRow>, ReadError>,
        album: Result<Vec<AlbumRow>, ReadError>,
        artist: Result<Vec<ArtistRow>, ReadError>,
        /// The seek indexes of tracks 1 and 2, each asked for alone.
        seek_index: Result<Vec<ExtraRow>, ReadError>,
    }

    /// The rows that asking for two rows alone found.
    fn alone<T>(found: [Result<Option<T>, ReadError>; 2]) -> Result<Vec<T>, ReadError> {
        found
            .into_iter()
            .collect::<Result<Vec<Option<T>>, ReadError>>()
            .map(|found| found.into_iter().flatten().collect())
    }

    /// What the permit `holder` makes sees of [`first_batch`].
    fn seen(cache: &Cache, holder: fn() -> Permit) -> Seen {
        Seen {
            tracks: read(cache, move |reader| tracks(reader, &holder(), None, 10)),
            albums: read(cache, move |reader| albums(reader, &holder(), None, 10)),
            artists: read(cache, move |reader| artists(reader, &holder(), None, 10)),
            track: alone(
                [1, 2].map(|n| read(cache, move |reader| track(reader, &holder(), track_id(n)))),
            ),
            album: alone(
                [1, 2].map(|n| read(cache, move |reader| album(reader, &holder(), album_id(n)))),
            ),
            artist: alone(
                [1, 2].map(|n| read(cache, move |reader| artist(reader, &holder(), artist_id(n)))),
            ),
            seek_index: alone([1, 2].map(|n| {
                read(cache, move |reader| {
                    extra(reader, &holder(), track_id(n), ExtraKind::SeekIndex)
                })
            })),
        }
    }

    /// A permit that sees exactly these rows, as a page and one by one.
    fn sees(
        tracks: Vec<TrackRow>,
        albums: Vec<AlbumRow>,
        artists: Vec<ArtistRow>,
        seek_index: Vec<ExtraRow>,
    ) -> Seen {
        Seen {
            tracks: Ok(tracks.clone()),
            albums: Ok(albums.clone()),
            artists: Ok(artists.clone()),
            track: Ok(tracks),
            album: Ok(albums),
            artist: Ok(artists),
            seek_index: Ok(seek_index),
        }
    }

    /// Verifies: SEC-IAM-070
    #[test]
    fn a_reader_never_returns_a_row_in_a_library_the_permit_does_not_hold() {
        let cache = filled(&[SCHEMA]);
        let seek_index_of_track_2 = || ExtraRow {
            track: track_id(2),
            library: library(2),
            kind: ExtraKind::SeekIndex,
            body: b"two".to_vec(),
        };
        let library_1 = || {
            sees(
                vec![TrackRow(full_track())],
                vec![AlbumRow(full_album())],
                vec![ArtistRow(full_artist())],
                vec![seek_index_of_track_1()],
            )
        };
        let library_2 = || {
            sees(
                vec![TrackRow(bare_track())],
                vec![AlbumRow(bare_album())],
                vec![ArtistRow(bare_artist())],
                vec![seek_index_of_track_2()],
            )
        };
        let both = || {
            sees(
                vec![TrackRow(full_track()), TrackRow(bare_track())],
                vec![AlbumRow(full_album()), AlbumRow(bare_album())],
                vec![ArtistRow(full_artist()), ArtistRow(bare_artist())],
                vec![seek_index_of_track_1(), seek_index_of_track_2()],
            )
        };
        let neither = || sees(Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let cases: [(fn() -> Permit, Seen); 7] = [
            (owner, both()),
            (|| member(&[1, 2]), both()),
            (|| member(&[3, 2, 1]), both()),
            (|| member(&[1]), library_1()),
            (|| member(&[2]), library_2()),
            (|| member(&[3]), neither()),
            (|| member(&[]), neither()),
        ];
        for (case, (holder, expected)) in cases.into_iter().enumerate() {
            assert_eq!((case, seen(&cache, holder)), (case, expected));
        }
    }

    /// The artist numbered `n`, in the library numbered `home`.
    fn artist_in(n: u8, home: u8) -> ArtistRecord {
        ArtistRecord {
            id: artist_id(n),
            library: library(home),
            ..bare_artist()
        }
    }

    #[test]
    fn a_page_holds_the_next_rows_the_permit_holds_in_identifier_order() {
        let cache = open(&[SCHEMA]);
        let numbered_album = |n| AlbumRecord {
            id: album_id(n),
            ..bare_album()
        };
        let numbered_track = |n| TrackRecord {
            id: track_id(n),
            ..bare_track()
        };
        let batch = CatalogBatch {
            artists: [(3, 2), (1, 2), (5, 2), (2, 1), (4, 1)]
                .map(|(n, home)| artist_in(n, home))
                .to_vec(),
            albums: [3, 1, 2].map(numbered_album).to_vec(),
            tracks: [2, 3, 1].map(numbered_track).to_vec(),
            ..CatalogBatch::default()
        };
        assert_eq!(apply(&cache, &batch).as_ref().map(Vec::len), Ok(11));
        let page = |holder: fn() -> Permit, after: Option<u8>, limit: u32| {
            read(&cache, move |reader| {
                artists(reader, &holder(), after.map(artist_id), limit)
            })
        };
        let found = |numbered: &[(u8, u8)]| -> Result<Vec<ArtistRow>, ReadError> {
            Ok(numbered
                .iter()
                .map(|(n, home)| ArtistRow(artist_in(*n, *home)))
                .collect())
        };
        assert_eq!(page(owner, None, 2), found(&[(1, 2), (2, 1)]));
        assert_eq!(page(owner, Some(2), 2), found(&[(3, 2), (4, 1)]));
        assert_eq!(page(owner, Some(4), 2), found(&[(5, 2)]));
        assert_eq!(page(owner, Some(5), 2), found(&[]));
        assert_eq!(page(owner, None, 0), found(&[]));
        assert_eq!(
            page(owner, None, 10),
            found(&[(1, 2), (2, 1), (3, 2), (4, 1), (5, 2)])
        );
        // The limit counts only the rows the permit holds.
        assert_eq!(page(|| member(&[2]), None, 2), found(&[(1, 2), (3, 2)]));
        assert_eq!(page(|| member(&[2]), Some(3), 2), found(&[(5, 2)]));
        assert_eq!(
            read(&cache, |reader| tracks(
                reader,
                &owner(),
                Some(track_id(1)),
                1
            )),
            Ok(vec![TrackRow(numbered_track(2))])
        );
        assert_eq!(
            read(&cache, |reader| albums(
                reader,
                &owner(),
                Some(album_id(2)),
                5
            )),
            Ok(vec![AlbumRow(numbered_album(3))])
        );
    }

    /// Verifies: SEC-API-066
    #[test]
    fn text_with_sql_metacharacters_is_stored_and_read_back_as_it_is() {
        let cache = open(&[SCHEMA]);
        for hostile in HOSTILE {
            let text = || hostile.to_owned();
            let track = TrackRecord {
                title: text(),
                title_sort: Some(text()),
                artist_credit: text(),
                disc_subtitle: Some(text()),
                genres: vec![text(), "Jazz".to_owned()],
                moods: vec![text()],
                styles: vec![text()],
                labels: vec![text()],
                grouping: vec![text()],
                ..full_track()
            };
            let album = AlbumRecord {
                title: text(),
                title_sort: Some(text()),
                artist_credit: text(),
                genres: vec![text()],
                labels: vec![text()],
                ..full_album()
            };
            let artist_record = ArtistRecord {
                name: text(),
                name_sort: Some(text()),
                genres: vec![text()],
                ..full_artist()
            };
            let credit = Credit::new(text(), Role::Composer, Some(text()), None).expect("a credit");
            let batch = CatalogBatch {
                artists: vec![artist_record.clone()],
                albums: vec![album.clone()],
                tracks: vec![track.clone()],
                credits: vec![credits_of(1, vec![credit])],
                extras: vec![kept(1, ExtraKind::Lyrics, hostile.as_bytes())],
                removed: Vec::new(),
            };
            let lyrics = ExtraRow {
                track: track_id(1),
                library: library(1),
                kind: ExtraKind::Lyrics,
                body: hostile.as_bytes().to_vec(),
            };
            let stored = (
                apply(&cache, &batch),
                read(&cache, |reader| tracks(reader, &owner(), None, 10)),
                read(&cache, |reader| albums(reader, &owner(), None, 10)),
                read(&cache, |reader| artists(reader, &owner(), None, 10)),
                read(&cache, |reader| {
                    extra(reader, &owner(), track_id(1), ExtraKind::Lyrics)
                }),
                select(&cache, ALL_CREDITS),
            );
            assert_eq!(
                (hostile, stored),
                (
                    hostile,
                    (
                        Ok(vec![
                            changed(RecordId::Artist(artist_id(1))),
                            changed(RecordId::Album(album_id(1))),
                            changed(RecordId::Track(track_id(1))),
                        ]),
                        Ok(vec![TrackRow(track)]),
                        Ok(vec![AlbumRow(album)]),
                        Ok(vec![ArtistRow(artist_record)]),
                        Ok(Some(lyrics)),
                        vec![Row(vec![
                            t(TRACK_1),
                            i(0),
                            t(hostile),
                            i(4),
                            t(hostile),
                            Value::Null
                        ])],
                    )
                )
            );
        }
    }

    #[test]
    fn a_stored_row_that_cannot_be_read_fails_the_read_and_is_not_left_out() {
        let cache = filled(&[SCHEMA]);
        assert_eq!(
            run(
                &cache,
                "UPDATE tracks SET kind = 99 WHERE id = 'trk_00000000000000000000000002'"
            ),
            Ok(1)
        );
        assert_eq!(
            read(&cache, |reader| tracks(reader, &owner(), None, 10)),
            Err(ReadError::Damaged)
        );
        assert_eq!(
            read(&cache, |reader| track(reader, &owner(), track_id(2))),
            Err(ReadError::Damaged)
        );
        // A read that does not reach the damaged row still answers.
        assert_eq!(
            read(&cache, |reader| tracks(reader, &member(&[1]), None, 10)),
            Ok(vec![TrackRow(full_track())])
        );
        assert_eq!(
            run(
                &cache,
                "UPDATE albums SET compilation = 2 WHERE id = 'alb_00000000000000000000000002'"
            ),
            Ok(1)
        );
        assert_eq!(
            read(&cache, |reader| albums(reader, &owner(), None, 10)),
            Err(ReadError::Damaged)
        );
        assert_eq!(
            read(&cache, |reader| album(reader, &owner(), album_id(2))),
            Err(ReadError::Damaged)
        );
        assert_eq!(
            run(
                &cache,
                "UPDATE artists SET mbid = 'not a UUID' \
                 WHERE id = 'art_00000000000000000000000002'"
            ),
            Ok(1)
        );
        assert_eq!(
            read(&cache, |reader| artists(reader, &owner(), None, 10)),
            Err(ReadError::Damaged)
        );
        assert_eq!(
            read(&cache, |reader| artist(reader, &owner(), artist_id(2))),
            Err(ReadError::Damaged)
        );
        assert_eq!(
            run(
                &cache,
                "UPDATE tracks SET library = 'nowhere' \
                 WHERE id = 'trk_00000000000000000000000001'"
            ),
            Ok(1)
        );
        assert_eq!(
            read(&cache, |reader| extra(
                reader,
                &owner(),
                track_id(1),
                ExtraKind::SeekIndex
            )),
            Err(ReadError::Damaged)
        );
    }

    #[test]
    fn a_cache_without_the_catalogues_tables_fails_every_call_with_sqlites_error() {
        let cache = open(&[OVERRIDES]);
        let no_such_table = || DbError::Sqlite { code: 1 };
        assert_eq!(
            apply(&cache, &first_batch()),
            Err(StoreError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| tracks(reader, &owner(), None, 10)),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| track(reader, &owner(), track_id(1))),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| albums(reader, &owner(), None, 10)),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| album(reader, &owner(), album_id(1))),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| artists(reader, &owner(), None, 10)),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| artist(reader, &owner(), artist_id(1))),
            Err(ReadError::Db(no_such_table()))
        );
        assert_eq!(
            read(&cache, |reader| extra(
                reader,
                &owner(),
                track_id(1),
                ExtraKind::File
            )),
            Err(ReadError::Db(no_such_table()))
        );
    }

    #[test]
    fn a_record_reads_back_from_the_cells_of_its_row() {
        assert_eq!(track_row(&full_track_cells()), Some(TrackRow(full_track())));
        assert_eq!(track_row(&bare_track_cells()), Some(TrackRow(bare_track())));
        assert_eq!(album_row(&full_album_cells()), Some(AlbumRow(full_album())));
        assert_eq!(album_row(&bare_album_cells()), Some(AlbumRow(bare_album())));
        assert_eq!(
            artist_row(&full_artist_cells()),
            Some(ArtistRow(full_artist()))
        );
        assert_eq!(
            artist_row(&bare_artist_cells()),
            Some(ArtistRow(bare_artist()))
        );
        assert_eq!(
            extra_row(&seek_index_cells(), ExtraKind::SeekIndex),
            Some(seek_index_of_track_1())
        );
    }

    /// The cells a reader gets for the seek index of track 1: the track,
    /// the track's library and the octets.
    fn seek_index_cells() -> Vec<Value> {
        vec![t(TRACK_1), t(LIBRARY_1), b(b"seek")]
    }

    /// `cells` with the cell at `index` replaced by `value`.
    fn with(cells: &[Value], index: usize, value: Value) -> Vec<Value> {
        let mut changed = cells.to_vec();
        changed[index] = value;
        changed
    }

    /// Fails unless `read` refuses `cells` with any one cell replaced by a
    /// value of a type no column holds, with the first cell missing, and
    /// with a cell too many.
    fn refuses_each_wrong_cell<T: Debug + PartialEq>(
        read: fn(&[Value]) -> Option<T>,
        cells: &[Value],
    ) {
        for index in 0..cells.len() {
            assert_eq!(
                (index, read(&with(cells, index, Value::Real(0.5)))),
                (index, None)
            );
        }
        assert_eq!(read(&cells[1..]), None);
        let mut longer = cells.to_vec();
        longer.push(Value::Null);
        assert_eq!(read(&longer), None);
    }

    #[test]
    fn a_row_with_a_cell_of_the_wrong_type_or_the_wrong_number_of_cells_is_refused() {
        refuses_each_wrong_cell(track_row, &full_track_cells());
        refuses_each_wrong_cell(album_row, &full_album_cells());
        refuses_each_wrong_cell(artist_row, &full_artist_cells());
        refuses_each_wrong_cell(
            |cells| extra_row(cells, ExtraKind::SeekIndex),
            &seek_index_cells(),
        );
    }

    #[test]
    fn a_track_row_holding_a_value_outside_its_range_is_refused() {
        let cases = [
            // An album's identifier, a malformed one, and unknown kinds.
            (0, t(ALBUM_1)),
            (0, t("trk_1")),
            (1, i(0)),
            (1, i(70_000)),
            (2, t(TRACK_1)),
            // An artist list naming a track, and an album cell naming an
            // artist.
            (6, b(b"\0\0\0\0\0\0\0\x1etrk_00000000000000000000000001")),
            (7, t(ARTIST_1)),
            // Track 0, a negative total, disc 10,000 and a total past 16
            // bits.
            (8, i(0)),
            (9, i(-1)),
            (10, i(10_000)),
            (11, i(65_536)),
            // Month 13, a day without a month, year 0, a year past 16 bits,
            // a negative date and one past 32 bits.
            (13, i(19_771_301)),
            (13, i(19_770_004)),
            (13, i(204)),
            (13, i(700_000_101)),
            (14, i(-1)),
            (14, i(4_294_967_296)),
            // A list cut off in a length, a text shorter than its length, a
            // text that is not UTF-8 and a length no blob can hold.
            (15, b(b"\0\0\0")),
            (16, b(b"\0\0\0\0\0\0\0\x09Calm")),
            (17, b(b"\0\0\0\0\0\0\0\x01\xff")),
            (18, b(b"\xff\xff\xff\xff\xff\xff\xff\xffRecords")),
            (20, i(3)),
            (21, b(b"\0\0\0\0\0\0\0\x0bnot an ISRC")),
            (22, t("not a UUID")),
            // An unknown codec, MP3 in a FLAC container, an unknown
            // container, and a format outside each part's range.
            (23, i(99)),
            (23, i(4)),
            (24, i(99)),
            (25, i(0)),
            (26, i(65)),
            (27, i(0)),
            (28, i(0)),
            (29, i(2_592_000_001)),
            // An unknown scale, bits that are no 32-bit float, a gain of
            // 200 dB and a peak of 17.
            (30, i(3)),
            (31, i(-1)),
            (31, i(1_128_792_064)),
            (32, i(1_099_431_936)),
            (33, i(0)),
            (36, i(-1)),
            (37, i(4_294_967_296)),
            // Plain lyrics from a frame that is always timed, and unknown
            // timing, availability and origin.
            (38, i(1)),
            (38, i(4)),
            (39, i(0)),
            (40, i(i64::MAX)),
            (41, i(7)),
        ];
        for (index, value) in cases {
            let cells = with(&full_track_cells(), index, value.clone());
            assert_eq!((index, &value, track_row(&cells)), (index, &value, None));
        }
    }

    #[test]
    fn an_album_or_artist_row_holding_a_value_outside_its_range_is_refused() {
        let albums = [
            (0, t(TRACK_1)),
            (1, t(ALBUM_1)),
            (5, b(b"\0")),
            // Year 0 and the 30th of February.
            (6, i(0)),
            (7, i(19_760_230)),
            (8, i(6)),
            (9, b(&[13])),
            (9, b(&[1, 0])),
            // A flag is 0 or 1.
            (10, i(2)),
            (10, i(-1)),
            (13, i(-1)),
            (14, i(65_536)),
            (15, i(2_592_000_001)),
            (16, i(2)),
            (17, t("not a UUID")),
            (18, t("")),
            (19, i(i64::MIN)),
        ];
        for (index, value) in albums {
            let cells = with(&full_album_cells(), index, value.clone());
            assert_eq!((index, &value, album_row(&cells)), (index, &value, None));
        }
        let artists = [
            (0, t(ALBUM_1)),
            (1, t("lib_1")),
            (4, t("not a UUID")),
            (5, i(-1)),
            (6, i(4_294_967_296)),
            (7, b(b"\0")),
            (8, i(2)),
        ];
        for (index, value) in artists {
            let cells = with(&full_artist_cells(), index, value.clone());
            assert_eq!((index, &value, artist_row(&cells)), (index, &value, None));
        }
        // An extra of something that is not a track, or in something that
        // is not a library.
        for (index, value) in [(0, t(ALBUM_1)), (1, t(TRACK_1))] {
            let cells = with(&seek_index_cells(), index, value);
            assert_eq!(
                (index, extra_row(&cells, ExtraKind::SeekIndex)),
                (index, None)
            );
        }
    }

    #[test]
    fn every_row_names_the_library_it_is_in() {
        assert_eq!(TrackRow(full_track()).library(), library(1).get());
        assert_eq!(AlbumRow(bare_album()).library(), library(2).get());
        assert_eq!(ArtistRow(full_artist()).library(), library(1).get());
        assert_eq!(seek_index_of_track_1().library(), library(1).get());
        // So a permit's libraries admit or refuse it.
        let permit = member(&[1]);
        assert!(permit.libraries().admits(&TrackRow(full_track())));
        assert!(!permit.libraries().admits(&AlbumRow(bare_album())));
    }

    #[test]
    fn the_record_tables_hold_the_fields_of_the_field_table_under_their_names() {
        let table = |record: RecordKind| match record {
            RecordKind::Track => "tracks",
            RecordKind::Album => "albums",
            RecordKind::Artist => "artists",
        };
        let fields: Vec<(&str, &str)> = CatalogField::ALL
            .iter()
            .map(|field| (table(field.record()), field.name()))
            .collect();
        let columns: Vec<(&str, &str)> = SCHEMA
            .columns
            .iter()
            .map(|column| (column.table, column.name))
            .collect();
        let (records, others) = columns.split_at(71);
        assert_eq!(records, fields);
        assert_eq!(
            others,
            [
                ("credits", "track"),
                ("credits", "seq"),
                ("credits", "name"),
                ("credits", "role"),
                ("credits", "detail"),
                ("credits", "mbid"),
                ("extras", "track"),
                ("extras", "kind"),
                ("extras", "body"),
            ]
        );
        // Every column is library data.
        assert_eq!(
            column("tracks", "title"),
            Column {
                table: "tracks",
                name: "title",
                class: DataClass::Library,
            }
        );
        assert!(
            SCHEMA
                .columns
                .iter()
                .all(|column| column.class == DataClass::Library)
        );
    }
}
