//! A float is not narrowed to an integer, and an integer is not widened to a float. Either would
//! be a conversion nobody asked for.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.ratio: u8
}

fn main() {}
