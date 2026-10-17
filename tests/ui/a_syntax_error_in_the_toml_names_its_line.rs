//! A toml that does not parse is reported with the parser's own position and the line it
//! stopped at, so a syntax error in a large file is not found by eye.

tomlfuse::file! {
    "tests/ui/malformed.toml"

    [engine]
    engine.*
}

fn main() {}
