//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! The shapes of value that do bind, and what they bind as.
//!
//! The ones that cannot bind are in `tests/ui/`, where they have to fail to compile. This
//! is the other half: refusing a mixed array must not take an array of arrays with it, nor
//! an empty one, which has no element to say what kind it holds.

use tomlfuse::file;

file! {
    "tests/values.toml"

    [shapes]
    shapes.*
}

#[test]
fn every_scalar_binds_as_its_own_type() {
    const { assert!(shapes::FLAG) };
    assert_eq!(shapes::TEXT, "a string");
    assert_eq!(shapes::COUNT, -3_i64);
    assert_eq!(shapes::RATIO, 1.5_f64);
}

#[test]
fn an_array_of_one_kind_binds_as_a_slice_of_it() {
    let words: &[&str] = shapes::WORDS;
    assert_eq!(words, ["one", "two"]);
}

#[test]
fn an_array_of_arrays_binds_as_a_slice_of_slices() {
    let grid: &[&[i64]] = shapes::GRID;
    assert_eq!(grid, [&[1, 2, 3][..], &[4, 5, 6][..]]);

    // Rows of different lengths are still one kind.
    let ragged: &[&[i64]] = shapes::RAGGED;
    assert_eq!(ragged, [&[1][..], &[2, 3][..]]);
}

#[test]
fn an_empty_array_adapts_to_the_arrays_beside_it() {
    let nothing: &[&str] = shapes::NOTHING;
    assert!(nothing.is_empty());

    // An empty row has no element to say what it holds, and must not turn the others into
    // something else.
    let holes: &[&[i64]] = shapes::HOLES;
    assert_eq!(holes, [&[][..], &[1, 2][..]]);
}
