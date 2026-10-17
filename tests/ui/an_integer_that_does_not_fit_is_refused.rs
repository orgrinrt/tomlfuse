//! A value outside the range of the width it is given fails the build, naming the key, rather
//! than wrapping to something that fits.

tomlfuse::file! {
    "tests/ui/widths.toml"

    [size]
    size.voxel_pixels: u8
}

fn main() {}
