# `tomlfuse-core`

[![Crates.io](https://img.shields.io/crates/v/tomlfuse-core)](https://crates.io/crates/tomlfuse-core) [![docs.rs](https://img.shields.io/docsrs/tomlfuse-core)](https://docs.rs/tomlfuse-core)

> Read a toml file and walk it into typed constants. The plain library behind the tomlfuse macros.

It reads a toml file, walks it through a set of patterns and returns what is left as a tree of modules and typed constants. It knows nothing about Rust tokens, so a build script or a tool can call it and write the same constants in whatever language it likes.

## Usage

```bash
cargo add tomlfuse-core
```

```rust
use tomlfuse_core::{bind, Pattern, Section, Source};

let source = Source::parse("app.toml", "app.toml".into(), "[size]\nvoxel = 17\n").unwrap();
let section = Section {
    name: "engine".into(),
    include: vec![Pattern::new("size.*")],
    ..Section::default()
};

let bound = bind(&source, &section).unwrap();
assert!(bound.problems.is_empty());
assert_eq!(bound.module.name.as_deref(), Some("engine"));
assert_eq!(bound.module.consts[0].name, "VOXEL");
```

A [`Source`] is a file that has been found, read and parsed, a [`Section`] says which keys to take and what to call them, and [`bind`] walks one through the other and returns a [`Bound`]: a [`Module`] of constants plus a list of [`Problem`]s. A value that has no constant to be is a `Problem` naming its key, and the rest of the section is bound regardless, so the caller decides what one is worth. The macros make each a `compile_error!`.

[`Source`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html
[`Section`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Section.html
[`bind`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/fn.bind.html
[`Bound`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Bound.html
[`Problem`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Problem.html
[`Module`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Module.html

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/dev/LICENSE)
