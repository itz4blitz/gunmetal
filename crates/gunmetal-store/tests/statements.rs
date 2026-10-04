//! Values reach the cache only as bound parameters, so text full of SQL
//! metacharacters is stored and read back as it is and runs as nothing.
//! Real SQLite files in a temporary data root.

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use gunmetal_fs::sqlite::{Query, Value};
use support::{Data, FIRST, INSERT_TRACK, TRACKS, named, text, titles, wait};

/// Titles a request might send to break out of a statement.
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

const BY_TITLE: Query = Query::new("SELECT title FROM tracks WHERE title = ?1");

/// Verifies: SEC-API-066
#[test]
fn text_with_sql_metacharacters_is_stored_and_matched_as_it_is() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(
        wait(store.write(|tx| {
            HOSTILE
                .iter()
                .map(|title| tx.execute(&INSERT_TRACK.bind(text(title))))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Into::into)
        })),
        Ok(vec![1; HOSTILE.len()])
    );
    assert_eq!(titles(&store), named(&HOSTILE));
    // An exact match finds each title once; `%` and `_` are not wildcards.
    for title in HOSTILE {
        assert_eq!(
            (
                title,
                wait(store.read(move |reader| reader.query(&BY_TITLE.bind(text(title)))))
            ),
            (title, Ok(Ok(named(&[title]))))
        );
    }
    let missing =
        wait(store.read(|reader| reader.query(&BY_TITLE.bind(Value::Text("Intro".to_owned())))));
    assert_eq!(missing, Ok(Ok(vec![])));
}
