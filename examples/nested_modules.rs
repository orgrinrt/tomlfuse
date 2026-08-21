//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! A toml's hierarchy becomes a module hierarchy, and `**` reaches through it.
//!
//! `*` matches one segment and `**` matches any number, so `**` from the root takes the
//! whole file and reproduces its shape: a table becomes a `pub mod`, a key becomes a
//! `pub const`, and nesting is nesting.
//!
//! The kebab-case a toml is comfortable with is not a Rust identifier, so `max-body-bytes`
//! would arrive as `MAX_BODY_BYTES`. Nothing in this file needs it, and it is worth knowing
//! before a rename in the toml quietly renames a constant.
//!
//! Comments come across as documentation. The comment above `[app]` in the toml is what
//! `cargo doc` shows for the `app` module, which is the only reason a comment written for
//! a human editing the toml reaches a human reading the code.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [conf]
    **
}

fn main() {
    println!("{} {}", conf::app::NAME, conf::app::VERSION);
    println!("{}:{}", conf::server::HOST, conf::server::PORT);
    println!("deploy slot {}", conf::server::internal::DEPLOY_SLOT);
    println!("pool of {}", conf::database::POOL_SIZE);
    println!("logging to {:?}", conf::logging::TARGETS);
}
