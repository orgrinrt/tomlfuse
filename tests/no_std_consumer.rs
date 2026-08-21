//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! A `#![no_std]` crate with no allocator gets working constants out of this one.
//!
//! The claim is about the expansion, not about this crate, which is a proc macro and runs
//! inside the compiler with `std` on every configuration. So building this crate proves
//! nothing, and `trybuild` proves nothing either: it compiles one configuration of this
//! crate and hands it files, where what is under test is a whole separate crate with its
//! own manifest, its own feature selection and its own `#![no_std]`.
//!
//! What that leaves is writing the consumer out and building it, which is what happens
//! below. Every selection gets the same source, and the source reads a const of each type
//! the expansion can produce, so a type that started needing `std` would take the build
//! down rather than sitting unnoticed behind a `&'static str` that still worked.
//!
//! The control is the half that makes the rest mean anything. A consumer that forgot its
//! `#![no_std]` passes every check here while proving nothing, and nothing about the output
//! would say so. So one consumer is written that is identical except for naming `std`, and
//! it has to *fail*. If it compiles, the attribute is not doing what the other cases assume.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The toml every consumer below reads. One value of each type the expansion can produce.
const FIXTURE: &str = r#"
[surface]
text = "a string"
count = 3
ratio = 1.5
flag = true
list = ["one", "two"]
"#;

/// Writes a consumer crate and builds it against this one at `features`.
///
/// Returns whether it compiled, and what cargo said when it did not. The crate is written
/// under this crate's own target directory, so `cargo clean` takes it with everything else,
/// and it builds into a target directory of its own so it does not fight the outer
/// `cargo test` for the build lock.
fn consumer_compiles(name: &str, features: &[&str], body: &str) -> (bool, String) {
    let root = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/target/no-std-consumers"
    ))
    .join(name);
    fs::create_dir_all(root.join("src")).expect("the consumer directory");
    fs::write(root.join("surface.toml"), FIXTURE).expect("the consumer fixture");

    let features_list = features
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.0.0"
edition = "2021"

[dependencies.tomlfuse]
path = "{crate_dir}"
default-features = false
features = [{features_list}]

[workspace]
"#,
            name = name,
            crate_dir = env!("CARGO_MANIFEST_DIR"),
            features_list = features_list,
        ),
    )
    .expect("the consumer manifest");
    fs::write(root.join("src/lib.rs"), body).expect("the consumer source");

    let output = Command::new(env!("CARGO"))
        .args(["check", "--quiet"])
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("cargo runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Reads one const of every type the expansion produces, under `#![no_std]`.
const USES_EVERY_TYPE: &str = r#"#![no_std]

tomlfuse::file! {
    "surface.toml"
    [surface]
    surface.*
}

pub const TEXT: &str = surface::TEXT;
pub const COUNT: i64 = surface::COUNT;
pub const RATIO: f64 = surface::RATIO;
pub const FLAG: bool = surface::FLAG;
pub const LIST: &[&str] = surface::LIST;
"#;

#[test]
fn a_no_std_consumer_compiles_on_every_selection() {
    for features in [&[][..], &["no_std"][..], &["no_alloc"][..], &["no_std", "no_alloc"][..]] {
        let name = if features.is_empty() { "default".to_string() } else { features.join("_") };
        let (ok, stderr) = consumer_compiles(&format!("nostd_{name}"), features, USES_EVERY_TYPE);
        assert!(
            ok,
            "a `#![no_std]` consumer failed to build against features {features:?}:\n{stderr}"
        );
    }
}

#[test]
fn the_no_std_consumer_really_has_no_std() {
    // Identical to the case above but for the last line, which is the only thing that can
    // account for a difference in the outcome.
    let body = format!("{USES_EVERY_TYPE}\npub fn control() -> std::string::String {{ std::string::String::new() }}\n");
    let (ok, stderr) = consumer_compiles("nostd_control", &["no_std"], &body);
    assert!(
        !ok,
        "the control consumer named `std` and compiled anyway, so `#![no_std]` is not in \
         effect and every other case here proves nothing"
    );
    assert!(
        stderr.contains("unresolved module or unlinked crate `std`")
            || stderr.contains("failed to resolve"),
        "the control failed for some reason other than `std` being absent, which is not the \
         thing it is here to establish:\n{stderr}"
    );
}
