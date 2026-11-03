# `tomlfuse`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/stargazers)
[![Crates.io](https://img.shields.io/crates/v/tomlfuse)](https://crates.io/crates/tomlfuse)
[![docs.rs](https://img.shields.io/docsrs/tomlfuse)](https://docs.rs/tomlfuse)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/issues)
![License](https://img.shields.io/github/license/orgrinrt/tomlfuse?color=%23009689)

> Bind toml fields into properly typed build-time constants with flexible patterns and hierarchies.

</div>

A toml file that holds a value the code needs, a tile size, a timeout, the version of a dependency, is usually read at runtime or copied by hand into a constant that then drifts from the file. `tomlfuse` is a set of three proc macros, `file!`, `package!` and `workspace!`, that read the toml while the compiler runs and expand what was selected into `pub const` items, so the file stays the one place the value is written and the code gets a constant of the right type.

Which keys are taken is decided by glob patterns written inside the macro, with `*` for one segment, `**` for any number of them, `{a,b}` for alternatives and a `!` prefix for exclusion. Tables become nested modules, the comment above a key becomes the doc comment of its constant, and an alias can rename a path where two sections would otherwise collide. The expansion names nothing outside `core` and allocates nothing, so it works in a `#![no_std]` crate with no allocator, and the toml is bound as a build input, which means editing it rebuilds whatever was generated from it.

The reading and walking itself lives in [`tomlfuse-core`](https://crates.io/crates/tomlfuse-core), a plain library that knows nothing about Rust tokens, and the macros are a thin layer over it.

## Usage

```bash
cargo add tomlfuse
```

`file!` takes a path, then one or more `[module]` headers, each followed by the patterns that belong in it.

```rust,ignore
use tomlfuse::file;

file! {
    "path/to/config.toml"

    [settings]         // the module that holds every match of the patterns below
    config.*           // all config.* paths
    !config.internal.* // except the internals

    [shortcuts]
    // an alias renames one path, which is for singular values and tables, so it takes no globs
    alias timeout = config.params.timeout
}

fn main() {
    println!("Debug mode: {}", settings::DEBUG);     // config.debug
    println!("Timeout: {}", shortcuts::TIMEOUT);     // config.params.timeout
}
```

`package!` and `workspace!` work the same way over a `Cargo.toml`, which is the usual reason to bind anything at all, metadata of the crate into the crate. When the path is left out, `package!` uses `CARGO_MANIFEST_DIR` and walks upwards if that is missing, and `workspace!` looks for the first manifest holding `[workspace]` from the crate being built upwards, falling back to the crate's own manifest.

```rust,ignore
use tomlfuse::{package, workspace};

package! {
    [pkg]
    package.*

    [deps]
    dependencies.*
}

workspace! {
    [workspace]
    members.*
    !members.foo   // workspace::FOO does not compile
}

fn main() {
    println!("{} {}", pkg::NAME, pkg::VERSION);
    println!("Tokio: {}", deps::tokio::VERSION);
    println!("Serde features: {:?}", deps::serde::FEATURES);
}
```

An integer binds as `i64` and a float as `f64` unless the pattern says otherwise. A type after the pattern, one of `i8` to `i64`, `u8` to `u64`, `f32` and `f64`, binds every number it reaches at that width, and checks the value against it while the build runs, so a `300` for a `u8` stops with a message naming the key. An integer is not widened to a float, which means a key written `3` cannot be an `f32` and wants `3.0` in the file. `usize` and `isize` are left out on purpose, since a proc macro runs on the host and would be checking against a pointer width that may not be the target's, and `as usize` works in a `const` anyway.

A width on an array gives it a fixed length read from the file, and a pattern with a width has to reach at least one number, so a typo in the key is an error instead of a silent `i64`.

```rust,ignore
file! {
    "path/to/engine.toml"

    [size]
    size.voxel_pixels: u32     // pub const VOXEL_PIXELS: u32

    [colours]
    palette.world: u8          // pub const WORLD: [[u8; 3]; 3] for three rows of three
}
```

Values with no constant to be, which are an array of mixed kinds, a table inside an array, a datetime, infinity and NaN, are a `compile_error!` naming the key in the file. A datetime that is wanted for its text can be written as a string in the toml.

## Example

The `examples/` directory holds one example for each piece of the pattern language, each reading a slice of the same [`app.toml`](examples/app.toml): [sections](examples/one_section.rs), [value types](examples/every_value_type.rs), [exclusions](examples/exclusions.rs), [aliases](examples/aliases.rs), [alternation](examples/alternation.rs), [widths](examples/widths.rs) and [nesting](examples/nested_modules.rs). Two put them together, [`whole_app_config`](examples/whole_app_config.rs) over one file with the whole language, and [`build_metadata_and_config`](examples/build_metadata_and_config.rs) which runs `package!` and `file!` in one binary.

```bash
cargo run --example whole_app_config
```

## Motivation

Constants that come from configuration tend to get written twice, once in the file where a person edits them and once in the source where the compiler needs them, and nothing says the two agree. Reading the file in a build script solves that, although it leaves the type-mapping, the module layout and the rebuild tracking to be written again in every project. `tomlfuse` does that part once, with the selection of keys kept in the macro call next to where the constants are used, so a reader of the code sees what is taken from the file without opening it.

## Extras

### Status

Pre-1.0, so a minor version may break and a patch version does not. The minimum supported rust is `1.88.0`, which is what `globset`, a direct dependency, declares, and raising it counts as a breaking change. The crate itself always builds with `std`, being a proc macro that runs inside the compiler.

### Cargo features

| Feature | Default | Effect |
|---|---|---|
| `no_std` | off | Names the guarantee that the expansion needs nothing outside `core`, held by `tests/no_std_consumer.rs`. Switches nothing, so a workspace that enables it everywhere can name it here |
| `no_alloc` | off | Implies `no_std`, and names that the expansion allocates nothing |

### Limitations

Complex globs such as `config.*.timeout` are not covered by tests yet and may or may not work in every case. Alternation is spelled with commas, `{a,b,c}`, as `globset` reads it, and character classes, `[a-z]`, are not supported, because a module header is also written in brackets and the macro input carries no newlines to tell the two apart. Aliases take a single path and not a batch, so an alias with a star in it is refused. Arrays are homogeneous, and binding mixed ones as tuples of options is possible later if someone has a use for it, in which case an issue describing it is welcome.

### The other crates

[`tomlfuse-core`](https://crates.io/crates/tomlfuse-core) is the toml reader and field walker without any macro in it, for a build script or a tool that wants the same constants from the same file, in a shader header for instance.

## Support

Feel free to contribute! If unsure about wasting work, the best practice is to throw in an issue describing what you'd do, and only then commit to writing a big PR, because chances are, it might not be something that belongs here. However, forks are always a valid choice and we'd encourage everyone to experiment and have their own takes on this. When doing this, do mind the license(s) though!

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/dev/LICENSE)
