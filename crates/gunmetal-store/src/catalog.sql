-- The catalogue's tables (WP-067): the synced-library records, each
-- track's credits, and the extras kept beside a track.
--
-- The three record tables hold one column for each field of the core's
-- field table, under the field's name and in the order of the fields'
-- codes. A list is one blob in which each entry follows its length in
-- octets, written as eight big-endian octets. A date is one number, year *
-- 10000 + month * 100 + day, with 0 for a month or day that is not known.
-- A gain or a peak is the bit pattern of its 32-bit float, so that it reads
-- back exactly. A value from a closed vocabulary is its stable code.

CREATE TABLE tracks (
    id TEXT NOT NULL PRIMARY KEY,
    kind INTEGER NOT NULL,
    library TEXT NOT NULL,
    title TEXT NOT NULL,
    title_sort TEXT,
    artist_credit TEXT NOT NULL,
    artists BLOB NOT NULL,
    album TEXT,
    track_number INTEGER,
    track_total INTEGER,
    disc_number INTEGER,
    disc_total INTEGER,
    disc_subtitle TEXT,
    date INTEGER,
    original_date INTEGER,
    genres BLOB NOT NULL,
    moods BLOB NOT NULL,
    styles BLOB NOT NULL,
    labels BLOB NOT NULL,
    grouping BLOB NOT NULL,
    advisory INTEGER,
    isrc BLOB NOT NULL,
    recording_mbid TEXT,
    codec INTEGER NOT NULL,
    container INTEGER NOT NULL,
    sample_rate INTEGER,
    bit_depth INTEGER,
    channels INTEGER,
    bitrate INTEGER,
    duration INTEGER,
    track_gain_scale INTEGER,
    track_gain INTEGER,
    track_peak INTEGER,
    album_gain_scale INTEGER,
    album_gain INTEGER,
    album_peak INTEGER,
    trim_delay INTEGER,
    trim_padding INTEGER,
    lyrics_timing INTEGER,
    availability INTEGER NOT NULL,
    added INTEGER NOT NULL,
    lyrics_origin INTEGER
) STRICT, WITHOUT ROWID;

CREATE TABLE albums (
    id TEXT NOT NULL PRIMARY KEY,
    library TEXT NOT NULL,
    title TEXT NOT NULL,
    title_sort TEXT,
    artist_credit TEXT NOT NULL,
    artists BLOB NOT NULL,
    date INTEGER,
    original_date INTEGER,
    primary_type INTEGER,
    secondary_types BLOB NOT NULL,
    compilation INTEGER NOT NULL,
    genres BLOB NOT NULL,
    labels BLOB NOT NULL,
    track_count INTEGER NOT NULL,
    disc_count INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    has_artwork INTEGER NOT NULL,
    release_mbid TEXT,
    release_group_mbid TEXT,
    added INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE artists (
    id TEXT NOT NULL PRIMARY KEY,
    library TEXT NOT NULL,
    name TEXT NOT NULL,
    name_sort TEXT,
    mbid TEXT,
    album_count INTEGER NOT NULL,
    track_count INTEGER NOT NULL,
    genres BLOB NOT NULL,
    has_artwork INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

-- One row for each credit of a track, in the order the credits were given.
CREATE TABLE credits (
    track TEXT NOT NULL,
    seq INTEGER NOT NULL,
    name TEXT NOT NULL,
    role INTEGER NOT NULL,
    detail TEXT,
    mbid TEXT,
    PRIMARY KEY (track, seq)
) STRICT, WITHOUT ROWID;

-- What is kept beside a track's record, one row for each kind: the octets
-- the package that owns that kind encoded.
CREATE TABLE extras (
    track TEXT NOT NULL,
    kind INTEGER NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY (track, kind)
) STRICT, WITHOUT ROWID;
