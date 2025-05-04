//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

#![doc = stringify!(include!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]

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

/// Expands to a bound constants from the workspace's `Cargo.toml`.
///
/// Locates the workspace root by traversing up from the current crate,
/// then generates modules and constants for selected keys and sections.
///
/// # Pattern syntax
/// - Dot notation for key paths: `workspace.members`
/// - Wildcards for groups: `workspace.*`
/// - Negation for exclusions: `!workspace.excluded`
/// - Aliases for renaming: `alias new = old`
/// - Section headers for modules: `[workspace]`
///
/// Each section header creates a module; patterns select which keys to expose as constants.
///
/// # Example
/// ```
/// use tomlfuse::workspace;
///
/// workspace! {
///     // generate a module with bound workspace meta
///     [workspace]
///     workspace.*
/// }
///
/// // access the generated constants
/// fn main() {
///     for member in workspace::MEMBERS {
///         println!("Found workspace member: {}", member);
///     }
/// }
/// ```
///
/// See also: [`package!`], [`file!`]
#[proc_macro]
#[deprecated(since = "0.0.3", note = "This crate is deprecated. Please use the `confuse` crate instead.")]
pub fn workspace(input: TokenStream) -> TokenStream {
    // find workspace root
    let cargo_path = find_workspace_root().join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Expands to a module exposing constants from the current crate's `Cargo.toml`.
///
/// Enables compile-time access to desired package metadata, dependencies, features, and custom tables.
///
/// # Pattern syntax
/// - Dot notation for key paths: `package.name`
/// - Wildcards for groups: `dependencies.*`
/// - Negation for exclusions: `!package.metadata.excluded`
/// - Aliases for renaming: `alias new = old.path.to.replace`
/// - Section headers for modules: `[package]`
///
/// Each section header creates a module; patterns select which keys to expose as constants.
///
/// # Example
/// ```
/// use tomlfuse::package;
///
/// package! {
///     // extract package meta
///     [package]
///     package.*
///
///     // then extract the deps
///     [deps]
///     dependencies.*
/// }
///
/// // use the bound consts in your code
/// fn version_info() -> String {
///     format!("{} v{} by {}",
///         package::NAME, package::VERSION, package::AUTHORS[0])
/// }
/// fn tokio_info() -> String {
///     use deps::tokio::*;
///     format!("{} v{} by {}",
///         NAME, VERSION, AUTHORS[0])
/// }
/// ```
///
/// See also: [`crate::workspace!`], [`crate::file!`]
#[proc_macro]
#[deprecated(since = "0.0.3", note = "This crate is deprecated. Please use the `confuse` crate instead.")]
pub fn package(input: TokenStream) -> TokenStream {
    // use manifest dir for crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let cargo_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Expands to bound constants from any toml file.
///
/// The first argument is the path to the toml file (relative to crate root).
///
/// # Pattern syntax
/// - Dot notation for key paths: `foo.bar`
/// - Wildcards for groups: `baz.*`
/// - Negation for exclusions: `!foo.bar.excluded`
/// - Aliases for renaming: `alias new = old.path.to.replace`
/// - Section headers for modules: `[foo]`
///
/// Each section header creates a module; patterns select which keys to expose as constants.
///
/// # Example
/// ```
/// use tomlfuse::file;
///
/// file!(
///     // path to source toml
///     "tests/test.toml"
///
///     // extract app config
///     [app]
///     app.*
///     !app.logging.*
///     
///     [logging]
///     app.logging.*
/// );
///
/// // then use the generated consts
/// fn setup() {
///     println!("Starting {} v{}", app::NAME, app::VERSION;
///     set_log_level(logging::LEVEL);
/// ```
///
/// See also: [`workspace!`], [`package!`]
#[proc_macro]
#[deprecated(since = "0.0.3", note = "This crate is deprecated. Please use the `confuse` crate instead.")]
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
