//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! A key bound at the width the macro input names, and left at `i64` or `f64` otherwise.
//!
//! The type is checked by the annotation each test binds the constant to, which does not
//! compile if the constant is of any other type, so a width that was parsed and then ignored
//! fails the build rather than a test run.

use tomlfuse::file;

file! {
    "tests/widths.toml"

    [size]
    size.voxel_pixels: u8
    size.frames: u32
    size.offset: i16
    size.ratio: f32
    size.unsaid
    size.steps: u8

    // A key that goes by another name takes its width from the original's path.
    [renamed]
    alias px = size.voxel_pixels: u16

    // Dashes in the file are underscores in the pattern, as for any other key.
    [tuning]
    tuning.tone_soft: f32

    // Several keys at once, through the glob.
    [both]
    size.{frames,voxel_pixels}: u64
}

#[test]
fn an_integer_binds_at_the_width_named() {
    let _: u8 = size::VOXEL_PIXELS;
    let _: u32 = size::FRAMES;
    let _: i16 = size::OFFSET;
    const { assert!(size::VOXEL_PIXELS == 17) };
    const { assert!(size::FRAMES == 30) };
    const { assert!(size::OFFSET == -12) };
}

#[test]
fn a_float_binds_at_the_width_named() {
    let _: f32 = size::RATIO;
    let _: f32 = tuning::TONE_SOFT;
    assert_eq!(size::RATIO, 0.5_f32);
    assert_eq!(tuning::TONE_SOFT, 0.55_f32);
}

#[test]
fn a_key_with_no_width_is_still_an_i64() {
    let _: i64 = size::UNSAID;
    assert_eq!(size::UNSAID, 7);
}

#[test]
fn the_elements_of_an_array_take_the_width() {
    let steps: &[u8] = size::STEPS;
    assert_eq!(steps, [1, 2, 3]);
}

#[test]
fn an_alias_takes_the_width_given_after_the_original() {
    let _: u16 = renamed::PX;
    const { assert!(renamed::PX == 17) };
}

#[test]
fn one_width_can_cover_several_keys() {
    let _: u64 = both::FRAMES;
    let _: u64 = both::VOXEL_PIXELS;
}
