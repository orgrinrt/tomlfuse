# `tomlfuse`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/stargazers)
[![Crates.io](https://img.shields.io/crates/v/tomlfuse)](https://crates.io/crates/tomlfuse)
[![docs.rs](https://img.shields.io/docsrs/tomlfuse)](https://docs.rs/tomlfuse)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/issues)
![License](https://img.shields.io/github/license/orgrinrt/tomlfuse?color=%23009689)

> Toml fields bound into typed constants at compile time, chosen with glob patterns. Ships `file!`, `package!` and `workspace!`, and what it emits is `core` only.

</div>

`tomlfuse` reads a toml file while the crate compiles and binds what it finds as ordinary
constants, so `app.name` in the file becomes `settings::NAME` in the code, a `pub const` of
`&'static str` holding whatever the file said at the moment of the build. Which keys come
across is said with glob patterns over the dotted paths, and each group of patterns lands in
a module named in the invocation, so the shape the rest of the program sees is chosen there
and doesn't have to be the toml's own shape, though it can be that too.

There's three macros and they differ only in where the file comes from. `file!` takes a
path, `package!` reads the crate's own `Cargo.toml`, and `workspace!` climbs from there to
the nearest manifest with a `[workspace]` table in it. So the version a binary prints, the
dependencies it was compiled against and its own configuration can all come through the same
mechanism and sit as constants in one binary.

Comments in the toml come across too, as doc comments on the constant or module they sit
above, and the file gets registered as a build input (through an `include_bytes!` of it), so
editing it rebuilds what was generated from it instead of leaving the previous values in
place with the build reporting success.

The expansion names nothing outside `core`, so a `#![no_std]` crate with no allocator gets
working constants out of this. The crate itself is a proc macro though, so it runs inside the
compiler with `std` whatever the consumer is; only what it emits is what a consumer has to
carry.

## Usage

```bash
cargo add tomlfuse
```

`file!` takes the path first, then one or more sections. A section opens with its name in
brackets and is followed by the patterns whose matches land in it, so `[settings]` with
`app.*` under it gives a `settings` module holding one constant per key under `[app]`:

```rust
use tomlfuse::file;

file! {
    "examples/app.toml"

    [settings]
    app.*
}

fn main() {
    println!("{} {}", settings::NAME, settings::VERSION);
    if settings::VERBOSE {
        println!("verbose");
    }
}
```

A relative path is looked up from the crate first (its `CARGO_MANIFEST_DIR`) and then from
the workspace root, and an absolute one is taken as given. A file found in neither place is
a compile error naming both, which is what a mistyped path should be.

A pattern is a dotted path with globs in it. `*` matches one segment and `**` any number,
`{a,b}` matches either name at that position (commas and no spaces, since a space inside the
braces is a character to be matched), and a leading `!` takes what the rest matches back out
of the section it sits in. Order between the two doesn't matter, everything matched is
collected first and then everything excluded is removed, and what's removed is gone rather
than empty: naming it is a compile error. `alias name = some.path` binds one value under a
name of its own, and it takes a single path, not a pattern. Character classes like `[a-z]`
aren't part of it, because `[` is what opens a section header and the macro input has no line
breaks in it to tell the two apart.

Keys become upper case constants and tables become lower case modules, and a dash in a key
turns into an underscore on the way, so `max-body-bytes` arrives as `MAX_BODY_BYTES`. The
part of the path the pattern spells out by name gets dropped and what the glob matched keeps
its shape: `config.*` gives `DEBUG` and `settings::TIMEOUT` for `config.debug` and
`config.settings.timeout`, and a bare `**` reproduces the whole file as nested modules. An
alternation keeps its names though, so `{logging,telemetry}.*` gives `logging::LEVEL` beside
`telemetry::ENABLED` in place of piling both under one module, which they'd collide in.

A string is a `&'static str`, an integer an `i64`, a float an `f64`, a boolean a `bool`, and
an array whose elements are all of one type a `&'static [T]` of that. A datetime comes across
as its string form, and an array mixing types lands as one string holding its debug form,
which compiles but isn't much use; both are on the list further down.

`package!` and `workspace!` take no path. The first reads the `Cargo.toml` of the crate being
compiled and the second walks upwards from it until a manifest with a `[workspace]` table
turns up, settling for the crate's own when none does. Everything after that reads the same:

```rust
use tomlfuse::package;

package! {
    [pkg]
    package.*
    !package.metadata.*

    [deps]
    dependencies.*
}

fn main() {
    println!("{} {}", pkg::NAME, pkg::VERSION);
    println!("built against syn {}", deps::syn::VERSION);
}
```

Do note that a dependency spelled as a plain version string, `globset = "^0.4"`, is a
constant (`deps::GLOBSET`), and one spelled as a table is a module with the table's keys in
it, as above with `deps::syn::VERSION`. That's the toml's shape and not something this crate
decides.

A comment above a key or a table, or on the same line after it, becomes a `#[doc]` on what it
turned into, so a toml commented for whoever edits it documents the constants in `cargo doc`
as well. A blank line between the comment and the key breaks the association, and a comment
with nothing under it is dropped. The generated module carries an `#[allow]` for
`missing_docs` and for the clippy lints that judge a literal's value, since the value came
from the file and the code from here, and neither side can do anything about a diagnostic
spanned at the macro invocation.

The [`examples/`](https://github.com/orgrinrt/tomlfuse/tree/main/examples) directory holds one
file per piece of the pattern language, all reading slices of the same
[`app.toml`](https://github.com/orgrinrt/tomlfuse/blob/main/examples/app.toml), and every one
of them is built and run by the test suite, so they can't drift from what the crate parses.

## Example

Here's the binding for a small server, reading the `app.toml` the examples share. It has a
`timeout` under both `[server]` and `[database]`, which would collide as `TIMEOUT` if both
sections were bound into one module, so both are excluded and brought back under an alias
each. The `internal` table under `[server]` is an operator's bookkeeping that the binary has
no business knowing, so it's carved out. And the sections here are the program's own grouping:
`runtime` gathers what the binary starts and `observability` what an operator turns up, and
neither of those exists in the file.

```rust
use tomlfuse::file;

file! {
    "examples/app.toml"

    // what the binary calls itself
    [meta]
    app.*

    // what it starts, with the operator's bookkeeping left out and the two
    // `timeout` keys given names that say which is which
    [runtime]
    server.*
    database.*
    !server.internal.*
    !server.timeout
    !database.timeout
    alias request_timeout = server.timeout
    alias query_timeout = database.timeout

    // what an operator turns up when something is wrong
    [observability]
    {logging,telemetry}.*

    // two keys out of one section, without taking the section
    [ceilings]
    limits.{max_body_bytes,max_connections}
}

fn main() {
    println!("{} {}", meta::NAME, meta::VERSION);
    println!("listening on {}:{}", runtime::HOST, runtime::PORT);
    println!("  requests give up after {}s", runtime::REQUEST_TIMEOUT);
    println!("  queries give up after {}s", runtime::QUERY_TIMEOUT);
    println!("logging {} to {:?}", observability::logging::LEVEL, observability::logging::TARGETS);
    if observability::telemetry::ENABLED {
        println!("telemetry to {}", observability::telemetry::ENDPOINT);
    }
    println!("refusing bodies over {} bytes", ceilings::MAX_BODY_BYTES);
}
```

After this `runtime::TIMEOUT` doesn't exist and neither does `runtime::internal::OWNER`, and
writing either is a build error, which is the whole point of excluding them over merely not
reading them. Nothing here runs at startup either: every value is a constant in the binary, a
pattern that stops matching is a compile error, and editing the toml rebuilds the lot.

## Motivation

A configuration a binary reads at startup is a runtime failure waiting for the day the file
is wrong, and a value that never changes between builds has little reason to be read at all.
So the values get baked in: a pattern that stops matching fails the build, a type gets
checked where the constant is used, and there's no parsing and no file access when the
program runs. The cost is that changing a value means rebuilding, which is the trade, and
it's the right one for build metadata and for the kind of configuration that ships inside the
binary anyway, and the wrong one for anything an operator is meant to edit in place. This
does nothing for the latter.

`env!("CARGO_PKG_VERSION")` and its siblings cover the crate's own name, version and a few
other fields, which is fine for what they cover. What they don't give is the versions of what
the crate was compiled against, or the `[package.metadata]` table a project keeps its own
things in, and those are what a `--version` line or a bug report actually wants; `package!`
reads the manifest whole, so there's no list of fields to fall off.

The same can be done with a build script writing a rust file for `include!`, and that's what
this saves writing per project, along with getting the types and the module shape from the
toml instead of by hand. The pattern language is the part that's worth having in one place,
honestly, since every project that does this by hand ends up with some half of it.

## Extras

### Status

Early days still, so the api hasn't settled and a release can move things out from under
whatever was written against the previous one. Every release is tagged and the log between
two tags is what moved. The pattern language as described above is what's covered by tests
and what I'd rely on; anything past it is best tried before being leaned on.

Builds on stable. The `rust-version` in the manifest is 1.88, which is what `globset`
declares, so nothing older resolves the dependency graph whatever this crate's own source
would have allowed.

### Cargo features

There are two, `no_std` and `no_alloc`, and neither switches anything, because there's
nothing to switch: the expansion is `core` only and allocates nothing on every build, a
`const` lives in the binary and a `&'static [T]` points into it. They exist so a workspace
that turns those flags on across every dependency can name them here without the build
failing on an unknown feature. `no_alloc` implies `no_std`. The guarantee is held by a test
that writes out a real `#![no_std]` crate, builds it against this one on every feature
selection, and checks with a control that the crate genuinely has no `std` to fall back on.

### Limitations

Arrays have to be of one type. A mixed one, `[1, "a", 3.14]`, comes across as a single string
of its debug form, which is at least a value that compiles, and not much more than that. I'm
not sure a heterogeneous array is a common enough thing in a config to be worth typing it
properly, but I'd be interested to hear of a use case that needs it.

A datetime is bound as its string form, since there's no `core` type to bind it to.

An alias takes one path and gives it one name. A pattern alias would have to say what happens
to the part the star matched, and that's not settled, so for now the renaming is one value at
a time.

Character classes, `[a-z]`, are not supported and probably won't be, for the section header
reason above. Alternation, `{a,b}`, covers most of what they'd be used for in a config.

`workspace!` reads whichever manifest opens a `[workspace]` table first on the way up, and a
crate that isn't in one gets its own manifest, so for a single crate the two macros read the
same file.

## Support

Feel free to contribute! If unsure about wasting work, the best practice is to throw in an issue describing what you'd do, and only then commit to writing a big PR, because chances are, it might not be something that belongs here. However, forks are always a valid choice and we'd encourage everyone to experiment and have their own takes on this. When doing this, do mind the license(s) though!

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/main/LICENSE)
