//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use input::MacroInput;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens, TokenStreamExt};
use std::env;
use std::path::PathBuf;
use syn::__private::TokenStream2;
use syn::{parse_macro_input, LitStr};

mod comments;
mod field;
mod input;
mod module;
mod pattern;
mod utils;

use utils::*;

/// Generate constants from the workspace Cargo.toml file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::workspace;
///
/// workspace! {
///     [workspace]
///     workspace.*
/// }
/// ```
#[proc_macro]
pub fn workspace(input: TokenStream) -> TokenStream {
    // find workspace root
    let root_path = find_workspace_root();
    let cargo_path = root_path.join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Generate constants from the current crate's Cargo.toml file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::package;
///
/// package! {
///     [package]
///     package.*
///     
///     [deps]
///     dependencies.*
/// }
/// ```
#[proc_macro]
pub fn package(input: TokenStream) -> TokenStream {
    // use manifest dir for crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let cargo_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    __codegen(input, Some(cargo_path))
}

/// Generate constants from an arbitrary TOML file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::file;
///
/// file!(
///     "tests/test.toml"
///
///     [app]
///     app.*
///     
///     [logging]
///     logging.*
/// );
/// ```
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
