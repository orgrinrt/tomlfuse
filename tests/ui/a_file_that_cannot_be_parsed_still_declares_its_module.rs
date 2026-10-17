//! When the toml cannot be bound the module is still declared, empty, so naming it adds no
//! unresolved-module error under the one that says what went wrong.

tomlfuse::file! {
    "tests/ui/malformed.toml"

    [engine]
    engine.*
}

use engine as _;

fn main() {}
