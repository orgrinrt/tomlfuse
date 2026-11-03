# `tomlfuse-core`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/stargazers)
[![Crates.io](https://img.shields.io/crates/v/tomlfuse-core)](https://crates.io/crates/tomlfuse-core)
[![docs.rs](https://img.shields.io/docsrs/tomlfuse-core)](https://docs.rs/tomlfuse-core)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/tomlfuse.svg)](https://github.com/orgrinrt/tomlfuse/issues)
![License](https://img.shields.io/github/license/orgrinrt/tomlfuse?color=%23009689)

> Read a toml file and walk it into typed constants. The plain library behind the tomlfuse macros.

</div>

It reads a toml file, walks it through a set of patterns and returns what is left as a tree of modules and typed constants. It knows nothing about Rust tokens, so a build script or a tool can call it and write the same constants in whatever language it likes. The macros are in [`tomlfuse`](https://crates.io/crates/tomlfuse), which depends on this crate.

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

A [`Source`] is a file that has been found, read and parsed. [`Source::read`] takes an exact
path, and [`Source::locate`] searches the way a path in a crate's source is expected to: the
crate, then the workspace. A [`Section`] says which keys to take and what to call them. [`bind`]
walks one through the other and returns a [`Bound`]: a [`Module`], whose constants each carry a
name, the comment that sat above the key, a [`Ty`] and a [`Lit`], and a list of [`Problem`]s.

A value that has no constant to be, an array of mixed kinds, a table inside an array, a
datetime or a float that is infinite or NaN, is a `Problem` naming its key in the file, and
the rest of the section is bound regardless. The caller decides what one is worth; the
macros make each a `compile_error!`.

[`Source`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html
[`Source::read`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html#method.read
[`Source::locate`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Source.html#method.locate
[`Section`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Section.html
[`bind`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/fn.bind.html
[`Bound`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Bound.html
[`Problem`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Problem.html
[`Module`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/struct.Module.html
[`Ty`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/enum.Ty.html
[`Lit`]: https://docs.rs/tomlfuse-core/latest/tomlfuse_core/enum.Lit.html

## Support

Feel free to contribute! If unsure about wasting work, the best practice is to throw in an issue describing what you'd do, and only then commit to writing a big PR, because chances are, it might not be something that belongs here. However, forks are always a valid choice and we'd encourage everyone to experiment and have their own takes on this. When doing this, do mind the license(s) though!

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/tomlfuse/blob/dev/LICENSE)
