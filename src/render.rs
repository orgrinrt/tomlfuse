//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! Turns what `tomlfuse-core` bound into the Rust the macros expand to.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::LitStr;
use tomlfuse_core::{Lit, Module, Source, Ty};

/// The tokens for the type a constant is declared with.
pub fn ty(ty: &Ty) -> TokenStream2 {
    match ty {
        Ty::Str => quote! { &'static str },
        Ty::Int => quote! { i64 },
        Ty::Float => quote! { f64 },
        Ty::Bool => quote! { bool },
        Ty::Slice(elem) => {
            let elem = self::ty(elem);
            quote! { &'static [#elem] }
        },
    }
}

/// The tokens for the value a constant is given.
pub fn lit(lit: &Lit) -> TokenStream2 {
    match lit {
        Lit::Str(s) => quote! { #s },
        Lit::Int(i) => quote! { #i },
        Lit::Float(f) => quote! { #f },
        Lit::Bool(b) => quote! { #b },
        Lit::List(elems) => {
            let elems = elems.iter().map(self::lit);
            quote! { &[#(#elems),*] }
        },
    }
}

/// Wraps a field's comment into a `#[doc = "..."]` attribute token.
///
/// Preserves the field's original comment formatting if available.
///
/// Returns empty tokens if the field has no comment.
#[inline]
pub fn get_doc_comment(comment: Option<&str>) -> TokenStream2 {
    // TODO: actually let's try and parse empty comment lines as \n instead of wholesale
    let comment = comment.unwrap_or_default();
    let lit = LitStr::new(comment, proc_macro2::Span::call_site());
    if comment.is_empty() {
        quote! {}
    } else {
        quote! {
            #[doc = #lit]
        }
    }
}

/// Registers the toml with cargo, so that editing it rebuilds what was generated from it.
///
/// A procedural macro reads files behind cargo's back: nothing in a macro invocation tells
/// it that the output depends on anything but the source file the invocation sits in.
/// Without this, editing the toml and rebuilding leaves the old constants in place, and the
/// build reports success.
///
/// `include_bytes!` is what does it. `proc_macro::tracked_path` is the direct way and is
/// unstable. The bytes are bound to an anonymous constant, so this costs a name nobody can
/// collide with and nothing at runtime.
pub fn tracker(source: &Source) -> TokenStream2 {
    let path = source.path().to_string_lossy();
    let path = LitStr::new(&path, proc_macro2::Span::call_site());
    quote! {
        const _: &[u8] = include_bytes!(#path);
    }
}

/// A diagnostic at the macro invocation, in place of the constants that could not be made.
pub fn compile_error(message: &str) -> TokenStream2 {
    quote! { ::core::compile_error!(#message); }
}

/// The module a `[name]` block becomes, with everything bound into it.
pub fn section(name: &Ident, module: &Module, source: &Source) -> TokenStream2 {
    // A section is named in the macro invocation rather than in the toml, so there is no
    // comment anywhere that could document it, and it shipped with none. That made this
    // crate unusable from any crate carrying `#![deny(missing_docs)]`: the lint fires on
    // the generated module, spanned at the invocation, where no `#[allow]` the consumer
    // writes can reach it. Saying where the contents came from is the documentation a
    // reader of `cargo doc` wanted anyway.
    let root_mod_doc = {
        let path = source.path();
        let text = format!(
            "Constants bound from `{}`.",
            path.file_name()
                .map_or_else(|| path.to_string_lossy(), |name| name.to_string_lossy())
        );
        quote! { #[doc = #text] }
    };
    let contents = contents(module);

    quote! {
        // Lints that fire on a value rather than on how it was written do not belong
        // on generated code: the value came from the consumer's toml and the code
        // came from here, so neither party can act on the diagnostic. A toml holding
        // 3.14159 made `clippy::approx_constant` a deny-level error spanned at the
        // macro invocation, which no `#[allow]` in the consumer's own source reaches.
        //
        // `missing_docs` is the same thing pointed at documentation. A comment in the
        // toml becomes a `#[doc]` and shows up in `cargo doc`, which is the feature; a
        // key without one is a key somebody chose not to comment, and their config file
        // is not the place a crate's documentation policy belongs. It is unsatisfiable
        // besides, since a table that a dotted key brings into being has nowhere to put
        // a comment: `a.b = 1` documents `b`, and `a` never gets a line of its own.
        #[allow(
            clippy::approx_constant,
            clippy::excessive_precision,
            clippy::unreadable_literal,
            missing_docs,
        )]
        #root_mod_doc
        pub mod #name {
            #contents
        }
    }
}

/// The constants of a module and then the modules inside it.
fn contents(module: &Module) -> TokenStream2 {
    let mut tokens = TokenStream2::new();

    for constant in &module.consts {
        let name = format_ident!("{}", constant.name);
        let ty = ty(&constant.ty);
        let value = lit(&constant.value);
        let comment = get_doc_comment(constant.comment.as_deref());
        tokens.extend(quote! {
            #comment
            pub const #name: #ty = #value;
        });
    }

    for inner in &module.modules {
        let body = contents(inner);
        // A module with no name of its own contributes its contents to the enclosing one
        // rather than opening one.
        tokens.extend(match &inner.name {
            Some(name) => {
                let name = format_ident!("{}", name);
                let comment = get_doc_comment(inner.comment.as_deref());
                quote! {
                    #comment
                    pub mod #name {
                        #body
                    }
                }
            },
            None => body,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: the escaped quotes for string values are expected, because they need to be
    //       string literals in the generated code

    #[test]
    fn test_string_value_tokens() {
        assert_eq!(ty(&Ty::Str).to_string(), "& 'static str");
        assert_eq!(lit(&Lit::Str("test".into())).to_string(), "\"test\"");
    }

    #[test]
    fn test_numeric_value_tokens() {
        assert_eq!(ty(&Ty::Int).to_string(), "i64");
        // numeric literal includes type suffix in output
        assert_eq!(lit(&Lit::Int(42)).to_string(), "42i64");
        assert_eq!(ty(&Ty::Float).to_string(), "f64");
        assert!(lit(&Lit::Float(std::f64::consts::PI)).to_string().starts_with("3.14"));
    }

    #[test]
    fn test_boolean_value_tokens() {
        assert_eq!(ty(&Ty::Bool).to_string(), "bool");
        assert_eq!(lit(&Lit::Bool(true)).to_string(), "true");
        assert_eq!(lit(&Lit::Bool(false)).to_string(), "false");
    }

    #[test]
    fn test_array_tokens() {
        let strings = Ty::Slice(Box::new(Ty::Str));
        assert_eq!(ty(&strings).to_string(), "& 'static [& 'static str]");
        let list = lit(&Lit::List(vec![Lit::Str("a".into()), Lit::Str("b".into())])).to_string();
        assert!(list.contains("\"a\""));
        assert!(list.contains("\"b\""));
        // this is proper form because tokens display with space delims
        assert_eq!(lit(&Lit::List(vec![])).to_string(), "& []");
    }

    #[test]
    fn test_get_doc_comment_empty() {
        assert_eq!(get_doc_comment(None).to_string(), "");
    }

    #[test]
    fn test_get_doc_comment_with_escaping() {
        let doc = get_doc_comment(Some("with `code` and 'quotes'")).to_string();
        // verify doc comment contains the basic content
        assert!(doc.contains("with"));
        assert!(doc.contains("code"));
        assert!(doc.contains("quotes"));
    }
}
