//! An array of more than one kind has no type to be a constant of, and used to be bound as
//! the text of its Debug output. It is an error that names the key instead.

tomlfuse::file! {
    "tests/ui/unbindable.toml"

    [row]
    row.mixed
}

fn main() {}
