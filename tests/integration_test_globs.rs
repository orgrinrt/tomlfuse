//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Alternation, `{a,b}`, against `tests/test.toml`.
//!
//! It was an enum variant nothing constructed, a parser that could not read it, and two
//! `unimplemented!()` catch-alls reachable from a match. The `Display` that feeds globset
//! also joined alternatives with `", "`, and a space in a glob is a character to be
//! matched, so `{a, b}` would have looked for a segment beginning with a space.

use tomlfuse::file;

file! {
    "tests/test.toml"

    // Both alternatives resolve, and `debug`, which sits beside them under `config`,
    // does not.
    [either]
    config.{logging,settings}.*

    // Alternation on the last segment, which is the shape a config binding usually wants.
    [leaf]
    config.settings.{timeout,retries}

    // One alternative is still an alternation.
    [single]
    config.{debug}
}

#[test]
fn both_alternatives_resolve() {
    // The one that matters. An alternation matching only its first branch would still
    // pass a test that checked one of them, which is why both are named here.
    assert_eq!(either::logging::LEVEL, "info");
    assert_eq!(either::logging::FORMAT, "json");
    assert_eq!(either::settings::TIMEOUT, 500);
    assert_eq!(either::settings::RETRIES, 3);
}

#[test]
fn alternation_on_the_last_segment() {
    assert_eq!(leaf::TIMEOUT, 500);
    assert_eq!(leaf::RETRIES, 3);
}

#[test]
fn one_alternative_behaves_as_the_bare_segment_would() {
    const { assert!(!single::DEBUG) };
}

#[test]
fn the_alternatives_keep_their_own_names() {
    // `config.settings.*` would flatten `settings` away, because a leading run of literal
    // segments is what gets stripped. An alternation is not one, so both branches stay
    // named, and they have to: `logging.level` and `settings.level` would otherwise
    // collide on `LEVEL`.
    assert_eq!(either::logging::LEVEL, "info");
    assert_ne!(either::settings::TIMEOUT, 0);
}
