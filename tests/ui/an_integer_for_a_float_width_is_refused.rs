//! The same the other way: an integer is not widened to a float. Write it with a decimal
//! point in the file if a float is what is meant.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.count: f32
}

fn main() {}
