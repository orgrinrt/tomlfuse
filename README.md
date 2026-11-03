# `tomlfuse`

[![Crates.io](https://img.shields.io/crates/v/tomlfuse)](https://crates.io/crates/tomlfuse) [![docs.rs](https://img.shields.io/docsrs/tomlfuse)](https://docs.rs/tomlfuse)

> Bind toml fields into properly typed build-time constants with flexible patterns and hierarchies.

`tomlfuse` is three proc macros, `file!`, `package!` and `workspace!`, that read a toml file while the crate compiles and turn the keys you pick into constants. Which keys are taken is decided by glob patterns written inside the macro, with `*` for one segment, `**` for any number of them, `{a,b}` for alternatives and a `!` prefix for exclusion. Tables become nested modules, the comment above a key becomes the doc comment of its constant, and nothing is read at runtime.

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

`package!` and `workspace!` work the same way over a `Cargo.toml`, which is the usual reason to bind anything at all. When the path is left out, `package!` uses `CARGO_MANIFEST_DIR`.

```rust,ignore
use tomlfuse::package;

package! {
    [pkg]
    package.*

    [deps]
    dependencies.*
}

fn main() {
    println!("{} {}", pkg::NAME, pkg::VERSION);
    println!("Tokio: {}", deps::tokio::VERSION);
}
```

An integer binds as `i64` and a float as `f64` unless the pattern says otherwise. A type after the pattern, like `size.voxel_pixels: u32`, binds every number it reaches at that width and checks the value against it while the build runs. Values that have no constant to be, such as an array of mixed kinds, a datetime, infinity or NaN, are a `compile_error!` naming the key in the file.

The `examples/` directory has one example for each piece of the pattern language, all reading slices of the same `app.toml`, and `cargo run --example whole_app_config` runs the full one.

## Status

Pre-1.0, so a minor version may break and a patch version does not. The minimum supported rust is `1.88.0`. There are two cargo features, `no_std` and `no_alloc`, which both default to off and only name the guarantee that the expansion needs nothing outside `core`, and allocates nothing in the second case. Complex globs such as `config.*.timeout` aren't covered by tests yet, and character classes like `[a-z]` are not supported.

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/dev/LICENSE)
