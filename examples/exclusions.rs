//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Taking a whole subtree and then carving pieces out of it, with `!`.
//!
//! `server.*` on its own would bind the operator's deploy slot and owner alongside the
//! host and port. They are in the toml because somebody's tooling puts them there, and a
//! binary that names them has taken a dependency on that tooling by accident.
//!
//! An exclusion is a pattern like any other with `!` in front, and it applies to the
//! section it appears in. Order does not matter: everything matched is collected, then
//! everything excluded is removed.
//!
//! What is excluded is gone rather than hidden. Naming `net::internal::OWNER` below is a
//! compile error, not an empty value at runtime, which is the difference between a binding
//! that documents its surface and one that merely narrows it.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [net]
    server.*
    !server.internal.*
}

fn main() {
    println!("{}:{}", net::HOST, net::PORT);
    println!("timeout {}s", net::TIMEOUT);

    // net::internal::OWNER does not exist. Uncommenting this does not compile.
    // println!("{}", net::internal::OWNER);
}
