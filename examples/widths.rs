//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Saying what type a number is, with `: type` after the pattern.
//!
//! A toml integer binds as `i64` and a float as `f64`, which is rarely what the code that
//! reads them wants: a port is a `u16`, a sample rate is an `f32`. Without a width every
//! consumer writes a narrowing function per constant. With one, the constant comes out as
//! the type and the build checks that the value fits.
//!
//! Try changing `server.port` in `examples/app.toml` to 70000. The build stops and names the
//! key, rather than wrapping to a port that is not the one in the file. The same goes for a
//! pattern too broad for its width: `server.*: u16` would refuse `server.host`, which is a
//! string.
//!
//! An array takes the width too, and becomes a fixed array whose lengths are the file's, so a
//! table of colours is the `[[u8; 3]; 4]` an API taking one asks for.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [net]
    server.port: u16
    server.timeout: u32

    [sampling]
    telemetry.sample_rate: f32

    // A width can follow an alias too, and is the width of the original key.
    [shortcuts]
    alias pool = database.pool_size: u8

    // And one width can cover several keys.
    [limits]
    limits.*: u64

    // Rows of three channels come out as `[[u8; 3]; 4]`, with both lengths read from the file.
    // A fifth row in the file makes it a `[[u8; 3]; 5]`, and a row with two channels is an
    // error, since a fixed array has one length.
}

file! {
    "examples/palette.toml"

    [colours]
    palette.world: u8
}

fn main() {
    // The annotations are the point: each of these is a compile error if the constant is of
    // any other type.
    let port: u16 = net::PORT;
    let timeout: u32 = net::TIMEOUT;
    let sample_rate: f32 = sampling::SAMPLE_RATE;
    let pool: u8 = shortcuts::POOL;
    let body: u64 = limits::MAX_BODY_BYTES;
    let world: [[u8; 3]; 4] = colours::WORLD;

    println!("port {port}, timeout {timeout}s");
    println!("sampling {sample_rate}");
    println!("pool of {pool}");
    println!(
        "bodies up to {body} bytes, {} connections",
        limits::MAX_CONNECTIONS
    );
    for (i, [r, g, b]) in world.into_iter().enumerate() {
        println!("colour {i} is #{r:02x}{g:02x}{b:02x}");
    }
}
