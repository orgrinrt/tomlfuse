//! A table inside an array has no constant to bind as, and used to be bound as the text of
//! its Display output. It is an error that names the element instead.

tomlfuse::file! {
    "tests/ui/unbindable.toml"

    [row]
    row.records
}

fn main() {}
