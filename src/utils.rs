//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value;

pub fn convert_value_to_tokens(value: &Value) -> (TokenStream2, TokenStream2) {
    match value {
        Value::String(s) => (quote! { &'static str }, quote! { #s }),
        Value::Integer(i) => {
            let i_val = *i as i64;
            (quote! { i64 }, quote! { #i_val })
        }
        Value::Float(f) => {
            (quote! { f64 }, quote! { #f })
        }
        Value::Boolean(b) => (
            quote! { bool },
            if *b { quote! { true } } else { quote! { false } }
        ),
        Value::Datetime(dt) => {
            let dt_str = dt.to_string();
            (quote! { DateTime }, quote! { #dt_str })
        }
        Value::Array(arr) => {
            if arr.iter().all(|v| matches!(v, Value::String(_))) {
                let strings: Vec<_> = arr.iter()
                    .filter_map(|v| {
                        if let Value::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                (quote! { &'static [&'static str] }, quote! { &[#(#strings),*] })
            } else {
                // fallback to string for other array types
                // FIXME: not very robust, should handle each type properly
                let array_str = format!("{:?}", arr);
                (quote! { &'static str }, quote! { #array_str })
            }
        }
        // fall back to string for other types
        _ => {
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
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
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
        return "_root".to_string();  // default name
    }
    i.replace('-', "_")
}
