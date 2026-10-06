//! The architecture test of the authorisation door: no module of the server
//! crate reaches stored objects without the policy's `Permit`, or gets a
//! `Permit` without the layer, except the few on a written list
//! (SEC-TM-024, SEC-API-010).
//!
//! The storage readers that return a user-visible object take a `Permit`,
//! which only the core's policy function mints, so a handler that has not
//! asked the policy cannot call them. Some entry points take none.
//! [`WRITTEN_LIST`] names each of them and the server modules that may name
//! it, and this test reads every source file of the server crate on every
//! gate run and fails when any other module names one. A module a later
//! package adds is therefore checked the moment it merges, without anyone
//! adding it to a list here. Only `src/` is read: the crate's integration
//! tests are not modules of the server.
//!
//! How a name is found. The search is textual and fails closed. A line names
//! an entry point when it holds the name as a whole identifier, and not as
//! part of a longer one, wherever it stands: called, path-qualified,
//! imported alone, in a group or under an alias, or written in a comment or
//! a string. A function cannot be called without its name being written,
//! at the call or at an import, and a glob import does not hide the call.
//! The policy function's name is also an English word, so for it alone a
//! comment line is not read (see [`comment`]).
//!
//! The lookups need more than that, because three directories may name the
//! lookup type and each may make only its own lookup. [`indirect`] lets a
//! module name the type in three forms only: a lookup in full, such as
//! `PrePrincipal::Grant`; the plain import, `use <path>::PrePrincipal;`; and
//! the store's error step for a failed lookup, `Step::PrePrincipal(`. A
//! group, glob or aliased import of the lookups, a type alias and a
//! parameter of the type all fail. Every call of the reader must also give
//! it a lookup in full, with nothing but spaces and line breaks between, so
//! a lookup that reaches a call as a value fails too. Each lookup in full is
//! then held to the directory that owns it by its own entry.
//!
//! What the search does not hold:
//!
//! - Code that a macro writes from what it is handed. A macro handed one
//!   lookup in full can expand to another, and the search reads only the
//!   source as written.
//! - An entry point that a listed module hands on, by a re-export, a
//!   wrapper or a value it returns. The search holds who names an entry
//!   point, not who reaches it through a module that may; what the listed
//!   directories make public is theirs to keep narrow.
//! - Inside `access/`, `session/` and `verifier/`, the SQLite door's
//!   openers are allowed whole: the search does not tell a test's second
//!   connection to the identity database from a module's own read of it.
//! - The identity store's writer, which takes no `Permit`. It is not on the
//!   list, so any module that holds the store can change any of its tables,
//!   the grants table included, and only review stops that.
//! - The cache's raw reader in `gunmetal-store`, which hands any caller a
//!   reader for any static query. The server crate does not depend on that
//!   crate yet.
//! - A call made inside another crate on the server's behalf.
//!
//! An entry point also has to stay findable. Each entry names the directory
//! that declares it, and once that directory exists its sources must still
//! hold the name: renaming an entry point without changing the list fails
//! here, instead of quietly leaving the new name unwatched. The user log's
//! replay (WP-068) is listed under the name its plan entry gives. The audit
//! log (WP-069) is not built, and the names its plan entry sketches for its
//! verifier and its append, `verify` and `append`, are too common for a
//! textual search to hold to one module. It owes the list an entry for
//! each, and [`OWED`] makes this test fail the moment its directory holds a
//! source file and no entry is declared in it.
#![expect(
    clippy::disallowed_methods,
    reason = "this test lists and reads the server crate's own source files by path, and writes a fixture tree in a scratch directory, so that a module a later package adds is scanned without anyone listing it; no door module reads a source tree, and a file included at compile time cannot be listed (SEC-TM-024)"
)]

use std::fs;
use std::path::{Path, PathBuf};

use gunmetal_testkit::tempdir::TempDir;

/// One source file: its path below the directory that was read, with `/`
/// between its parts, and its text.
type Source = (String, String);

/// Which lines of a module are read for an entry point's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lines {
    /// Every line: code, comment and string alike.
    All,
    /// Every line but a comment line, for a name that is also an English
    /// word.
    Code,
}

/// An entry point that takes no `Permit`: a way to stored objects, or to a
/// `Permit`, that does not pass through the layer.
struct Door {
    /// The name a caller has to write to use it.
    name: &'static str,
    /// The directory below `crates/` whose sources declare it.
    declared_in: &'static str,
    /// The server modules that may name it, as paths below `src/`: a
    /// directory, written with a final `/`, stands for every module in it.
    callers: &'static [&'static str],
    /// Which lines of a module are read for its name.
    lines: Lines,
}

/// The written list: every entry point that takes no `Permit`, and who may
/// name it.
///
/// - The identity store's pre-principal lookups, which run before there is
///   a principal, and the reader that makes them: the session-token lookup
///   (`session/`, WP-062), the credential lookup behind the verifier's
///   pre-authentication handle (`verifier/`, WP-064) and the grant read
///   that builds the `Permit` (`access/`, WP-065). Each lookup is held to
///   the directory that owns it, and [`indirect`] holds the forms in which
///   the type and the reader may be written.
/// - The user log's rebuild replay, which yields events only into the
///   projection builders and returns nothing a handler can serve
///   (`startup/`, WP-095).
/// - The SQLite door's two openers. Either hands back a connection that
///   runs any statement on the database it is given, so a module that
///   opened the identity database could read or change any row of it with
///   no `Permit`. Only the three directories that own a lookup may name
///   them: the session and the verifier open a second connection in their
///   tests (WP-062, WP-064), and the layer opens a cache in its own.
/// - The policy function, which mints a real `Permit` from facts and a
///   context its caller fills in. A module that called it would skip the
///   store's grant read, the listener's path class and the refusal's event,
///   so only the layer may (`access/`, WP-065). It is read on code lines
///   only.
const WRITTEN_LIST: [Door; 9] = [
    Door {
        name: "read_pre_principal",
        declared_in: "gunmetal-durable/src/identity",
        callers: &["access/", "session/", "verifier/"],
        lines: Lines::All,
    },
    Door {
        name: "PrePrincipal",
        declared_in: "gunmetal-durable/src/identity",
        callers: &["access/", "session/", "verifier/"],
        lines: Lines::All,
    },
    Door {
        name: "PrePrincipal::SessionToken",
        declared_in: "gunmetal-durable/src/identity",
        callers: &["session/"],
        lines: Lines::All,
    },
    Door {
        name: "PrePrincipal::Credential",
        declared_in: "gunmetal-durable/src/identity",
        callers: &["verifier/"],
        lines: Lines::All,
    },
    Door {
        name: "PrePrincipal::Grant",
        declared_in: "gunmetal-durable/src/identity",
        callers: &["access/"],
        lines: Lines::All,
    },
    Door {
        name: "replay_into",
        declared_in: "gunmetal-durable/src/userlog",
        callers: &["startup/"],
        lines: Lines::All,
    },
    Door {
        name: "open_db",
        declared_in: "gunmetal-fs/src",
        callers: &["access/", "session/", "verifier/"],
        lines: Lines::All,
    },
    Door {
        name: "open_untrusted",
        declared_in: "gunmetal-fs/src",
        callers: &["access/", "session/", "verifier/"],
        lines: Lines::All,
    },
    Door {
        name: "decide",
        declared_in: "gunmetal-core/src/authz",
        callers: &["access/"],
        lines: Lines::Code,
    },
];

/// The storage directories below `crates/` whose packages owe the list
/// entries it cannot hold yet: the audit log's verifier and its append
/// (WP-069). Once one of them holds a source file, the list must hold an
/// entry declared in it.
const OWED: [&str; 1] = ["gunmetal-durable/src/audit"];

/// The type whose variants are the identity store's pre-principal lookups.
const LOOKUP: &str = "PrePrincipal";

/// The lookups, as a module writes them after `PrePrincipal::`.
const LOOKUPS: [&str; 3] = ["SessionToken", "Credential", "Grant"];

/// The identity store's reader that makes a lookup.
const READER: &str = "read_pre_principal";

/// A line of a module that names an entry point the module is not listed
/// for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Use {
    /// The module's path below `src/`.
    module: String,
    /// The line, counted from 1.
    line: usize,
    /// The entry point it names.
    door: &'static str,
}

/// Every Rust source file below `dir`, sorted by path. A directory that
/// does not exist holds none.
fn sources(dir: &Path) -> Vec<Source> {
    let mut found = Vec::new();
    walk(dir, "", &mut found);
    found.sort();
    found
}

/// Adds to `found` every Rust source file below `dir`, each path written
/// after `prefix`.
fn walk(dir: &Path, prefix: &str, found: &mut Vec<Source>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("a directory entry is readable");
        let name = entry.file_name().into_string().expect("a UTF-8 file name");
        let path = format!("{prefix}{name}");
        if entry.file_type().expect("a file type").is_dir() {
            walk(&entry.path(), &format!("{path}/"), found);
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            let text = fs::read_to_string(entry.path()).expect("a UTF-8 source file");
            found.push((path, text));
        }
    }
}

/// Whether `c` can be part of an identifier.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The byte offsets at which `text` names `name` whole, and not as part of
/// a longer identifier.
fn places(text: &str, name: &str) -> impl Iterator<Item = usize> {
    text.match_indices(name)
        .map(|(at, _)| at)
        .filter(move |&at| {
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            before.is_none_or(|c| !is_word(c)) && after.is_none_or(|c| !is_word(c))
        })
}

/// Whether `text` names `name` whole, and not as part of a longer
/// identifier.
fn names(text: &str, name: &str) -> bool {
    places(text, name).next().is_some()
}

/// Whether `line` is a comment line, on which no code can stand: it begins
/// with `//` and holds no `"` and no `*/`. A line that begins inside a
/// string or a block comment, and leaves it, holds one of the two, so such
/// a line is a comment from end to end or lies whole inside a string or a
/// block comment.
fn comment(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("//") && !line.contains('"') && !line.contains("*/")
}

/// Whether `door` reads `line` for its name: every line, or for a name that
/// is also a word every line but a comment line.
fn reads(door: &Door, line: &str) -> bool {
    door.lines == Lines::All || !comment(line)
}

/// Whether `text` begins with `name` whole.
fn begins(text: &str, name: &str) -> bool {
    text.strip_prefix(name)
        .is_some_and(|rest| !rest.starts_with(is_word))
}

/// Whether `text` begins with one of the lookups in full, such as
/// `PrePrincipal::Grant`.
fn lookup_in_full(text: &str) -> bool {
    text.strip_prefix(LOOKUP)
        .and_then(|rest| rest.strip_prefix("::"))
        .is_some_and(|variant| LOOKUPS.iter().any(|lookup| begins(variant, lookup)))
}

/// Whether the lookup type, named whole at byte `at` of `line`, is named
/// in one of the forms the search can follow: a lookup in full, the plain
/// import of the type alone, or the store's error step for a failed lookup.
fn plain_lookup(line: &str, at: usize) -> bool {
    let (before, from) = line.split_at(at);
    let import = line
        .trim()
        .strip_prefix("use ")
        .and_then(|path| path.strip_suffix(';'))
        .and_then(|path| path.strip_suffix(LOOKUP))
        .and_then(|path| path.strip_suffix("::"))
        .is_some_and(|path| path.chars().all(|c| is_word(c) || c == ':'));
    let step = before
        .strip_suffix("Step::")
        .is_some_and(|rest| !rest.ends_with(is_word))
        && from
            .strip_prefix(LOOKUP)
            .is_some_and(|rest| rest.starts_with('('));
    lookup_in_full(from) || import || step
}

/// Whether the text `after` the reader's name opens a call whose first
/// argument is a lookup in full, with nothing but spaces and line breaks
/// between, as the formatter may write it.
fn plain_call(after: &str) -> bool {
    after
        .trim_start()
        .strip_prefix('(')
        .is_some_and(|arguments| lookup_in_full(arguments.trim_start()))
}

/// Whether the list entry `entry` covers the module at `path`: a
/// directory, written with a final `/`, covers every module below it, and
/// anything else covers only the module of exactly that path.
fn covers(entry: &str, path: &str) -> bool {
    if entry.ends_with('/') {
        path.starts_with(entry)
    } else {
        path == entry
    }
}

/// Every line of `tree` that names an entry point of `list` in a module
/// the entry does not list, in the order of the tree, then of its lines,
/// then of the list.
fn unlisted(tree: &[Source], list: &[Door]) -> Vec<Use> {
    let mut found = Vec::new();
    for (module, text) in tree {
        for (index, line) in text.lines().enumerate() {
            for door in list {
                let listed = door.callers.iter().any(|caller| covers(caller, module));
                if reads(door, line) && names(line, door.name) && !listed {
                    found.push(Use {
                        module: module.clone(),
                        line: index + 1,
                        door: door.name,
                    });
                }
            }
        }
    }
    found
}

/// Every line of `tree` that names the lookup type or the reader in a form
/// the search cannot follow, in the order of the tree, then of its lines,
/// the type before the reader.
fn indirect(tree: &[Source]) -> Vec<Use> {
    let mut found = Vec::new();
    for (module, text) in tree {
        let mut start = 0;
        for (index, line) in text.split_inclusive('\n').enumerate() {
            let types = places(line, LOOKUP)
                .filter(|&at| !plain_lookup(line, at))
                .map(|_| LOOKUP);
            let calls = places(line, READER)
                .filter(|&at| !plain_call(&text[start + at + READER.len()..]))
                .map(|_| READER);
            found.extend(types.chain(calls).map(|door| Use {
                module: module.clone(),
                line: index + 1,
                door,
            }));
            start += line.len();
        }
    }
    found
}

/// The entry points of `list` that the directory said to declare them,
/// below `crates`, no longer names. An entry whose directory does not
/// exist yet is not one of them.
fn renamed(crates: &Path, list: &[Door]) -> Vec<&'static str> {
    list.iter()
        .filter(|door| {
            let declared = sources(&crates.join(door.declared_in));
            !declared.is_empty()
                && !declared.iter().any(|(_, text)| {
                    text.lines()
                        .any(|line| reads(door, line) && names(line, door.name))
                })
        })
        .map(|door| door.name)
        .collect()
}

/// The directories of `owed`, below `crates`, that hold a source file
/// while no entry of `list` is declared in them.
fn unpaid(crates: &Path, owed: &[&'static str], list: &[Door]) -> Vec<&'static str> {
    owed.iter()
        .copied()
        .filter(|dir| {
            !sources(&crates.join(dir)).is_empty()
                && !list.iter().any(|door| door.declared_in == *dir)
        })
        .collect()
}

/// This crate's directory.
fn server() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Writes `files`, each a path with `/` between its parts and a text, below
/// `root`.
fn write_tree(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = root.join(path);
        let parent = file.parent().expect("a file is in a directory");
        fs::create_dir_all(parent).expect("the directory is created");
        fs::write(&file, text).expect("the file is written");
    }
}

/// A use of `door` on `line` of `module`.
fn used(module: &str, line: usize, door: &'static str) -> Use {
    Use {
        module: module.to_owned(),
        line,
        door,
    }
}

/// A server whose modules keep to the written list: the three modules that
/// own a pre-principal lookup, each making its own as the formatter writes
/// it and opening a second connection as their tests do, the layer asking
/// the policy, start-up replaying the user log, and a handler that reads
/// only under a permit and only writes about the policy.
const LAW_ABIDING: [(&str, &str); 7] = [
    (
        "src/lib.rs",
        "pub mod access;\npub mod handlers;\npub mod session;\n",
    ),
    (
        "src/access/grants.rs",
        "use gunmetal_durable::identity::pre_principal::PrePrincipal;\n\
         \n\
         fn held(store: &IdentityStore) {\n\
         \x20   store.read_pre_principal(PrePrincipal::Grant, &HELD);\n\
         }\n",
    ),
    (
        "src/access/policy.rs",
        "//! The decision is the core's ([`decide`]).\n\
         \n\
         use gunmetal_core::authz::{\n\
         \x20   Action, Context, PrincipalFacts, decide,\n\
         };\n\
         \n\
         fn ask(facts: &PrincipalFacts, context: &Context) {\n\
         \x20   decide(facts, Action::BrowseLibrary, &resource, context);\n\
         \x20   let cache = open_db(&root, &CACHE, pragmas);\n\
         }\n",
    ),
    (
        "src/session/token.rs",
        "use gunmetal_durable::identity::pre_principal::PrePrincipal;\n\
         use gunmetal_fs::sqlite::open_db;\n\
         \n\
         fn session(store: &IdentityStore) {\n\
         \x20   store\n\
         \x20       .read_pre_principal(\n\
         \x20           PrePrincipal::SessionToken,\n\
         \x20           &BY_HASH,\n\
         \x20       );\n\
         \x20   let other = open_db(&root, &IDENTITY, pragmas);\n\
         \x20   let failed = Step::PrePrincipal(PrePrincipal::SessionToken);\n\
         }\n",
    ),
    (
        "src/verifier/handle.rs",
        "fn credential(store: &IdentityStore) {\n\
         \x20   store.read_pre_principal(PrePrincipal::Credential, &BY_ID);\n\
         \x20   let restored = gunmetal_fs::sqlite::open_untrusted(&root, &BACKUP);\n\
         }\n",
    ),
    (
        "src/startup/rebuild.rs",
        "fn rebuild(log: &UserLog) {\n\x20   log.replay_into(stream, &mut builder);\n}\n",
    ),
    (
        "src/handlers/library.rs",
        "//! Classes decide visibility; the policy decides the rest.\n\
         \n\
         /// Lets the layer decide, then reads.\n\
         fn devices(store: &IdentityStore, permit: &Permit) {\n\
         \x20   // The policy will decide.\n\
         \x20   store.read_permitted(permit, &OWN_DEVICES);\n\
         \x20   let unread_pre_principal = read_pre_principal_count();\n\
         \x20   let other = NotPrePrincipal::Grant;\n\
         \x20   let reopened = reopen_db(open_db_file, open_untrusted_copy);\n\
         \x20   let undecided = decided || decide_later;\n\
         }\n",
    ),
];

/// Verifies: SEC-TM-024, SEC-API-010
#[test]
fn a_stand_in_handler_that_calls_a_pre_principal_lookup_fails_the_check() {
    let dir = TempDir::new("storage-access-handler").expect("a temporary directory");
    write_tree(dir.path(), &LAW_ABIDING);
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        []
    );

    // A handler reads every account's grants without asking the policy.
    write_tree(
        dir.path(),
        &[(
            "src/handlers/grants.rs",
            "use gunmetal_durable::identity::pre_principal::PrePrincipal;\n\
             \n\
             fn everyone(state: &AppState) {\n\
             \x20   state.identity.read_pre_principal(PrePrincipal::Grant, &ALL);\n\
             }\n",
        )],
    );
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        [
            used("handlers/grants.rs", 1, "PrePrincipal"),
            used("handlers/grants.rs", 4, "read_pre_principal"),
            used("handlers/grants.rs", 4, "PrePrincipal"),
            used("handlers/grants.rs", 4, "PrePrincipal::Grant"),
        ]
    );
}

/// Verifies: SEC-TM-024
#[test]
fn each_entry_point_may_be_named_only_by_the_modules_listed_for_it() {
    let dir = TempDir::new("storage-access-list").expect("a temporary directory");
    write_tree(dir.path(), &LAW_ABIDING);
    write_tree(
        dir.path(),
        &[
            // The session module may make its own lookup, not the grant
            // read, and a comment that names a lookup counts as a use.
            (
                "src/session/grants.rs",
                "fn grants(store: &IdentityStore) {\n\
                 \x20   // As PrePrincipal::Credential does.\n\
                 \x20   store.read_pre_principal(PrePrincipal::Grant, &HELD);\n\
                 }\n",
            ),
            // A module beside the layer whose name only starts like it.
            (
                "src/accessory/mod.rs",
                "fn lookup() {\n\x20   read_pre_principal();\n}\n",
            ),
            // A file at the top of the crate is in no listed directory.
            ("src/access.rs", "use PrePrincipal as Lookup;\n"),
            // The replay belongs to start-up alone.
            (
                "src/handlers/history.rs",
                "fn history(log: &UserLog) {\n\x20   log.replay_into(stream, &mut page);\n}\n",
            ),
            // Only Rust sources below `src/` are modules of the server.
            ("src/handlers/notes.md", "read_pre_principal\n"),
            ("tests/lookups.rs", "read_pre_principal\n"),
        ],
    );
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        [
            used("access.rs", 1, "PrePrincipal"),
            used("accessory/mod.rs", 2, "read_pre_principal"),
            used("handlers/history.rs", 2, "replay_into"),
            used("session/grants.rs", 2, "PrePrincipal::Credential"),
            used("session/grants.rs", 3, "PrePrincipal::Grant"),
        ]
    );
}

/// Modules outside the three that own a lookup, each opening a database
/// with the SQLite door in another way, and one in a directory whose name
/// only starts like one of the three.
const OPENERS: [(&str, &str); 6] = [
    // A handler opens the identity database and reads every grant.
    (
        "src/handlers/grants.rs",
        "use gunmetal_fs::sqlite::{Pragmas, Query, Synchronous, open_db};\n\
         \n\
         fn everyone(state: &AppState) {\n\
         \x20   let db = open_db(&state.data, &IDENTITY, Pragmas::new(Synchronous::Full));\n\
         \x20   db.query(&Query::new(\"SELECT account, library FROM library_grants\"));\n\
         }\n",
    ),
    // Path-qualified, with no import.
    (
        "src/restore/import.rs",
        "fn import(root: &DataRoot) {\n\
         \x20   gunmetal_fs::sqlite::open_untrusted(root, &IDENTITY);\n\
         }\n",
    ),
    // Under an alias: the import names it, though the call does not.
    (
        "src/handlers/alias.rs",
        "use gunmetal_fs::sqlite::open_db as connect;\n\
         \n\
         fn rows(root: &DataRoot) {\n\
         \x20   connect(root, &IDENTITY, pragmas);\n\
         }\n",
    ),
    // Through a glob import: the call names it.
    (
        "src/handlers/glob.rs",
        "use gunmetal_fs::sqlite::*;\n\
         \n\
         fn rows(root: &DataRoot) {\n\
         \x20   open_untrusted(root, &IDENTITY);\n\
         }\n",
    ),
    // Through a module imported under another name.
    (
        "src/jobs/sweep.rs",
        "use gunmetal_fs::sqlite as door;\n\
         \n\
         fn sweep(root: &DataRoot) {\n\
         \x20   door::open_db(root, &IDENTITY, pragmas);\n\
         }\n",
    ),
    (
        "src/sessions/mod.rs",
        "fn second(root: &DataRoot) {\n\
         \x20   open_db(root, &IDENTITY, pragmas);\n\
         }\n",
    ),
];

/// Verifies: SEC-TM-024, SEC-API-010
#[test]
fn a_module_outside_the_three_that_opens_a_database_itself_fails_the_check() {
    let dir = TempDir::new("storage-access-openers").expect("a temporary directory");
    write_tree(dir.path(), &LAW_ABIDING);
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        []
    );
    write_tree(dir.path(), &OPENERS);
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        [
            used("handlers/alias.rs", 1, "open_db"),
            used("handlers/glob.rs", 4, "open_untrusted"),
            used("handlers/grants.rs", 1, "open_db"),
            used("handlers/grants.rs", 4, "open_db"),
            used("jobs/sweep.rs", 4, "open_db"),
            used("restore/import.rs", 2, "open_untrusted"),
            used("sessions/mod.rs", 2, "open_db"),
        ]
    );
}

/// Modules other than the layer that call the policy function themselves,
/// each in another way, and two lines that begin like a comment but leave
/// a string or a block comment, so that code stands on them.
const MINTERS: [(&str, &str); 7] = [
    // A handler mints its own permit from facts it fills in, importing the
    // function in a group the formatter broke over lines.
    (
        "src/handlers/mint.rs",
        "use gunmetal_core::authz::{\n\
         \x20   Action, Context, PrincipalFacts,\n\
         \x20   decide,\n\
         };\n\
         \n\
         fn mint(facts: &PrincipalFacts, context: &Context) {\n\
         \x20   let permit = decide(facts, Action::BrowseLibrary, &resource, context);\n\
         }\n",
    ),
    // Path-qualified, with no import.
    (
        "src/handlers/path.rs",
        "fn mint() {\n\
         \x20   gunmetal_core::authz::decide(&facts, action, &resource, &context);\n\
         }\n",
    ),
    // Two lines that begin with `//` and yet hold code: one leaves a string
    // and one leaves a block comment.
    (
        "src/handlers/hidden.rs",
        "fn hidden() {\n\
         \x20   let note = (\"the policy\n\
         // \", decide(&facts, action, &resource, &context));\n\
         \x20   /*\n\
         // */ decide(&facts, action, &resource, &context);\n\
         }\n",
    ),
    // Under an alias.
    (
        "src/jobs/alias.rs",
        "use gunmetal_core::authz::decide as ask_policy;\n",
    ),
    // As a value, after a glob import.
    (
        "src/jobs/glob.rs",
        "use gunmetal_core::authz::*;\n\
         \n\
         fn later() {\n\
         \x20   let policy = decide;\n\
         }\n",
    ),
    // Through the core's module of the same name, which counts as naming
    // the function: the types are imported from `gunmetal_core::authz`.
    (
        "src/jobs/types.rs",
        "use gunmetal_core::authz::decide::Permit;\n",
    ),
    // The session module may make its lookup, not ask the policy itself.
    (
        "src/session/shortcut.rs",
        "fn shortcut() {\n\
         \x20   decide(&facts, action, &resource, &context);\n\
         }\n",
    ),
];

/// Verifies: SEC-TM-024, SEC-API-010
#[test]
fn a_module_outside_the_layer_that_calls_the_policy_itself_fails_the_check() {
    let dir = TempDir::new("storage-access-minters").expect("a temporary directory");
    write_tree(dir.path(), &LAW_ABIDING);
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        []
    );
    write_tree(dir.path(), &MINTERS);
    assert_eq!(
        unlisted(&sources(&dir.path().join("src")), &WRITTEN_LIST),
        [
            used("handlers/hidden.rs", 3, "decide"),
            used("handlers/hidden.rs", 5, "decide"),
            used("handlers/mint.rs", 3, "decide"),
            used("handlers/mint.rs", 7, "decide"),
            used("handlers/path.rs", 2, "decide"),
            used("jobs/alias.rs", 1, "decide"),
            used("jobs/glob.rs", 4, "decide"),
            used("jobs/types.rs", 1, "decide"),
            used("session/shortcut.rs", 2, "decide"),
        ]
    );
}

/// Modules of the directories that may name the lookup type, each naming
/// it or calling the reader in a form the search cannot follow.
const ROUNDABOUT: [(&str, &str); 7] = [
    // A group import of a lookup that is not the module's own, and a call
    // with it.
    (
        "src/session/braces.rs",
        "use gunmetal_durable::identity::pre_principal::PrePrincipal::{Grant};\n\
         \n\
         fn grants(store: &IdentityStore) {\n\
         \x20   store.read_pre_principal(Grant, &HELD);\n\
         }\n",
    ),
    // A glob import of the lookups.
    (
        "src/session/glob.rs",
        "use gunmetal_durable::identity::pre_principal::PrePrincipal::*;\n",
    ),
    // An aliased import, and a call through the alias.
    (
        "src/verifier/alias.rs",
        "use gunmetal_durable::identity::pre_principal::PrePrincipal as Lookup;\n\
         \n\
         fn grants(store: &IdentityStore) {\n\
         \x20   store.read_pre_principal(Lookup::Grant, &HELD);\n\
         }\n",
    ),
    // A type alias, and an impl in which `Self::Grant` would name a lookup.
    (
        "src/verifier/typed.rs",
        "type Lookup = PrePrincipal;\n\nimpl Pick for PrePrincipal {}\n",
    ),
    // A lookup that reaches the call as a value: a parameter of the type,
    // and one taken out of a failed read's step, which may be named.
    (
        "src/session/value.rs",
        "fn read(store: &IdentityStore, lookup: PrePrincipal) {\n\
         \x20   store.read_pre_principal(lookup, &HELD);\n\
         }\n\
         \n\
         fn again(store: &IdentityStore, failed: Step) {\n\
         \x20   if let Step::PrePrincipal(lookup) = failed {\n\
         \x20       store.read_pre_principal(lookup, &HELD);\n\
         \x20   }\n\
         }\n",
    ),
    // A lookup a macro completes, and the reader taken as a function.
    (
        "src/access/macro.rs",
        "macro_rules! lookup {\n\
         \x20   ($which:ident) => {\n\
         \x20       PrePrincipal::$which\n\
         \x20   };\n\
         }\n\
         \n\
         fn reader() {\n\
         \x20   let read = IdentityStore::read_pre_principal;\n\
         }\n",
    ),
    // A step of another type, whose name only ends like the store's.
    (
        "src/access/steps.rs",
        "fn step() {\n\
         \x20   let mine = IdentityStep::PrePrincipal(PrePrincipal::Grant);\n\
         }\n",
    ),
];

/// Verifies: SEC-TM-024
#[test]
fn a_lookup_named_or_passed_other_than_in_full_fails_the_check() {
    let dir = TempDir::new("storage-access-roundabout").expect("a temporary directory");
    write_tree(dir.path(), &LAW_ABIDING);
    assert_eq!(indirect(&sources(&dir.path().join("src"))), []);
    write_tree(dir.path(), &ROUNDABOUT);
    assert_eq!(
        indirect(&sources(&dir.path().join("src"))),
        [
            used("access/macro.rs", 3, "PrePrincipal"),
            used("access/macro.rs", 8, "read_pre_principal"),
            used("access/steps.rs", 2, "PrePrincipal"),
            used("session/braces.rs", 1, "PrePrincipal"),
            used("session/braces.rs", 4, "read_pre_principal"),
            used("session/glob.rs", 1, "PrePrincipal"),
            used("session/value.rs", 1, "PrePrincipal"),
            used("session/value.rs", 2, "read_pre_principal"),
            used("session/value.rs", 7, "read_pre_principal"),
            used("verifier/alias.rs", 1, "PrePrincipal"),
            used("verifier/alias.rs", 4, "read_pre_principal"),
            used("verifier/typed.rs", 1, "PrePrincipal"),
            used("verifier/typed.rs", 3, "PrePrincipal"),
        ]
    );
}

#[test]
fn a_tree_is_read_as_its_rust_sources_by_path_in_order() {
    let dir = TempDir::new("storage-access-tree").expect("a temporary directory");
    write_tree(
        dir.path(),
        &[
            ("src/zeta.rs", "z\n"),
            ("src/alpha/mod.rs", "a\n"),
            ("src/alpha/deep/leaf.rs", "l\n"),
            ("src/alpha/notes.txt", "n\n"),
            ("src/rs", "not a source\n"),
        ],
    );
    let read = |path: &str, text: &str| (path.to_owned(), text.to_owned());
    assert_eq!(
        sources(&dir.path().join("src")),
        [
            read("alpha/deep/leaf.rs", "l\n"),
            read("alpha/mod.rs", "a\n"),
            read("zeta.rs", "z\n"),
        ]
    );
    assert_eq!(sources(&dir.path().join("missing")), []);
}

#[test]
fn a_list_entry_covers_the_modules_of_a_directory_or_one_file() {
    let covered = |entry: &str, paths: [&str; 4]| paths.map(|path| covers(entry, path));
    assert_eq!(
        covered(
            "access/",
            [
                "access/grants.rs",
                "access/deep/mod.rs",
                "access.rs",
                "accessory/mod.rs"
            ]
        ),
        [true, true, false, false]
    );
    assert_eq!(
        covered(
            "app.rs",
            ["app.rs", "apps.rs", "app.rs/mod.rs", "nested/app.rs"]
        ),
        [true, false, false, false]
    );
}

#[test]
fn an_entry_point_that_its_declaring_directory_no_longer_names_fails_the_check() {
    let dir = TempDir::new("storage-access-renamed").expect("a temporary directory");
    let list = [
        Door {
            name: "read_pre_principal",
            declared_in: "durable/src/identity",
            callers: &["access/"],
            lines: Lines::All,
        },
        Door {
            name: "replay_into",
            declared_in: "durable/src/userlog",
            callers: &["startup/"],
            lines: Lines::All,
        },
        Door {
            name: "BrokenAt",
            declared_in: "durable/src/audit",
            callers: &["audit_cli/"],
            lines: Lines::All,
        },
        Door {
            name: "decide",
            declared_in: "core/src/authz",
            callers: &["access/"],
            lines: Lines::Code,
        },
    ];
    // The identity store declares its lookup. The user log exists and
    // calls its replay something else. The audit log is not built yet. The
    // policy's directory names it only on a comment line, which is not
    // read for a name that is also a word.
    write_tree(
        dir.path(),
        &[
            (
                "durable/src/identity/store.rs",
                "pub fn read_pre_principal() {}\n",
            ),
            (
                "durable/src/userlog/log.rs",
                "pub fn replay_into_builders() {}\n",
            ),
            ("core/src/authz/mod.rs", "//! The policy will decide.\n"),
        ],
    );
    assert_eq!(renamed(dir.path(), &list), ["replay_into", "decide"]);
    write_tree(
        dir.path(),
        &[
            ("durable/src/userlog/replay.rs", "pub fn replay_into() {}\n"),
            ("core/src/authz/decide.rs", "pub fn decide() {}\n"),
        ],
    );
    assert_eq!(renamed(dir.path(), &list), [""; 0]);
}

#[test]
fn a_store_that_owes_the_list_an_entry_fails_the_check_once_it_is_built() {
    let dir = TempDir::new("storage-access-owed").expect("a temporary directory");
    let owed = ["durable/src/audit", "durable/src/userlog"];
    let list = [Door {
        name: "replay_into",
        declared_in: "durable/src/userlog",
        callers: &["startup/"],
        lines: Lines::All,
    }];
    // Neither store holds a source file yet.
    write_tree(dir.path(), &[("durable/src/audit/notes.md", "verify\n")]);
    assert_eq!(unpaid(dir.path(), &owed, &list), [""; 0]);
    // Both are built, and only the user log has an entry.
    write_tree(
        dir.path(),
        &[
            ("durable/src/userlog/log.rs", "pub fn replay_into() {}\n"),
            ("durable/src/audit/log.rs", "pub fn verify() {}\n"),
        ],
    );
    assert_eq!(unpaid(dir.path(), &owed, &list), ["durable/src/audit"]);
}

/// Verifies: SEC-TM-024, SEC-API-010
#[test]
fn no_module_of_the_server_goes_round_the_permit_outside_the_written_list() {
    let tree = sources(&server().join("src"));
    // The walk found the crate, and the search sees the layer's own grant
    // read and, on its code lines, its own call of the policy, so an empty
    // result is not an empty search.
    let module = |path: &str| tree.iter().find(|(module, _)| module == path);
    assert!(module("lib.rs").is_some());
    assert!(module("access/grants.rs").is_some_and(|(_, text)| names(text, "read_pre_principal")));
    let policy_for_nobody = [Door {
        name: "decide",
        declared_in: "gunmetal-core/src/authz",
        callers: &[],
        lines: Lines::Code,
    }];
    assert!(
        unlisted(&tree, &policy_for_nobody)
            .iter()
            .any(|found| found.module == "access/policy.rs")
    );
    assert_eq!(unlisted(&tree, &WRITTEN_LIST), []);
    assert_eq!(
        indirect(&tree),
        [],
        "name the lookup type only in full (`PrePrincipal::SessionToken`), \
         by its plain import or as the store's error step, and give the reader \
         a lookup in full"
    );
    // Every entry point on the list is still declared where the list says,
    // and no store that owes the list an entry is built without one.
    let crates = server().join("..");
    assert_eq!(renamed(&crates, &WRITTEN_LIST), [""; 0]);
    assert_eq!(
        unpaid(&crates, &OWED, &WRITTEN_LIST),
        [""; 0],
        "the audit log owes the written list its verifier and its append \
         (WP-069): add an entry for each, declared in its directory, with \
         fixtures for both"
    );
}
