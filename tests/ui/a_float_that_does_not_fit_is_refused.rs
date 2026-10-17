//! A float past the largest the width holds would become infinity, which is not what the file
//! says.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.reach: f32
}

fn main() {}
