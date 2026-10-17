//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::model::{Lit, Ty};
use toml::Value;

/// Converts a toml `Value` into the type it binds as and the value it binds to.
///
/// Supports strings, integers, floats, booleans, datetimes, and homogeneous arrays.
/// Falls back to string representation for complex/mixed types (which should not be used/valid anyway).
#[cold]
pub fn bind_value(value: &Value) -> (Ty, Lit) {
    match value {
        Value::String(s) => (Ty::Str, Lit::Str(s.clone())),
        Value::Integer(i) => (Ty::Int, Lit::Int(*i)),
        Value::Float(f) => (Ty::Float, Lit::Float(*f)),
        Value::Boolean(b) => (Ty::Bool, Lit::Bool(*b)),
        Value::Datetime(dt) => {
            // TODO: proper DateTime support via chrono or similar
            (Ty::Str, Lit::Str(dt.to_string()))
        },
        Value::Array(arr) => {
            if arr.is_empty() {
                (Ty::Slice(Box::new(Ty::Str)), Lit::List(Vec::new()))
            } else {
                // single-step recurse to get type and value for the first element
                let (elem_ty, _) = bind_value(&arr[0]);
                // check all elements are of the same variant as the first (should be the case for most use cases)
                let same_type = arr
                    .iter()
                    .all(|v| std::mem::discriminant(v) == std::mem::discriminant(&arr[0]));
                if same_type {
                    let elems: Vec<_> = arr.iter().map(|v| bind_value(v).1).collect();
                    (Ty::Slice(Box::new(elem_ty)), Lit::List(elems))
                } else {
                    // fallback for mixed types
                    (Ty::Str, Lit::Str(format!("{:?}", arr)))
                }
            }
        },
        _ => {
            // fallback for unsupported types, are there any we should support?
            (Ty::Str, Lit::Str(format!("{}", value)))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_string_value_conversion() {
        let (ty, val) = bind_value(&Value::String("test".to_string()));
        assert_eq!(ty, Ty::Str);
        assert_eq!(val, Lit::Str("test".into()));
    }

    #[test]
    fn test_numeric_values() {
        assert_eq!(bind_value(&Value::Integer(42)), (Ty::Int, Lit::Int(42)));
        assert_eq!(bind_value(&Value::Float(PI)), (Ty::Float, Lit::Float(PI)));
    }

    #[test]
    fn test_boolean_value() {
        assert_eq!(bind_value(&Value::Boolean(true)), (Ty::Bool, Lit::Bool(true)));
        assert_eq!(bind_value(&Value::Boolean(false)).1, Lit::Bool(false));
    }

    #[test]
    fn test_datetime_value() {
        // parse a toml string containing a datetime to get a Value::Datetime
        // TODO: see if we can somehow, from somewhere, import and directly use the toml_datetime::DateTime...?
        let parsed: Value = r#"date = 2023-01-01T12:00:00Z"#.parse().unwrap();
        let (ty, val) = bind_value(parsed.get("date").unwrap());

        // a string type, per the implementation
        assert_eq!(ty, Ty::Str);
        // the value contains the date string (exact format may vary)
        let Lit::Str(text) = val else { panic!("a datetime binds as a string") };
        assert!(text.contains("2023-01-01"));
        assert!(text.contains("12:00:00"));
    }

    #[test]
    fn test_homogeneous_array() {
        let strings = Value::Array(vec![Value::String("a".into()), Value::String("b".into())]);
        let (ty, val) = bind_value(&strings);
        assert_eq!(ty, Ty::Slice(Box::new(Ty::Str)));
        assert_eq!(val, Lit::List(vec![Lit::Str("a".into()), Lit::Str("b".into())]));
    }

    #[test]
    fn test_empty_array() {
        let (ty, val) = bind_value(&Value::Array(vec![]));
        assert_eq!(ty, Ty::Slice(Box::new(Ty::Str)));
        assert_eq!(val, Lit::List(vec![]));
    }

    #[test]
    fn test_mixed_array_fallback() {
        let mixed = Value::Array(vec![Value::String("a".into()), Value::Integer(1)]);
        let (ty, val) = bind_value(&mixed);
        assert_eq!(ty, Ty::Str);
        // debug representation of mixed array
        assert_eq!(val, Lit::Str(format!("[String(\"{}\"), Integer({})]", "a", 1)));
    }
}
