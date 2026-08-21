//! `!` removes a match rather than hiding it, so the name is gone at compile time.

tomlfuse::file! {
    "tests/ui/fixture.toml"

    [net]
    server.*
    !server.internal.*
}

fn main() {
    let _ = net::internal::OWNER;
}
