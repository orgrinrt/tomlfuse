//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use input::MacroInput;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::env;
use std::path::PathBuf;
use syn::{parse_macro_input, LitStr};

mod comments;
mod field;
mod input;
mod module;
mod pattern;
mod utils;

use utils::*;

/// Binds constants from the workspace's `Cargo.toml`.
///
/// The manifest is found by walking upwards from the crate being compiled (its
/// `CARGO_MANIFEST_DIR`) until one with a `[workspace]` table turns up, and a crate that
/// isn't in a workspace gets its own manifest instead, so for a single crate this reads the
/// same file [`package!`] does. Everything after that is the same as [`file!`] with the path
/// left out: sections in brackets, and under each the patterns whose matches land in it.
///
/// ```
/// use tomlfuse::workspace;
///
/// workspace! {
///     [members]
///     workspace.members
///
///     [meta]
///     workspace.metadata.*
/// }
///
/// fn main() {
///     for member in members::MEMBERS {
///         println!("{member}");
///     }
/// }
/// ```
///
/// Do note that `workspace.members` is an array of strings in the manifest, so it comes
/// across as a `&'static [&'static str]`, and a `[workspace.metadata]` table becomes a
/// module with a constant per key, the same as any other table.
#[proc_macro]
pub fn workspace(input: TokenStream) -> TokenStream {
    // find workspace root
    let cargo_path = find_workspace_root().join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Binds constants from the `Cargo.toml` of the crate being compiled.
///
/// The manifest is the one `CARGO_MANIFEST_DIR` points at, so there's no path to give, and
/// the rest reads like [`file!`]: sections in brackets, and under each the patterns whose
/// matches land in it. What it's mostly for is the questions a binary asks about itself,
/// its version, its licence, and what it was compiled against, where `env!` and the
/// `CARGO_PKG_*` variables cover the crate's own fields and nothing about the dependencies.
///
/// ```
/// use tomlfuse::package;
///
/// package! {
///     [pkg]
///     package.*
///     !package.metadata.*
///
///     [deps]
///     dependencies.*
/// }
///
/// fn main() {
///     println!("{} {}, {}", pkg::NAME, pkg::VERSION, pkg::LICENSE);
///     println!("built against syn {}", deps::syn::VERSION);
/// }
/// ```
///
/// A dependency spelled as a plain version string is a constant of that string, and one
/// spelled as a table (`syn = { version = "^2.0" }`) is a module holding the table's keys,
/// which is the manifest's shape and not a choice made here. `[package.metadata]` is
/// excluded above because that's where a project keeps its own things, but it binds like
/// any other table when wanted. [`workspace!`] is the same over the workspace's manifest.
#[proc_macro]
pub fn package(input: TokenStream) -> TokenStream {
    // use manifest dir for crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let cargo_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Binds constants from a toml file at the given path.
///
/// The path comes first, as a string literal. A relative one is looked up from the crate
/// (its `CARGO_MANIFEST_DIR`) and then from the workspace root, an absolute one is taken as
/// given, and a file found in neither place fails the build with both places named. The
/// file is registered as a build input as well, so editing it rebuilds whatever was bound
/// from it.
///
/// After the path come the sections. Each opens with a name in brackets, which becomes a
/// `pub mod` of that name, and is followed by the patterns whose matches land in it: a
/// dotted path where `*` matches one segment, `**` any number and `{a,b}` either name, a
/// leading `!` takes its matches back out of the section, and `alias name = some.path`
/// binds one value under a name of its own. Keys become upper case constants, tables become
/// lower case modules, and the part of a path the pattern spells out by name is dropped, so
/// `app.*` puts `app.name` at `NAME` directly under the section.
///
/// ```
/// use tomlfuse::file;
///
/// file! {
///     "examples/app.toml"
///
///     [app]
///     app.*
///
///     [logging]
///     logging.*
///     !logging.targets
/// }
///
/// fn main() {
///     println!("{} {}", app::NAME, app::VERSION);
///     println!("logging at {} as {}", logging::LEVEL, logging::FORMAT);
/// }
/// ```
///
/// A string binds as `&'static str`, an integer as `i64`, a float as `f64`, a boolean as
/// `bool` and an array of one element type as `&'static [T]`, so none of it needs `std` on
/// the consumer's side. A datetime comes across as its string form, and an array mixing
/// types lands as a single string of its debug form. Comments in the file become doc
/// comments on what they sat above. [`package!`] and [`workspace!`] are this with the path
/// left out and the manifest found from the crate instead.
#[proc_macro]
pub fn file(input: TokenStream) -> TokenStream {
    __codegen(input, None) // we require the path to be passed in the macro, so we can directly do this
}

fn __codegen(input: TokenStream, src: Option<PathBuf>) -> TokenStream {
    let ts: TokenStream = if let Some(path) = src {
        // for better dx, the path can be omitted in macro input, we'll prepend it for convenience here
        // (requires the caller to pass us something in `src` though)
        let path_str: LitStr = LitStr::new(&path.to_string_lossy(), Span::call_site());
        let mut _ts2: TokenStream2 = quote! {
            #path_str
        };
        _ts2.extend::<TokenStream2>(input.into());
        _ts2.into()
    } else {
        input
    };
    let macro_input: MacroInput = parse_macro_input!(ts as MacroInput);
    quote! {#macro_input}.into()
}
