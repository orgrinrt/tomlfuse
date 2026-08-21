//! Excluding a key and aliasing it leaves the alias and takes the original name away.
//!
//! Two sections here have a `timeout`, so both are aliased to say which is which. Without
//! the exclusions the bare `TIMEOUT` would exist too, holding whichever of the two the
//! binding happened to reach last.

tomlfuse::file! {
    "tests/ui/fixture.toml"

    [runtime]
    server.*
    !server.internal.*
    !server.timeout
    alias request_timeout = server.timeout
}

fn main() {
    // The alias is here. The name it was taken from is not.
    let _ = runtime::REQUEST_TIMEOUT;
    let _ = runtime::TIMEOUT;
}
