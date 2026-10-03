//! Replays every committed seed through its registered harness on stable
//! Rust, so the gate runs the whole corpus of every harness, including the
//! reproducers of past findings (SEC-MED-028).
//!
//! Each harness also has its own `tests/<harness>_corpus.rs`, which pins
//! every seed's bytes and the exact outcome the harness reports for it. This
//! file holds the checks that apply to every harness at once, so a harness
//! added later is replayed without anyone remembering to add it here.

use std::fs;
use std::path::{Path, PathBuf};

use gunmetal_fuzz::registry::{self, Harness};

/// The committed corpus: one directory per harness, named after it.
fn seeds_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds")
}

/// The names of the entries directly inside `dir`, sorted.
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("directory is readable")
        .map(|entry| {
            entry
                .expect("directory entry is readable")
                .file_name()
                .into_string()
                .expect("names are UTF-8")
        })
        .collect();
    names.sort();
    names
}

/// The registered names, in registry order.
fn registered() -> Vec<&'static str> {
    registry::all().iter().map(|harness| harness.name).collect()
}

/// Verifies: SEC-MED-028
#[test]
fn every_harness_replays_every_seed_in_its_directory() {
    for &Harness { name, run } in registry::all() {
        let dir = seeds_root().join(name);
        let seeds = names_in(&dir);
        assert!(!seeds.is_empty(), "harness {name} has no seeds");
        for seed in seeds {
            let bytes = fs::read(dir.join(&seed)).expect("seed is readable");
            run(&bytes);
        }
    }
}

/// Verifies: SEC-MED-027, SEC-MED-028
#[test]
fn every_seed_directory_belongs_to_a_registered_harness() {
    assert_eq!(names_in(&seeds_root()), registered());
}

#[test]
fn the_registry_is_sorted_with_no_duplicates() {
    let names = registered();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}
