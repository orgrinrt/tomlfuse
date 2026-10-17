//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Problem;
use crate::model::{Lit, Ty};
use toml::Value;

/// The type of a value, before an empty array has had its say.
///
/// An empty array holds nothing that says what kind it holds, so it takes the kind of the
/// arrays beside it. That cannot be decided element by element, which is why this is kept
/// apart from [`Ty`] until the whole value has been seen.
enum Shape {
    Leaf(Ty),
    /// An array, whose elements are of this shape, or of none yet when it is empty.
    List(Option<Box<Shape>>),
}

impl Shape {
    fn into_ty(self) -> Ty {
        match self {
            Shape::Leaf(ty) => ty,
            // Nothing says what an array that stayed empty holds, and `&[]` is every type.
            Shape::List(None) => Ty::Slice(Box::new(Ty::Str)),
            Shape::List(Some(elem)) => Ty::Slice(Box::new(elem.into_ty())),
        }
    }

    /// What to call this in a message.
    fn kind(&self) -> &'static str {
        match self {
            Shape::Leaf(Ty::Str) => "a string",
            Shape::Leaf(Ty::Int) => "an integer",
            Shape::Leaf(Ty::Float) => "a float",
            Shape::Leaf(Ty::Bool) => "a boolean",
            Shape::Leaf(Ty::Slice(_)) | Shape::List(_) => "an array",
        }
    }
}

/// The one shape two shapes agree on, or the two kinds that do not.
fn merge(a: Shape, b: Shape) -> Result<Shape, (&'static str, &'static str)> {
    match (a, b) {
        (Shape::Leaf(a), Shape::Leaf(b)) if a == b => Ok(Shape::Leaf(a)),
        (Shape::List(None), Shape::List(other)) | (Shape::List(other), Shape::List(None)) => {
            Ok(Shape::List(other))
        },
        (Shape::List(Some(a)), Shape::List(Some(b))) => Ok(Shape::List(Some(Box::new(merge(*a, *b)?)))),
        (a, b) => Err((a.kind(), b.kind())),
    }
}

fn refuse(key: &str, reason: impl Into<String>) -> Problem {
    Problem {
        key: key.to_string(),
        reason: reason.into(),
    }
}

/// Converts a toml `Value` into the type it binds as and the value it binds to.
///
/// Supports strings, integers, floats, booleans and arrays of one kind, which may be arrays
/// in turn. Anything else has no constant to bind as and is a [`Problem`] naming `key`: an
/// array of mixed kinds, a table inside an array, and a datetime. They used to be bound as
/// the text of their `Debug` or `Display` output, so a row that was meant to be a record
/// built into a string, and failed, or did not, wherever it was read.
#[cold]
pub fn bind_value(key: &str, value: &Value) -> Result<(Ty, Lit), Problem> {
    let (shape, lit) = shaped(key, value)?;
    Ok((shape.into_ty(), lit))
}

fn shaped(key: &str, value: &Value) -> Result<(Shape, Lit), Problem> {
    match value {
        Value::String(s) => Ok((Shape::Leaf(Ty::Str), Lit::Str(s.clone()))),
        Value::Integer(i) => Ok((Shape::Leaf(Ty::Int), Lit::Int(*i))),
        Value::Float(f) => Ok((Shape::Leaf(Ty::Float), Lit::Float(*f))),
        Value::Boolean(b) => Ok((Shape::Leaf(Ty::Bool), Lit::Bool(*b))),
        // TODO: proper DateTime support via chrono or similar
        Value::Datetime(_) => Err(refuse(
            key,
            "is a datetime, which has no type to bind as. Write it as a string if its text is \
             what is wanted.",
        )),
        // A table on its own is a module and never gets here, so this is one inside an array.
        Value::Table(_) => Err(refuse(
            key,
            "is a table inside an array, which cannot be bound. Give each one a table of its \
             own in the file instead.",
        )),
        Value::Array(arr) => {
            let mut shape: Option<Shape> = None;
            let mut elems = Vec::with_capacity(arr.len());
            for (i, elem) in arr.iter().enumerate() {
                let (elem_shape, elem_lit) = shaped(&format!("{key}[{i}]"), elem)?;
                shape = Some(match shape {
                    None => elem_shape,
                    Some(so_far) => merge(so_far, elem_shape).map_err(|(a, b)| {
                        refuse(
                            key,
                            format!(
                                "holds both {a} and {b}, and a constant has one type. Make \
                                 every element the same kind."
                            ),
                        )
                    })?,
                });
                elems.push(elem_lit);
            }
            Ok((Shape::List(shape.map(Box::new)), Lit::List(elems)))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Binds the value written as `value = <text>`.
    fn bound(text: &str) -> Result<(Ty, Lit), Problem> {
        let parsed: Value = format!("value = {text}").parse().unwrap();
        bind_value("value", parsed.get("value").unwrap())
    }

    fn slice(of: Ty) -> Ty {
        Ty::Slice(Box::new(of))
    }

    #[test]
    fn test_string_value_conversion() {
        let (ty, val) = bind_value("k", &Value::String("test".to_string())).unwrap();
        assert_eq!(ty, Ty::Str);
        assert_eq!(val, Lit::Str("test".into()));
    }

    #[test]
    fn test_numeric_values() {
        assert_eq!(bind_value("k", &Value::Integer(42)).unwrap(), (Ty::Int, Lit::Int(42)));
        assert_eq!(bind_value("k", &Value::Float(PI)).unwrap(), (Ty::Float, Lit::Float(PI)));
    }

    #[test]
    fn test_boolean_value() {
        assert_eq!(bound("true").unwrap(), (Ty::Bool, Lit::Bool(true)));
        assert_eq!(bound("false").unwrap().1, Lit::Bool(false));
    }

    #[test]
    fn test_homogeneous_array() {
        let (ty, val) = bound(r#"["a", "b"]"#).unwrap();
        assert_eq!(ty, slice(Ty::Str));
        assert_eq!(val, Lit::List(vec![Lit::Str("a".into()), Lit::Str("b".into())]));
    }

    #[test]
    fn test_empty_array() {
        let (ty, val) = bound("[]").unwrap();
        assert_eq!(ty, slice(Ty::Str));
        assert_eq!(val, Lit::List(vec![]));
    }

    #[test]
    fn an_array_of_arrays_is_a_slice_of_slices() {
        let (ty, _) = bound("[[1, 2], [3]]").unwrap();
        assert_eq!(ty, slice(slice(Ty::Int)));
    }

    #[test]
    fn an_empty_array_takes_the_kind_of_the_arrays_beside_it() {
        // Whichever side it is on. Taking the first element's type for the lot typed the
        // first of these as arrays of strings, and the integers inside it would not compile.
        assert_eq!(bound("[[], [1, 2]]").unwrap().0, slice(slice(Ty::Int)));
        assert_eq!(bound("[[1, 2], []]").unwrap().0, slice(slice(Ty::Int)));
        assert_eq!(bound("[[], [[1]]]").unwrap().0, slice(slice(slice(Ty::Int))));
        // With nothing beside it to say, it stays an array of strings, as it always was.
        assert_eq!(bound("[[], []]").unwrap().0, slice(slice(Ty::Str)));
    }

    #[test]
    fn a_mixed_array_is_refused_naming_the_key_and_the_kinds() {
        let err = bound(r#"[1, "two"]"#).unwrap_err();
        assert_eq!(err.key, "value");
        assert!(err.reason.contains("an integer and a string"), "{err}");
        // Two numbers of different kinds are still two kinds.
        assert!(bound("[1, 2.5]").unwrap_err().reason.contains("an integer and a float"));
    }

    #[test]
    fn a_mix_inside_an_inner_array_is_refused_too() {
        let err = bound(r#"[[1], ["a"]]"#).unwrap_err();
        assert!(err.reason.contains("an integer and a string"), "{err}");
        let err = bound("[[1], 2]").unwrap_err();
        assert!(err.reason.contains("an array and an integer"), "{err}");
    }

    #[test]
    fn a_table_in_an_array_is_refused_naming_the_element() {
        let err = bound(r#"[{ name = "a" }, { name = "b" }]"#).unwrap_err();
        assert_eq!(err.key, "value[0]");
        assert!(err.reason.contains("table inside an array"), "{err}");
    }

    #[test]
    fn a_datetime_is_refused_wherever_it_is() {
        let err = bound("2026-10-17T09:30:00Z").unwrap_err();
        assert_eq!(err.key, "value");
        assert!(err.reason.contains("datetime"), "{err}");

        let err = bound("[[1], [2026-10-17T09:30:00Z]]").unwrap_err();
        assert_eq!(err.key, "value[1][0]");
    }

    #[test]
    fn a_problem_reads_as_a_sentence_about_its_key() {
        let err = bound("2026-10-17T09:30:00Z").unwrap_err();
        assert_eq!(
            err.to_string(),
            "tomlfuse: `value` is a datetime, which has no type to bind as. Write it as a \
             string if its text is what is wanted."
        );
    }
}
