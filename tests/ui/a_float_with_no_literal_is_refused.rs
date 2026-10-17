//! Infinity and NaN cannot be written as a literal, and a float too small for its width would
//! read back as zero. Each is refused at the file, where the first two used to panic the macro.

tomlfuse::file! {
    "tests/ui/nonfinite.toml"

    [odd]
    odd.infinite
    odd.not_a_number
    odd.tiny: f32
}

fn main() {}
