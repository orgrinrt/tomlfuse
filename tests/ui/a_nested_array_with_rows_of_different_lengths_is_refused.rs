//! With a width the array is a fixed one, and a fixed array has one length. Rows that differ
//! are reported at the key, and without a width the same rows are a slice of slices.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.ragged: u8
}

fn main() {}
