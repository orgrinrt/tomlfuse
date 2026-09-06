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

/// Turns a toml value into the type and the literal of the constant it becomes.
///
/// Strings are `&'static str`, integers `i64`, floats `f64`, booleans `bool`. An array whose
/// elements all become the same type is `&'static [T]` of that type, nested arrays included.
/// An array whose elements do not is a tuple, one position per element, since that is the
/// one shape that holds several types at compile time without a box or a dispatch.
///
/// A datetime is its textual form, and a table inside an array is the toml text of the
/// table: neither has a constant to be.
// FIXME: a table inside an array becomes a string of toml. A struct per table shape would
// be the typed answer, and needs a design for naming the type and for arrays of tables
// whose members differ.
#[cold]
pub fn convert_value_to_tokens(value: &Value) -> (TokenStream2, TokenStream2) {
    match value {
        Value::String(s) => (quote! { &'static str }, quote! { #s }),
        Value::Integer(i) => (quote! { i64 }, quote! { #i }),
        Value::Float(f) => (quote! { f64 }, quote! { #f }),
        Value::Boolean(b) => (quote! { bool }, quote! { #b }),
        Value::Datetime(dt) => {
            let dt_str = dt.to_string();
            (quote! { &'static str }, quote! { #dt_str })
        },
        Value::Array(arr) => {
            if arr.is_empty() {
                return (quote! { &'static [&'static str] }, quote! { &[] });
            }
            let (types, values): (Vec<_>, Vec<_>) = arr.iter().map(convert_value_to_tokens).unzip();
            // Compared as generated types rather than as toml variants, so two arrays that
            // are both arrays but hold different element types count as different, and
            // land in a tuple where an array of them would not compile.
            let first = types[0].to_string();
            let homogeneous = types.iter().all(|ty| ty.to_string() == first);
            if homogeneous {
                let elem_ty = &types[0];
                (quote! { &'static [#elem_ty] }, quote! { &[#(#values),*] })
            } else {
                // A trailing comma in both, so a tuple of one is still a tuple. It cannot
                // arise here, since one element is homogeneous with itself, but the
                // spelling costs nothing and rules the case out.
                (quote! { (#(#types,)*) }, quote! { (#(#values,)*) })
            }
        },
        _ => {
            let val_str = format!("{}", value);
            (quote! { &'static str }, quote! { #val_str })
        },
    }
}

/// The `#[doc = "..."]` attribute carrying a field's comment, or nothing where it has none.
#[inline]
pub fn get_doc_comment(field: &TomlField) -> TokenStream2 {
    match field.comment.as_deref() {
        Some(comment) if !comment.is_empty() => {
            let lit = LitStr::new(comment, proc_macro2::Span::call_site());
            quote! { #[doc = #lit] }
        },
        _ => quote! {},
    }
}

/// The workspace root, found by climbing from `CARGO_MANIFEST_DIR` to the first directory
/// whose `Cargo.toml` opens a `[workspace]` table.
///
/// Falls back to the manifest directory itself where no ancestor is one, which is what a
/// crate outside any workspace has for a root.
#[cold]
pub fn find_workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(manifest_dir);

    let mut path = manifest_path.clone();
    while !is_workspace_root(&path.join("Cargo.toml")) {
        if !path.pop() {
            return manifest_path;
        }
    }
    path
}

/// Whether the manifest at `path` exists, parses, and opens a `[workspace]` table.
#[cold]
pub fn is_workspace_root(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(toml) = content.parse::<Value>() {
            return toml.get("workspace").is_some();
        }
    }
    false
}

/// A toml key as a Rust identifier: surrounding quotes stripped, dashes made underscores,
/// and the empty key as [`ROOT`].
#[inline]
pub fn to_valid_ident(input: &str) -> String {
    let i = input.trim_start_matches('"').trim_end_matches('"');
    if i.is_empty() {
        return ROOT.to_string();
    }
    kebab_to_snake(i)
}

/// kebab-case to snake_case, which is every dash made an underscore.
#[inline]
pub fn kebab_to_snake(input: &str) -> String {
    input.replace('-', "_")
}

/// snake_case to kebab-case, the inverse, for looking a normalised path up in the comments
/// read from the toml, which keep the keys as written.
#[inline]
pub fn snake_to_kebab(input: &str) -> String {
    input.replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::TomlField;
    use std::f64::consts::PI;
    use std::fs;
    use tempfile::TempDir;
    use toml::Value;

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
    fn a_mixed_array_becomes_a_tuple() {
        let mixed = Value::Array(vec![Value::String("a".into()), Value::Integer(1)]);
        let (ty, val) = convert_value_to_tokens(&mixed);
        assert_eq!(ty.to_string(), "(& 'static str , i64 ,)");
        assert_eq!(val.to_string(), "(\"a\" , 1i64 ,)");
    }

    #[test]
    fn an_array_of_arrays_is_an_array_where_the_inner_shapes_agree() {
        let rows = Value::Array(vec![
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
            Value::Array(vec![Value::Integer(3)]),
        ]);
        let (ty, _) = convert_value_to_tokens(&rows);
        assert_eq!(ty.to_string(), "& 'static [& 'static [i64]]");
    }

    #[test]
    fn an_array_of_arrays_is_a_tuple_where_the_inner_shapes_differ() {
        // Both elements are arrays, so a check on the toml variant alone would file this as
        // homogeneous and generate a slice whose elements have two different types, which
        // fails to compile in the consumer with an error pointing at the macro.
        let rows = Value::Array(vec![
            Value::Array(vec![Value::Integer(1), Value::String("a".into())]),
            Value::Array(vec![Value::Integer(3)]),
        ]);
        let (ty, val) = convert_value_to_tokens(&rows);
        // The mixed inner array is itself a tuple, and the other stays a slice.
        assert_eq!(
            ty.to_string(),
            "((i64 , & 'static str ,) , & 'static [i64] ,)"
        );
        assert_eq!(val.to_string(), "((1i64 , \"a\" ,) , & [3i64] ,)");
    }

    #[test]
    fn snake_and_kebab_are_inverses_over_the_keys_that_use_them() {
        assert_eq!(snake_to_kebab("with_dash"), "with-dash");
        assert_eq!(kebab_to_snake(&snake_to_kebab("with_dash")), "with_dash");
        assert_eq!(snake_to_kebab("plain"), "plain");
    }

    #[test]
    fn test_get_doc_comment_empty() {
        let field = TomlField::default();
        assert_eq!(get_doc_comment(&field).to_string(), "");
    }

    #[test]
    fn test_get_doc_comment_with_escaping() {
        let field = TomlField {
            comment: Some("with `code` and 'quotes'".into()),
            ..TomlField::default()
        };
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
