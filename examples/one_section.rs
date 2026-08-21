//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! The smallest thing the crate does: one section of a toml, bound as constants.
//!
//! `file!` takes the path first, then one or more sections. A section opens with its name
//! in brackets and is followed by the patterns whose matches land in it, so `[settings]`
//! plus `app.*` gives a `settings` module holding one const per key under `[app]`.
//!
//! The names are upper-cased because they are constants. The values are baked into the
//! binary at compile time, so nothing here reads a file when it runs.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [settings]
    app.*
}

fn main() {
    println!("{} {}", settings::NAME, settings::VERSION);
    if settings::VERBOSE {
        println!("verbose");
    }
}
