//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::field::ROOT;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value;

pub fn convert_value_to_tokens(value: &Value) -> (TokenStream2, TokenStream2) {
    match value {
        Value::String(s) => (quote! { &'static str }, quote! { #s }),
        Value::Integer(i) => (quote! { i64 }, quote! { #i }),
        Value::Float(f) => (quote! { f64 }, quote! { #f }),
        Value::Boolean(b) => (quote! { bool }, quote! { #b }),
        Value::Datetime(dt) => {
            let dt_str = dt.to_string();
            (quote! { DateTime }, quote! { #dt_str })
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                (quote! { &'static [&'static str] }, quote! { &[] })
            } else {
                // single-step recurse to get type and value for the first element
                let (elem_ty, _) = convert_value_to_tokens(&arr[0]);
                // check all elements are of the same variant as the first (should be the case for most use cases)
                let same_type = arr.iter().all(|v| std::mem::discriminant(v) == std::mem::discriminant(&arr[0]));
                if same_type {
                    let elems: Vec<_> = arr.iter().map(|v| {
                        let (_, val) = convert_value_to_tokens(v);
                        val
                    }).collect();
                    (quote! { &'static [#elem_ty] }, quote! { &[#(#elems),*] })
                } else {
                    // fallback for mixed types
                    let array_str = format!("{:?}", arr);
                    (quote! { &'static str }, quote! { #array_str })
                }
            }
        }
        _ => {
            // fallback for unsupported types, are there any we should support?
            let val_str = format!("{}", value);
            (quote! { &'static str }, quote! { #val_str })
        }
    }
}

pub fn value_to_string_token(value: &Value) -> TokenStream2 {
    match value {
        Value::String(s) => quote! { #s },
        _ => {
            let s = value.to_string();
            quote! { #s }
        }
    }
}

pub fn find_workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(manifest_dir);

    // search upwards for workspace root
    let mut path = manifest_path.clone();

    while !is_workspace_root(&path.join("Cargo.toml")) {
        if !path.pop() {
            // fallback to pkg dir if no workspace found
            return manifest_path;
        }
    }
    path
}

pub fn is_workspace_root(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(toml) = content.parse::<Value>() {
            return toml.get("workspace").is_some();
        }
    }
    false
}

pub fn to_valid_ident(input: &str) -> String {
    // handle potentially somehow still quoted keys by removing quotes
    let i = input.trim_start_matches('"').trim_end_matches('"');
    // empty input handling
    if i.is_empty() {
        return ROOT.to_string(); // default name
    }
    fix_dashes(i)
}

// separate this logic for future expansion so that it applies outside of valid idents too
pub fn fix_dashes(input: &str) -> String {
    input.replace('-', "_")
}
