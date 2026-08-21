//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! The bindings a pattern is supposed to leave out are absent, and absent is a build error.
//!
//! A binding that excludes something has to remove the name, not merely stop populating it.
//! The difference is invisible from inside a passing test: a const that resolves to an empty
//! value and a const that does not exist both let `assert!(x.is_empty())` pass, and only one
//! of them is what an exclusion means. So the check is that naming the thing does not
//! compile, and the only way to hold that is a case that has to fail.
//!
//! Each case also pins the error, so a name disappearing for the wrong reason is a failure
//! rather than a pass. `an_excluded_key_is_not_reachable` would still fail to compile if the
//! macro stopped emitting anything at all, and the recorded stderr is what tells those two
//! apart.
//!
//! # The fixture, and why it is copied
//!
//! `trybuild` builds each case inside a scratch crate under `target/`, and a path handed to
//! `file!` resolves against that crate rather than this one. Baking an absolute path into
//! the case source would fix it and would put this machine's directory layout in a committed
//! file and in the recorded stderr beside it.
//!
//! So the fixture is copied to where the scratch crate resolves from, and the cases keep the
//! ordinary relative path a consumer would write. If `trybuild` ever moves that directory
//! the copy lands somewhere unread, the macro reports the file missing, and every case fails
//! with that message, which is a legible failure rather than a quiet pass.

use std::fs;
use std::path::PathBuf;

/// The cases that must not compile, counted against the directory rather than against a list
/// somebody has to remember to extend.
const EXPECTED_FAILING: usize = 2;

/// The cases that must. There is one, and it holds the other half of what an exclusion is
/// for: a binding has to be usable from an ordinary crate, and this one denies a lint that
/// the generated code used to trip on every item it emitted.
const EXPECTED_PASSING: usize = 1;

/// Where `trybuild` puts the crate it builds the cases in. It declares its own `[workspace]`,
/// so it is both the workspace root and the crate root that `file!` falls back to.
///
/// Under the resolved target directory rather than a literal `target/`. `CARGO_TARGET_DIR`
/// moves it and CI routinely sets it, and this used to hardcode the name: the fixture then
/// landed somewhere the scratch crate never reads and every case failed with the macro
/// correctly reporting the file missing.
fn scratch_crate() -> PathBuf {
    scratch_crate_under(&target_dir())
}

/// The same, from a build directory handed in rather than read from the environment.
///
/// Split out so the test below can check the property without mutating `CARGO_TARGET_DIR`.
/// Tests in one binary run on threads of one process, so setting it would race whatever else
/// is reading the environment, which is why `set_var` is unsafe. An earlier version of that
/// test did exactly that and the suite failed intermittently.
fn scratch_crate_under(target: &std::path::Path) -> PathBuf {
    target.join("tests/trybuild/tomlfuse")
}

/// Where cargo is putting build output for this run.
fn target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

#[test]
fn a_binding_leaves_out_what_the_pattern_excludes() {
    let ui = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ui"));

    let count = |dir: &PathBuf| {
        fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|e| e == "rs"))
            .count()
    };
    let failing = count(&ui);
    let passing = count(&ui.join("pass"));
    assert_eq!(
        (failing, passing),
        (EXPECTED_FAILING, EXPECTED_PASSING),
        "tests/ui holds {failing} failing and {passing} passing cases, against {EXPECTED_FAILING} \
         and {EXPECTED_PASSING} expected. A case added without updating the count runs but is \
         not accounted for, and one deleted stops running with nothing to say so."
    );

    let destination = scratch_crate().join("tests/ui");
    fs::create_dir_all(&destination).expect("the scratch fixture directory");
    fs::copy(ui.join("fixture.toml"), destination.join("fixture.toml"))
        .expect("the fixture copies into the scratch crate");

    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    cases.pass("tests/ui/pass/*.rs");
}

/// The scratch crate follows the build directory, wherever cargo put it.
///
/// This is the property that broke, and it is worth stating rather than the one it looks like.
/// `scratch_crate` hardcoded a literal `target/`, so with `CARGO_TARGET_DIR` set, which CI
/// routinely does, the fixture was copied somewhere trybuild's crate never reads and every
/// case above failed with the macro correctly reporting the file missing. The macro was right
/// and the setup was wrong.
#[test]
fn the_scratch_crate_follows_the_build_directory() {
    let elsewhere = PathBuf::from("/somewhere/not_called_target");
    assert!(
        scratch_crate_under(&elsewhere).starts_with(&elsewhere),
        "the scratch crate sits under the build directory it was given",
    );
    // The control. Without it a function ignoring its argument and returning a constant would
    // satisfy the assertion above whenever that constant happened to match.
    let other = PathBuf::from("/a/different/place");
    assert!(scratch_crate_under(&other).starts_with(&other));
    assert_ne!(scratch_crate_under(&elsewhere), scratch_crate_under(&other));
}
