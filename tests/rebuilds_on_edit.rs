//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Editing the bound toml rebuilds the crate that binds it.
//!
//! A procedural macro reads files behind cargo's back. Nothing in an invocation says its
//! output depends on anything but the source file the invocation sits in, so a macro that
//! reads a toml and does not register it leaves the previous constants in the binary when
//! the toml is edited, and the build reports success. Version 0.0.4 did exactly that.
//!
//! The only test that tells a tracked file from an untracked one is the whole sequence run
//! for real: build a crate that binds a toml, edit the toml, build again, and see whether
//! the constant that comes out is the new one. Calling the macro in this process cannot
//! show it, because the staleness lives in cargo's fingerprint and not in the expansion.
//!
//! The consumer is a separate crate under this crate's `target/`, for the reasons
//! `no_std_consumer.rs` gives, and it builds into a target directory of its own so it does
//! not wait on the lock the outer `cargo test` holds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

/// Where the consumer lives. Under the build directory, so `cargo clean` takes it too.
fn consumer_root(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/rebuild-consumers")).join(name)
}

/// Writes the consumer, which prints the one constant it binds.
///
/// The toml is written separately, by [`write_toml`], because that is the file the test
/// edits and everything else here stays put.
fn write_consumer(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("the consumer directory");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "rebuild_consumer"
version = "0.0.0"
edition = "2021"

[dependencies.tomlfuse]
path = "{crate_dir}"

[workspace]
"#,
            crate_dir = env!("CARGO_MANIFEST_DIR"),
        ),
    )
    .expect("the consumer manifest");
    fs::write(
        root.join("src/main.rs"),
        r#"tomlfuse::file! {
    "value.toml"
    [bound]
    settings.*
}

fn main() {
    println!("{}", bound::VALUE);
}
"#,
    )
    .expect("the consumer source");
}

fn write_toml(root: &Path, value: i64) {
    fs::write(root.join("value.toml"), format!("[settings]\nvalue = {value}\n"))
        .expect("the consumer toml");
}

/// Builds and runs the consumer, returning what it printed.
fn build_and_run(root: &Path) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "the consumer failed to build:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn editing_the_bound_toml_changes_the_constant_the_next_build_produces() {
    let root = consumer_root("edit_the_toml");
    write_consumer(&root);

    write_toml(&root, 1);
    let before = build_and_run(&root);

    // Cargo decides a build is stale by comparing the modification time of each input in
    // the dependency info against the output's. A filesystem that stamps to the second
    // would give the edit the same time as the build before it, and the edit would read as
    // older than nothing. A second's wait is what separates "the macro did not track the
    // file" from "the clock could not tell the two builds apart".
    sleep(Duration::from_millis(1100));

    write_toml(&root, 2);
    let after = build_and_run(&root);

    assert_eq!(before, "1", "the first build did not bind the first value");
    assert_eq!(
        after, "2",
        "the toml was edited from 1 to 2 and the next build still printed {after}. The \
         generated code is not registering the file with cargo, so the old constant stayed \
         in the binary and the build reported success"
    );
}
