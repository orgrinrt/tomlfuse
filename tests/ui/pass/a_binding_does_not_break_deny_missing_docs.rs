//! A crate that denies `missing_docs` can still use a binding.
//!
//! It could not, before. Every generated item was undocumented as far as the lint was
//! concerned, and it fires spanned at the macro invocation, where no `#[allow]` the consumer
//! writes reaches it. So the whole crate was unusable from any library that denies the lint,
//! which is an ordinary thing for a library to do.
//!
//! `conf::internal` below is the case that cannot be fixed by commenting the toml harder.
//! It exists because `internal.owner` is a dotted key, so the table is brought into being by
//! the key rather than written down, and there is no line above it for a comment to sit on.

#![deny(missing_docs)]
//! (the crate-level doc `missing_docs` also wants)

tomlfuse::file! {
    "tests/ui/fixture.toml"

    [conf]
    app.*
    server.*
}

fn main() {
    let _ = conf::NAME;
    let _ = conf::HOST;
    // The module a dotted key created, which no comment could have documented.
    let _ = conf::internal::OWNER;
}
