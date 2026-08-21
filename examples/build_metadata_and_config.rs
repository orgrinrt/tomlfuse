//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Both macros in one binary: the crate's own manifest, and a file beside it.
//!
//! `package!` finds the `Cargo.toml` this crate is built from, so it needs no path and
//! reads whatever the manifest says at the moment of the build. That covers the questions a
//! binary asks about itself, which is what `--version` prints and what a bug report needs:
//! its own version, its licence, what it was compiled against.
//!
//! `file!` takes a path and covers everything else. The two sit side by side because they
//! answer different questions, and neither is a substitute for the other. There is a
//! `workspace!` too, which reads the workspace manifest rather than the crate's.
//!
//! What makes this worth doing at compile time rather than with `env!` is the dependency
//! versions. `env!("CARGO_PKG_VERSION")` gives the crate's own and nothing else; the
//! manifest holds every version the build resolved against, and a bug report that names them
//! is one nobody has to ask a follow-up question about.

use tomlfuse::{file, package};

package! {
    // What this crate calls itself, minus the metadata table, which is where a
    // project keeps things that are nobody else's business.
    [pkg]
    package.*
    !package.metadata.*

    // What it was compiled against.
    [deps]
    dependencies.*
}

file! {
    "examples/app.toml"

    [conf]
    app.*
    logging.*
}

fn main() {
    println!("{} {}", pkg::NAME, pkg::VERSION);
    println!("  {}", pkg::DESCRIPTION.trim());
    println!("  licensed {}, built for rust {}", pkg::LICENSE, pkg::RUST_VERSION);
    println!();

    println!("built against");
    println!("  syn      {}", deps::syn::VERSION);
    println!("  quote    {}", deps::quote::VERSION);
    println!("  globset  {}", deps::GLOBSET);
    println!();

    // The manifest is this crate's; the file is the application's. Same syntax, and the
    // constants sit in the same binary, which is the whole reason both macros exist.
    println!("configured as {} {}", conf::NAME, conf::VERSION);
    println!("  logging {} to {:?}", conf::LEVEL, conf::TARGETS);
}
