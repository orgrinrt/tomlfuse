//! A datetime has no type to bind as, and used to be bound as the text it was written with.
//! It is an error that names the key instead.

tomlfuse::file! {
    "tests/ui/unbindable.toml"

    [row]
    row.when
}

fn main() {}
