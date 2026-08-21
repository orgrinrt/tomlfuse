//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Giving one value a name of its own, with `alias`.
//!
//! Two sections here have a `timeout` and they mean different things. Binding both under
//! one section would put two `TIMEOUT` constants in one module, and the second would
//! shadow the first, so the binary would silently use the wrong one.
//!
//! An alias names a single path and says what to call it. The name is the constant's, so
//! `alias request_timeout = server.timeout` produces `REQUEST_TIMEOUT`, and the two
//! values sit beside each other without either of them being reachable by accident.
//!
//! Aliases take one path rather than a pattern. A pattern alias would have to say what
//! happens to the part the star matched, and that is not settled.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [timeouts]
    alias request_timeout = server.timeout
    alias query_timeout = database.timeout
}

fn main() {
    println!("a request gives up after {}s", timeouts::REQUEST_TIMEOUT);
    println!("a query gives up after {}s", timeouts::QUERY_TIMEOUT);
}
