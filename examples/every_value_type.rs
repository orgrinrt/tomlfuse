//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! What each toml type becomes, spelled out by annotating every constant.
//!
//! The annotations are the point. Each one is the type the expansion chose, written by hand
//! so the compiler checks the claim: get one wrong and this stops building. Every one of
//! them is a `core` type, which is what lets a `#![no_std]` crate with no allocator use
//! this. Strings and arrays are `&'static`, pointing into the binary rather than at
//! anything owned.
//!
//! A float is `f64` and an integer is `i64`, because toml has one of each and picking
//! narrower would mean guessing which width the value was meant for.

use tomlfuse::file;

file! {
    "examples/app.toml"

    [text]
    app.name
    logging.format

    [numbers]
    server.port
    telemetry.sample_rate

    [flags]
    telemetry.enabled

    [lists]
    logging.targets
}

fn main() {
    let name: &'static str = text::NAME;
    let format: &'static str = text::FORMAT;
    let port: i64 = numbers::PORT;
    let sample_rate: f64 = numbers::SAMPLE_RATE;
    let enabled: bool = flags::ENABLED;
    let targets: &'static [&'static str] = lists::TARGETS;

    println!("name        &'static str            {name}");
    println!("format      &'static str            {format}");
    println!("port        i64                     {port}");
    println!("sample_rate f64                     {sample_rate}");
    println!("enabled     bool                    {enabled}");
    println!("targets     &'static [&'static str] {targets:?}");
}
