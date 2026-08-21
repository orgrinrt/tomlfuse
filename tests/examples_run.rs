//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Every example builds and runs, and prints something.
//!
//! An example is documentation that can be wrong, and the pattern language is exactly the
//! kind of surface where it goes wrong quietly: a pattern that stops matching produces a
//! module with fewer constants in it, and the example stops compiling. That is the good
//! case. The bad one is an example nobody runs, which stays in the repository claiming a
//! syntax the crate no longer parses.
//!
//! Building is most of the check here, because every value an example prints is a constant
//! resolved at compile time. Running is still worth doing: it catches an example that
//! compiles and then panics, and the empty-output assertion catches one whose `main` was
//! left as a stub.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The examples, read from the directory rather than listed, so one added without being
/// named here still runs.
fn examples() -> Vec<String> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/examples"));
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("examples/ exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .filter_map(|path| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    names.sort();
    names
}

#[test]
fn every_example_runs_and_says_something() {
    let names = examples();
    assert!(
        names.len() >= 8,
        "found {} examples, which is fewer than the directory is supposed to hold. An \
         example deleted stops being checked, and nothing else would report it.",
        names.len()
    );

    for name in &names {
        let output = Command::new(env!("CARGO"))
            .args(["run", "--quiet", "--example", name])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env(
                "CARGO_TARGET_DIR",
                concat!(env!("CARGO_MANIFEST_DIR"), "/target/examples-run"),
            )
            .output()
            .expect("cargo runs");

        assert!(
            output.status.success(),
            "`cargo run --example {name}` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "`{name}` ran and printed nothing, so whatever it was showing is not being shown"
        );
    }
}
