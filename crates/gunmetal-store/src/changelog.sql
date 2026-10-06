-- The ordered change log (LIB-018): one row for each change to a synced
-- record, numbered in the order the changes were committed.
--
-- AUTOINCREMENT makes SQLite keep the highest number it has given out in
-- its own sqlite_sequence table, so compaction never frees a number, even
-- when it has removed every row. The log reads its head from there. That
-- table is in this file with the rows: the cache runs with
-- synchronous=NORMAL, so a power loss can take its last commits, and the
-- numbers they held are then given out again.
--
-- A row holds a record's public identifier in its text form and the code
-- of what happened to it (1 for written, 2 for removed), and nothing of
-- the record itself.
--
-- The checks name the kinds of record and the codes this build logs: a
-- track, an album or an artist, written or removed. A row of any other
-- kind or code is refused when it is written. A build that logs another
-- kind or code has other text here, so its schema digest is another and it
-- builds its own cache instead of reading rows it does not know. The
-- checks do not read the rest of an identifier: the log does that when it
-- reads a row.
CREATE TABLE changelog (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    record TEXT NOT NULL CHECK (substr(record, 1, 4) IN ('trk_', 'alb_', 'art_')),
    op INTEGER NOT NULL CHECK (op IN (1, 2))
) STRICT;
