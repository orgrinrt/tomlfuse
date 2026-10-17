//! A width is one of the integer or float types. Anything else is reported at the type that
//! was written.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.voxel_pixels: u7
}

fn main() {}
