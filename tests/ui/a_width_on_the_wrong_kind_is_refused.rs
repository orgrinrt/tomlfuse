//! A width applies to integers and floats. On anything else it is a mistake in the pattern,
//! and is said so rather than ignored.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.label: u32
}

fn main() {}
