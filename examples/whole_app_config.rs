//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! One binding that uses the whole pattern language on one file.
//!
//! Every earlier example in this directory shows one thing. This is what they look like
//! together, which is how a real binding is written: a handful of sections, each one taking
//! a subtree, carving out what the binary has no business knowing, and giving a name to the
//! two or three values that would otherwise collide.
//!
//! The sections are the design. A section is a module, so the shape chosen here is the shape
//! the rest of the program sees, and it does not have to be the toml's shape. `runtime`
//! below gathers the server and the database because that is what the binary starts;
//! `observability` gathers logging and telemetry because that is what an operator turns up.
//! Neither grouping exists in the file.
//!
//! Nothing here runs at startup. Every value is a constant in the binary, so a mistake in a
//! pattern is a compile error rather than a panic on a Tuesday, and the toml is a build
//! input: editing it rebuilds what came out of it.

use tomlfuse::file;

file! {
    "examples/app.toml"

    // What the binary calls itself.
    [meta]
    app.*

    // What it starts. Two subtrees under one name, with the operator's own bookkeeping
    // left out, and the two `timeout` keys given names that say which is which.
    [runtime]
    server.*
    database.*
    !server.internal.*
    !server.timeout
    !database.timeout
    alias request_timeout = server.timeout
    alias query_timeout = database.timeout

    // What an operator turns up when something is wrong.
    [observability]
    {logging,telemetry}.*

    // The ceilings, which are checked often enough to be worth their own module.
    [ceilings]
    limits.{max_body_bytes,max_connections}
}

fn banner() -> impl core::fmt::Display {
    struct Banner;
    impl core::fmt::Display for Banner {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{} {}", meta::NAME, meta::VERSION)
        }
    }
    Banner
}

fn main() {
    println!("{}", banner());
    println!();

    println!("listening on {}:{}", runtime::HOST, runtime::PORT);
    println!("  requests give up after {}s", runtime::REQUEST_TIMEOUT);
    println!(
        "  database {}, pool of {}",
        runtime::URL,
        runtime::POOL_SIZE
    );
    println!("  queries give up after {}s", runtime::QUERY_TIMEOUT);
    println!();

    println!(
        "logging {} as {}",
        observability::logging::LEVEL,
        observability::logging::FORMAT
    );
    println!("  to {:?}", observability::logging::TARGETS);
    if observability::telemetry::ENABLED {
        println!(
            "  telemetry to {} at {}",
            observability::telemetry::ENDPOINT,
            observability::telemetry::SAMPLE_RATE
        );
    } else {
        println!("  telemetry off");
    }
    println!();

    println!("refusing bodies over {} bytes", ceilings::MAX_BODY_BYTES);
    println!(
        "refusing more than {} connections",
        ceilings::MAX_CONNECTIONS
    );

    // The subtree that was carved out is not reachable by any name:
    //   runtime::internal::OWNER      does not exist
    //   runtime::TIMEOUT              does not exist, because both were excluded and aliased
    // `tests/ui.rs` holds the cases that prove those two do not compile.
}
