//! A width for a pattern that reaches no key is a typo, and used to bind the key it was meant
//! for at `i64` without a word.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.*
    size.voxle: u8
}

fn main() {}
