//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::field::{TomlField, ROOT};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::path::{Path, PathBuf};
use std::{env, fs};
use syn::LitStr;
use toml::Value;

/// Converts a toml `Value` into a pair of tokens:
/// - First token represents the rust type (`&'static str`, `i64`, etc.)
/// - Second token represents the literal value (`"foo"`, `42`, etc.)
///
/// Supports strings, integers, floats, booleans, datetimes, and homogeneous arrays.
/// Falls back to string representation for complex/mixed types (which should not be used/valid anyway).
#[cold]
pub fn convert_value_to_tokens(value: &Value) -> (TokenStream2, TokenStream2) {
    match value {
        Value::String(s) => (quote! { &'static str }, quote! { #s }),
        Value::Integer(i) => (quote! { i64 }, quote! { #i }),
        Value::Float(f) => (quote! { f64 }, quote! { #f }),
        Value::Boolean(b) => (quote! { bool }, quote! { #b }),
        Value::Datetime(dt) => {
            // TODO: proper DateTime support via chrono or similar
            let dt_str = dt.to_string();
            (quote! { &'static str }, quote! { #dt_str })
        },
        Value::Array(arr) => {
            if arr.is_empty() {
                (quote! { &'static [&'static str] }, quote! { &[] })
            } else {
                // single-step recurse to get type and value for the first element
                let (elem_ty, _) = convert_value_to_tokens(&arr[0]);
                // check all elements are of the same variant as the first (should be the case for most use cases)
                let same_type = arr
                    .iter()
                    .all(|v| std::mem::discriminant(v) == std::mem::discriminant(&arr[0]));
                if same_type {
                    let elems: Vec<_> = arr
                        .iter()
                        .map(|v| {
                            let (_, val) = convert_value_to_tokens(v);
                            val
                        })
                        .collect();
                    (quote! { &'static [#elem_ty] }, quote! { &[#(#elems),*] })
                } else {
                    // fallback for mixed types
                    let array_str = format!("{:?}", arr);
                    (quote! { &'static str }, quote! { #array_str })
                }
            }
        },
        _ => {
            // fallback for unsupported types, are there any we should support?
            let val_str = format!("{}", value);
            (quote! { &'static str }, quote! { #val_str })
        },
    }
}

/// Converts a TOML `Value` to a string token representation.
///
/// String values are kept as-is, other types are converted to string form.
#[inline]
#[allow(dead_code)] // NOTE: useful api for future
pub fn value_to_string_token(value: &Value) -> TokenStream2 {
    match value {
        Value::String(s) => quote! { #s },
        _ => {
            // render any value as string token
            let s = value.to_string();
            if s.contains("& ") {
                // remove unnecessary space after ampersand if no lifetime
                let s = s.replace("& ", "&");
                quote! { #s }
            } else {
                // no processing required
                quote! { #s }
            }
        },
    }
}

/// Wraps a field's comment into a `#[doc = "..."]` attribute token.
///
/// Preserves the field's original comment formatting if available.
///
/// Returns empty tokens if the field has no comment.
#[inline]
pub fn get_doc_comment(field: &TomlField) -> TokenStream2 {
    // println!(" >> Figuring out the comment for field: {}", field.name);
    let comment_maybe = field.comment.clone();
    // let comment = comment_maybe.unwrap_or_default().replace("\n", "\n\n").replace("\\\n", "\n\n"); // proper newlines
    let comment = comment_maybe.unwrap_or_default(); // TODO: actually let's try and parse empty comment lines as \n instead of above wholesale
    let lit = LitStr::new(&comment, proc_macro2::Span::call_site());
    if comment.is_empty() {
        quote! {}
    } else {
        // println!("     >> It did have a comment! ({})", comment);
        quote! {
            #[doc = #lit]
        }
    }
}

/// Finds the workspace root by traversing upward from `CARGO_MANIFEST_DIR`.
///
/// Searches parent directories until it finds one with a Cargo.toml file
/// that contains a `[workspace]` table. This allows finding the workspace
/// root from any crate within the workspace.
///
/// # Returns
/// Falls back to the original manifest directory if no workspace root is found.
#[cold]
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

/// Determines if a path contains a workspace Cargo.toml file.
///
/// Checks if the file exists, can be read as TOML, and contains
/// a `[workspace]` table.
/// # Parameters
/// - `path`: Path to a potential Cargo.toml file
///
/// # Returns
/// `true` if the path exists, can be read, parsed as TOML, and has a workspace table.
#[cold]
pub fn is_workspace_root(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(toml) = content.parse::<Value>() {
            return toml.get("workspace").is_some();
        }
    }
    false
}

/// Converts a TOML key into a valid Rust identifier string.
///
/// Transformations applied:
///
/// 1. Strips surrounding quotes if present
/// 2. Replaces dashes with underscores (kebab-case to snake_case)
/// 3. Returns `ROOT` constant for empty input
///
/// # Parameters
/// - `input`: Raw TOML key to normalize
#[inline]
pub fn to_valid_ident(input: &str) -> String {
    // handle potentially somehow still quoted keys by removing quotes
    let i = input.trim_start_matches('"').trim_end_matches('"');
    // empty input handling
    if i.is_empty() {
        return ROOT.to_string(); // default name
    }
    kebab_to_snake(i)
}

/// Converts kebab-case to snake_case by replacing all dashes with underscores.
///
/// # Parameters
/// - `input`: String potentially containing dashes
///
/// Used for converting kebab-case to snake_case in toml keys.
#[inline]
pub fn kebab_to_snake(input: &str) -> String {
    // TODO: more sophisticated conversion and covering edge cases that I assume have to exist
    input.replace('-', "_")
}

/// Converts snake_case to kebab-case by replacing all underscores with dashes.
///
/// # Parameters
/// - `input`: String potentially containing underscores
///
/// Used for converting snake_case to kebab-case in TOML keys.
#[inline]
pub fn snake_to_kebab(input: &str) -> String {
    input.replace('_', "-")
}
// TODO: unit test for `snake_to_kebab`, but in practice unless we expand or add to this, it should do what it says on the tin

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::TomlField;
    use std::f64::consts::PI;
    use std::fs;
    use tempfile::TempDir;
    use toml::Value;

    // NOTE: the escaped quotes for string values are expected, because they need to be
    //       string literals in the generated code
    // FIXME: make this make more sense, the above note alone tells me this smells
    fn escaped(s: &str) -> String {
        if s.starts_with("\"") {
            // already escaped
            return s.to_string();
        }
        format!("\"{}\"", s)
    }

    #[test]
    fn test_string_value_conversion() {
        let value = Value::String("test".to_string());
        let (ty, val) = convert_value_to_tokens(&value);
        assert_eq!(ty.to_string(), "& 'static str");
        assert_eq!(val.to_string(), "\"test\"");
    }

    #[test]
    fn test_numeric_values() {
        let int = Value::Integer(42);
        let (ty, val) = convert_value_to_tokens(&int);
        assert_eq!(ty.to_string(), "i64");
        // numeric literal includes type suffix in output
        assert_eq!(val.to_string(), "42i64");
        let float = Value::Float(PI);
        let (ty, val) = convert_value_to_tokens(&float);
        assert_eq!(ty.to_string(), "f64");
        assert!(val.to_string().starts_with("3.14"));
    }

    #[test]
    fn test_boolean_value() {
        let t = Value::Boolean(true);
        let (ty, val) = convert_value_to_tokens(&t);
        assert_eq!(ty.to_string(), "bool");
        assert_eq!(val.to_string(), "true");

        let f = Value::Boolean(false);
        let (_, val) = convert_value_to_tokens(&f);
        assert_eq!(val.to_string(), "false");
    }

    #[test]
    fn test_datetime_value() {
        // parse a toml string containing a datetime to get a Value::Datetime
        // TODO: see if we can somehow, from somewhere, import and directly use the toml_datetime::DateTime...?
        let toml_str = r#"date = 2023-01-01T12:00:00Z"#;
        let parsed: toml::Value = toml_str.parse().unwrap();
        let date_value = parsed.get("date").unwrap();

        // extract type and value tokens
        let (ty, val) = convert_value_to_tokens(date_value);

        // assert that we get a string type (per the implementation)
        assert_eq!(ty.to_string(), "& 'static str");

        // assert the value contains the date string (exact format may vary)
        assert!(val.to_string().contains("2023-01-01"));
        assert!(val.to_string().contains("12:00:00"));
    }

    #[test]
    fn test_homogeneous_array() {
        let strings = Value::Array(vec![Value::String("a".into()), Value::String("b".into())]);
        let (ty, val) = convert_value_to_tokens(&strings);
        assert_eq!(ty.to_string(), "& 'static [& 'static str]");
        assert!(val.to_string().contains("\"a\""));
        assert!(val.to_string().contains("\"b\""));
    }

    #[test]
    fn test_empty_array() {
        let empty = Value::Array(vec![]);
        let (ty, val) = convert_value_to_tokens(&empty);
        assert_eq!(ty.to_string(), "& 'static [& 'static str]");
        assert_eq!(val.to_string(), "& []"); // this is proper form because tokens display with space delims
    }

    #[test]
    fn test_mixed_array_fallback() {
        let mixed = Value::Array(vec![Value::String("a".into()), Value::Integer(1)]);
        let (ty, val) = convert_value_to_tokens(&mixed);
        assert_eq!(ty.to_string(), "& 'static str");
        // debug representation of mixed array
        // NOTE: when a str value converts to a token, it gets escaped on display
        //       unsure whether or not this should be thus, or we should make it more sensible?
        let pat = format!("[String(\\\"{}\\\"), Integer({})]", "a", 1);
        assert!(
            val.to_string().contains(&pat),
            "{}, should contain: {}",
            val.to_string(),
            pat
        );
    }

    #[test]
    fn test_value_to_string_token() {
        let str_val = Value::String("hello".into());
        assert_eq!(
            value_to_string_token(&str_val).to_string(),
            escaped("hello")
        );

        let int_val = Value::Integer(42);
        // non-string values get quoted in string token output
        assert_eq!(value_to_string_token(&int_val).to_string(), escaped("42"));
        let bool_val = Value::Boolean(true);
        assert_eq!(
            value_to_string_token(&bool_val).to_string(),
            escaped("true")
        );
    }

    #[test]
    fn test_get_doc_comment_empty() {
        let field = TomlField::default();
        assert_eq!(get_doc_comment(&field).to_string(), "");
    }

    #[test]
    fn test_get_doc_comment_with_escaping() {
        let mut field = TomlField::default();
        field.comment = Some("with `code` and 'quotes'".into());
        let doc = get_doc_comment(&field).to_string();
        // assert!(doc.contains("with \\`code\\` and \\'quotes"));
        // verify doc comment contains the basic content
        assert!(doc.contains("with"));
        assert!(doc.contains("code"));
        assert!(doc.contains("quotes"));
    }

    #[test]
    fn test_find_workspace_root_setup() -> Result<(), Box<dyn std::error::Error>> {
        // create a temporary directory structure
        let temp = TempDir::new()?;
        let root = temp.path().to_path_buf();

        // create workspace structure
        let ws_root = root.join("workspace");
        let project = ws_root.join("project");
        fs::create_dir_all(&project)?;

        // create workspace Cargo.toml
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"project\"]",
        )?;

        // create project Cargo.toml
        fs::write(project.join("Cargo.toml"), "[package]\nname = \"test\"")?;

        // test with original manifest dir set to project
        let orig_dir = env::var("CARGO_MANIFEST_DIR").ok();
        env::set_var("CARGO_MANIFEST_DIR", project.to_string_lossy().to_string());

        // should find workspace root
        let found = find_workspace_root();
        assert_eq!(found, ws_root);

        // restore original env var
        if let Some(dir) = orig_dir {
            env::set_var("CARGO_MANIFEST_DIR", dir);
        }

        Ok(())
    }

    #[test]
    fn test_is_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let ws_toml = temp.path().join("Cargo.toml");
        let pkg_toml = temp.path().join("pkg").join("Cargo.toml");

        fs::create_dir_all(temp.path().join("pkg"))?;
        fs::write(&ws_toml, "[workspace]\nmembers = [\"pkg\"]")?;
        fs::write(&pkg_toml, "[package]\nname = \"pkg\"")?;

        assert!(is_workspace_root(&ws_toml));
        assert!(!is_workspace_root(&pkg_toml));
        assert!(!is_workspace_root(&temp.path().join("nonexistent.toml")));

        Ok(())
    }

    #[test]
    fn test_to_valid_ident() {
        assert_eq!(to_valid_ident(""), ROOT.to_string());
        assert_eq!(to_valid_ident("normal"), "normal");
        assert_eq!(to_valid_ident("with-dash"), "with_dash");
        assert_eq!(to_valid_ident("\"quoted\""), "quoted");
        assert_eq!(to_valid_ident("\"quoted-with-dash\""), "quoted_with_dash");
    }

    #[test]
    fn test_fix_dashes() {
        assert_eq!(kebab_to_snake("no-dashes-here"), "no_dashes_here");
        assert_eq!(kebab_to_snake("already_good"), "already_good");
        assert_eq!(kebab_to_snake("mixed-case_style"), "mixed_case_style");
        assert_eq!(kebab_to_snake("-leading-dash"), "_leading_dash");
        assert_eq!(kebab_to_snake("trailing-dash-"), "trailing_dash_");
    }
}
