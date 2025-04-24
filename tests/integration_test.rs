//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use tomlfuse::package;
// creates compile-time constants from the closest Cargo.toml file
package! {
    [package]
    package.*
    !package.metadata.*

    [deps]
    dependencies.*
    alias dependencies.syn = syn_dep

    [metadata]
    package.metadata.*
    !package.metadata.defaults.*

    [defaults]
    package.metadata.defaults.*
}

#[test]
fn test_generated_constants() {
    // check basic lookups work
    assert!(!package::AUTHORS.is_empty());
    assert!(!package::EDITION.is_empty());
    assert!(metadata::FOO == "bar");
    assert!(defaults::VALUE == 1);

    // log generated values
    println!("Package authors: {}", package::AUTHORS.join(", "));
    println!("Package edition: {}", package::EDITION);
    println!("Metadata test1: {} (should be \"bar\")", metadata::FOO);
    println!("Metadata test2: {} (should be 1)", defaults::VALUE);

    // check path helpers work
    assert!(CARGO_MANIFEST_DIR.exists());
    assert!(WORKSPACE_ROOT.join("Cargo.toml").exists());
}
