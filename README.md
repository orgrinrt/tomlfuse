# `tomlfuse`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/stargazers)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/tomlfuse)](https://crates.io/crates/tomlfuse)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/issues)
[![Latest Version](https://img.shields.io/badge/version-0.0.1-red.svg?label=latest)](https://github.com/orgrinrt/tomlfuse)
![Crates.io Version](https://img.shields.io/crates/v/tomlfuse?logoSize=auto&color=%23FDC700&link=https%3A%2F%2Fcrates.io%2Fcrates%2Ftomlfuse)
![Crates.io Size](https://img.shields.io/crates/size/tomlfuse?color=%23C27AFF&link=https%3A%2F%2Fcrates.io%2Fcrates%2Ftomlfuse)
![GitHub last commit](https://img.shields.io/github/last-commit/orgrinrt/tomlfuse?color=%23009689&link=https%3A%2F%2Fgithub.com%2Forgrinrt%2Ftomlfuse)

> Easily bind toml fields into properly typed build-time constants with flexible pattern matching and hierarchies.


</div>

## Features

- Compile-time binding of toml values to rust constants
- Flexibly preserve table hierarchies as nested modules
- Glob pattern support for selecting what to bind and what not to
    - Supports negated patterns for exclusion (`!` prefix)
- Alias support for renaming paths (`alias foo = bar.baz`)
- Preserves comments from toml as doc comments
- Infers and parses all types the `toml::Value` enum has variants for, including *arrays*
    - *tables* translate to rust modules, so that all of this is possible at constant time without excessive complexity

### Limitations and future work

#### Value types and patterns

- Presently only supports homogenous arrays (e.g. `["a", "b", "c"]`), not heterogeneous (e.g. `[1, "a", 3.14]`)
    - This is planned for the future
        - Initially by converting each element to a string representation and generating an array of strings in its stead (not ideal, but leaves the door open for consumer-side implementations for this)
        - Later down the line, as an optional alternative, by translating the array to an array of option tuples by merging the unique types of all the elements in the array as options wherein each
          `Some` value represents the element, and writing some convenience traits around the concept to get the values out of the array in a type-safe but "natural" way, while remaining build-time constant and avoiding dynamic dispatch
            - A tradeoff between runtime performance and binary size and compilation time, essentially,
              *if* someone truly needs this
    - However, I'm not sure this is a common enough use-case to make a priority right now, I would be interested to hear any use cases that would require this though
- As of right now, more complex globs are not covered in tests (e.g.
  `config.*.timeout`), and may or may not work in different cases
    - These tests and possibly some refactoring for increased robustness are however being implemented in very near future as it is fundamental to the concept to handle these
    - The most common use case would be the patterns supported right now, so this crate releases initially with just them stabilized
- Glob syntax for collections, i.e `{a|b|c}`, or other more involved patterns is not supported yet either
    - This is something that would be preferable to support, but also not a priority right now, since the use case of toml file binding feels to me like something that would not often warrant the use of this kind of complexity
- Aliasing currently only supports singular values (including tables), but not batches (i.e pattern aliases)
    - In future there will be support for simple batch aliasing by using the source path's segment that matches a star to place into the alias pattern's same index star
        - This will however have some constraints that make it less useful than I'd ultimately want it to be, like:
            - This would only work with patterns that contain nothing but glob stars (however the amount of those could be any)
            - If there are multiple stars, then both sides of the alias assignment must match the same amount of stars, otherwise it won't work, which may or may not be obvious and would probably be confusing to the user
    - In the long run, it'd be great to find a more robust solution, but this would be entirely outside this crate's scope, so it would be an integration of another crate that does this ultimately.
        - I would be interested to hear suggestions in the meanwhile

#### Extended features

- While constant time binding is the most useful case for something like this, it is not the only one, and I would like to explore the possibility of allowing for dynamic binding as well with some static safety measures such as creating a schematic based on a toml file for type-safe binding, and allowing sane statically typed instances of the toml file to be created and mutated at runtime with minimal, preferably zero dynamic dispatch overhead
- While this crate is named `tomlfuse`, it could just as well be abstracted away and made implementable for any file format
    - It will be great to be able to confuse people outside of toml alone
        - However, I hate that making this more generic kills the perfect opportunity to adapt this concept to ron... as
          `ronfuse`...
            - but I digress

## Usage

### Binding from a file

```rust
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
    alias timeout = config.settings.timeout
}

fn main() {
    println!("Debug mode: {}", settings::config::DEBUG);
    println!("Timeout: {}", shortcuts::TIMEOUT);
}
```

### Binding from package (Cargo.toml)

```rust
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
// not having to resolve or input the paths explicitly reduces the friction of using this crate
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

```rust
use tomlfuse::workspace;
workspace! {
    [workspace]
    members.*
    !members.foo
}
fn main() {
    println!("Workspace members: {:?}", workspace::MEMBERS);
    // while members array in the toml contains `foo`...
    println!("Workspace's foo member: {:?}", workspace::FOO);
    // ...this will not compile due to exclusion!
}
```

## Compatibility

This crate requires rust `1.64.0` or later.

For practical reasons, we pin the msrv there to utilize ver `1.64.0` cargo's stabilized
`workspace-inheritance` feature, but also to remain fairly compatible.

### Versioning policy

Minor versions may have breaking changes, which can include bumping msrv.

Patch versions are backwards compatible, so using version specifiers such as `~x.y` or `^x.y.0` is safe.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/master/LICENSE)
