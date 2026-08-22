# `tomlfuse`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/stargazers)
[![Crates.io](https://img.shields.io/crates/v/tomlfuse)](https://crates.io/crates/tomlfuse)
[![docs.rs](https://img.shields.io/docsrs/tomlfuse)](https://docs.rs/tomlfuse)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/issues)
![License](https://img.shields.io/github/license/orgrinrt/tomlfuse?color=%23009689)

> Toml fields bound into typed build-time constants with patterns and hierarchies.

</div>

## Relationship to `confuse`

[`confuse`](https://www.github.com/orgrinrt/confuse) generalises this approach beyond `toml` to
other file formats, and uses this crate for the `toml` case rather than reimplementing it. This
crate is the working `toml` implementation and is not deprecated. All three macros are
covered by integration tests, and the [limitations](#limitations-and-future-work) below still
apply.

## Features

- Compile-time binding of toml values to rust constants
- Flexibly preserve table hierarchies as nested modules
- Glob pattern support for selecting what to bind and what not to
  - `*` for one segment, `**` for any number
  - Alternation, `config.{debug,logging}.*`, matching any one of the alternatives
  - Negated patterns for exclusion (`!` prefix)
- Alias support for renaming paths (`alias foo = bar.baz`)
- Preserves comments from toml as doc comments
- Infers and parses all types the `toml::Value` enum has variants for, including *arrays*
  - *tables* translate to rust modules, so that all of this is possible at constant time without excessive complexity
- Works in `#![no_std]`, with or without an allocator

### `no_std` and no allocator

A binding names nothing outside `core`. Every value becomes a `pub const` of `&'static str`,
`&'static [T]`, `i64`, `f64` or `bool`, wrapped in `pub mod` and doc comments, plus one
`include_bytes!` binding the toml as a build input so editing it rebuilds what came out of it.
None of that is `std`, and none of it allocates: a `const` lives in the binary and a
`&'static [T]` points into it.

There are `no_std` and `no_alloc` features. Neither switches anything, because there is nothing
to switch; they exist so a workspace that turns them on across every dependency can name them
here. `tests/no_std_consumer.rs` is what holds the guarantee, by building a real `#![no_std]`
crate against this one with a control proving that crate genuinely has no `std` to fall back on.

This crate itself always builds with `std`. It is a proc macro, so it runs on the host inside
the compiler.

### Examples

`examples/` holds one example per piece of the pattern language, each reading a slice of the
same [`app.toml`](examples/app.toml): [sections](examples/one_section.rs),
[value types](examples/every_value_type.rs), [exclusions](examples/exclusions.rs),
[aliases](examples/aliases.rs), [alternation](examples/alternation.rs) and
[nesting](examples/nested_modules.rs). Two more put them together:
[`whole_app_config`](examples/whole_app_config.rs) uses the whole language on one file, and
[`build_metadata_and_config`](examples/build_metadata_and_config.rs) runs `package!` and
`file!` in one binary.

```bash
cargo run --example whole_app_config
```


## Usage

### Binding from a file

```rust,ignore
use tomlfuse::file;

file! {
    "path/to/config.toml"

    [settings] // <-- the module name that contains all the matches of the below patterns
    config.*           // = include all config.* paths
    !config.internal.* // = ...but exclude internals!

    [shortcuts]
    // you can create aliases for example to solve naming conflicts e.g when 
    // bringing in and mixing multiple sections of a toml file that could have same named fields.
    // note that aliases are intended for singular values (including tables though!)
    // so they should not contain glob patterns.
    alias timeout = config.params.timeout
}

fn main() {
    println!("Debug mode: {}", settings::DEBUG); // from toml's `config.debug`
    println!("Timeout: {}", shortcuts::TIMEOUT); // from toml's `config.params.timeout`
}
```

### Binding from package (Cargo.toml)

```rust,ignore
use tomlfuse::package;

// note that when path is omitted, the one from env, i.e. `CARGO_MANIFEST_DIR`, is used,
// or if that is missing too, the closest we can find walking dirs upwards until system root
package! {
    [pkg]
    package.*

    [deps]
    dependencies.*
}
// the main reason this variant (and the workspace one too) of the macro exist is for convenience,
// since one common use case is binding metadata from the package/workspace into the codebase.
// not having to resolve and/or input the paths explicitly reduces the friction of using this crate
// and also decreases the vectors for human error

fn main() {
    println!("Package name: {}", pkg::NAME);
    println!("Package version: {}", pkg::VERSION);
    println!("Tokio version: {}", deps::tokio::VERSION);
    println!("Serde features: {:?}", deps::serde::FEATURES);
    // note that currently this crate supports homogenous arrays, 
    // so the features const would be, as expected, an array of strings!
}
```

### Binding from workspace

Not currently covered with tests, so not guaranteed to work, but works similar to the package example.

When the path is omitted, looks for the first toml file that contains
`[workspace]` in the current directory and upwards until system root.

```rust,ignore
use tomlfuse::workspace;
workspace! {
    [workspace]
    members.*
    !members.foo
}
fn main() {
    println!("Workspace members: {:?}", workspace::MEMBERS);
    // while members array in the toml *does* contain a field `foo`...
    println!("Workspace's foo member: {:?}", workspace::FOO);
    // ...this will *not* compile due to the exclusion!
}
```

### Limitations and future work

#### Value types and patterns

- Presently only supports homogenous arrays (e.g. `["a", "b", "c"]`), not heterogeneous (e.g. `[1, "a", 3.14]`)
<details>
<summary>*Click to expand notes*</summary>

  - This is planned for the future
    - Initially by converting each element to a string representation and generating an array of strings in its stead (not ideal, but leaves the door open for consumer-side implementations for this)
    - Later down the line, as an optional alternative, by translating the array to an array of option tuples by merging the unique types of all the elements in the array as options wherein each
      `Some` value represents the element, and writing some convenience traits around the concept to get the values out of the array in a type-safe but "natural" way, while remaining build-time constant and avoiding dynamic dispatch
      - A tradeoff between runtime performance on one side, and binary size and compilation time on the other,
        *if* someone truly needs this
  - However, I'm not sure this is a common enough use-case to make a priority right now, I would be interested to hear any use cases that would require this though
</details>

- As of right now, more complex globs are not covered in tests (e.g.
  `config.*.timeout`), and may or may not work in different cases
<details>
<summary>*Click to expand notes*</summary>

  - These tests and possibly some refactoring for increased robustness are however being implemented in very near future as it is fundamental to the concept to handle these
  - The most common use case would be the patterns supported right now, so this crate releases initially with just them stabilized
</details>

- Alternation is supported, spelled `{a,b,c}` with commas rather than pipes, which is what
  globset reads. Character classes, `[a-z]`, are not, and will not be
<details>
<summary>*Click to expand notes*</summary>

  - A module header in this macro is `[name]`, and the macro input is a Rust token stream,
    which carries no newlines. So `config.debug` on one line followed by `[classes]` on the
    next is the same sequence of tokens as `config.debug[classes]`, and a parser that reads
    a bracket after an identifier as a character class swallows the next module header
    instead of starting a module
  - That was implemented and then removed for exactly this reason. Each pattern passed when
    tested alone, and the module following one went missing when they were tested together
  - The delimiter is spoken for. Alternation has no such clash
</details>

- Aliasing currently only supports singular values (including tables), but not batches (i.e pattern aliases)
<details>
<summary>*Click to expand notes*</summary>

  - In future there will be support for simple batch aliasing by using the source path's segment that matches a star to place into the alias pattern's same index star
    - This will however have some constraints that make it less useful than I'd ultimately want it to be, like:
      - This would only work with patterns that contain nothing but glob stars (however the amount of those could be any)
      - If there are multiple stars, then both sides of the alias assignment must match the same amount of stars, otherwise it won't work, which may or may not be obvious and would probably be confusing to the user
  - In the long run it would be better to resolve these at parse time rather than by string matching, but that is outside this crate's scope and would mean integrating another crate that already does it.
    - I would be interested to hear suggestions in the meanwhile
</details> 

#### Extended features

- While constant time binding is the most useful case for something like this, it is not the only one, and I would like to explore the possibility of allowing for dynamic binding as well with some static safety measures such as creating a schematic based on a toml file for type-safe binding, and allowing sane statically typed instances of the toml file to be created and mutated at runtime with minimal, preferably zero dynamic dispatch overhead
- While this crate is named `tomlfuse`, it could just as well be abstracted away and made implementable for any file format
<details>
<summary>*Click to expand notes*</summary>

  - It will be great to be able to confuse people outside of toml alone
    - However, I hate that making this more generic kills the perfect opportunity to adapt this concept to ron... as
      `ronfuse`...
      - but I digress
</details>


## Compatibility

This crate requires rust `1.73.0` or later. With present dependencies, this is the minimum supported version the dependencies allow. Bumping msrv is considered a breaking change and will be done in a minor version.

### Versioning policy

Minor versions may have breaking changes, which can include bumping msrv.

Patch versions are backwards compatible.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/dev/LICENSE)
