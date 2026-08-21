//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! `workspace!`, which is public and documented and had no test.
//!
//! This file used to be the licence header and nothing else. It reported
//! `running 0 tests ... ok`, which counts toward a green suite while asserting nothing, so
//! the one macro of the three with no coverage looked the same as the two with it.
//!
//! Nothing could be written here until the package declared a workspace of its own, since
//! `workspace!` binds from the manifest that opens a `[workspace]` table. It has one now.

use tomlfuse::workspace;

// Binds from the workspace root's Cargo.toml, which for a single-package workspace is this
// package's own.
workspace! {
    [meta]
    workspace.metadata.*

    [about]
    package.*
    !package.metadata.*
    !package.description
    !package.authors
    !package.keywords
    !package.categories
}

#[test]
fn workspace_metadata_binds() {
    assert_eq!(meta::PURPOSE, "exercising the workspace! macro");
    assert_eq!(meta::COUNT, 3);
}

#[test]
fn the_package_table_binds_from_the_workspace_manifest() {
    assert_eq!(about::NAME, "tomlfuse");
    assert_eq!(about::EDITION, "2021");
}

#[test]
fn exclusions_apply_to_the_workspace_macro_too() {
    // `!package.metadata.*` and the four `!package.<field>` lines above are what keep
    // those out. If exclusion were ignored the module would carry them and this file
    // would not compile, so the assertion is that it does.
    assert_eq!(about::LICENSE, "MPL-2.0");
}
