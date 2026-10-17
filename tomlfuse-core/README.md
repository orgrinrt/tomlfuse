# tomlfuse-core

The plain library behind the [`tomlfuse`](https://crates.io/crates/tomlfuse) macros. It reads
a toml file, walks it through a set of patterns and returns what is left as a tree of
modules and typed constants. It knows nothing about Rust tokens, so a build script or a tool
can call it and write the same constants in whatever language it likes.

```rust
use tomlfuse_core::{bind, Pattern, Section, Source};

let source = Source::parse("app.toml", "app.toml".into(), "[size]\nvoxel = 17\n").unwrap();
let section = Section {
    name: "engine".into(),
    include: vec![Pattern::new("size.*")],
    ..Section::default()
};

let module = bind(&source, &section).unwrap();
assert_eq!(module.name.as_deref(), Some("engine"));
assert_eq!(module.consts[0].name, "VOXEL");
```

A [`Source`] is a file that has been found, read and parsed. [`Source::read`] takes an exact
path, and [`Source::locate`] searches the way a path in a crate's source is expected to: the
crate, then the workspace. A [`Section`] says which keys to take and what to call them. [`bind`]
walks one through the other and returns a [`Module`], whose constants each carry a name, the
comment that sat above the key, a [`Ty`] and a [`Lit`].

The macros are in `tomlfuse`, which depends on this crate.

[`Source`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html
[`Source::read`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html#method.read
[`Source::locate`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html#method.locate
[`Section`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Section.html
[`bind`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/fn.bind.html
[`Module`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Module.html
[`Ty`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/enum.Ty.html
[`Lit`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/enum.Lit.html
