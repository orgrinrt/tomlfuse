//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Matching any one of several names, with `{a,b}`.
//!
//! The braces hold alternatives separated by commas, and the pattern matches a path that
//! takes any one of them at that position. `{logging,telemetry}.*` is the two patterns it
//! reads as, written once.
//!
//! It works on the last segment too, which is the shape a binding usually wants: pick two
//! keys out of a section without taking the section.
//!
//! Commas rather than pipes, and no spaces. A space inside the braces is a character to be
//! matched, so `{a, b}` looks for a name that begins with one.

use tomlfuse::file;

file! {
    "examples/app.toml"

    // Two whole sections, and nothing else. The database is not observability.
    [observability]
    {logging,telemetry}.*

    // Two keys out of one section.
    [ceilings]
    limits.{max_body_bytes,max_connections}
}

fn main() {
    println!("logging at {} in {}", observability::logging::LEVEL, observability::logging::FORMAT);
    println!("telemetry enabled: {}", observability::telemetry::ENABLED);
    println!("body limit {} bytes", ceilings::MAX_BODY_BYTES);
    println!("connection limit {}", ceilings::MAX_CONNECTIONS);
}
