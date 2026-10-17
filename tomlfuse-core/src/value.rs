//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Problem;
use crate::model::{FloatTy, IntTy, Lit, Ty, Width};
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
    /// An array of exactly this length, whose elements are all of this shape. An empty one
    /// holds the width's type, since the width is what says what it would have held.
    Array(Box<Shape>, usize),
}

/// Why two shapes are not one.
enum Clash {
    /// Different kinds of value, named for a message.
    Kinds(&'static str, &'static str),
    /// Fixed arrays of different lengths.
    Lengths(usize, usize),
}

impl Shape {
    fn into_ty(self) -> Ty {
        match self {
            Shape::Leaf(ty) => ty,
            // Nothing says what an array that stayed empty holds, and `&[]` is every type.
            Shape::List(None) => Ty::Slice(Box::new(Ty::Str)),
            Shape::List(Some(elem)) => Ty::Slice(Box::new(elem.into_ty())),
            Shape::Array(elem, len) => Ty::Array(Box::new(elem.into_ty()), len),
        }
    }

    /// What to call this in a message.
    fn kind(&self) -> &'static str {
        match self {
            Shape::Leaf(Ty::Str) => "a string",
            Shape::Leaf(Ty::Int(_)) => "an integer",
            Shape::Leaf(Ty::Float(_)) => "a float",
            Shape::Leaf(Ty::Bool) => "a boolean",
            Shape::Leaf(Ty::Slice(_) | Ty::Array(..)) | Shape::List(_) | Shape::Array(..) => {
                "an array"
            },
        }
    }
}

/// The one shape two shapes agree on, or the two kinds that do not.
fn merge(a: Shape, b: Shape) -> Result<Shape, Clash> {
    match (a, b) {
        (Shape::Leaf(a), Shape::Leaf(b)) if a == b => Ok(Shape::Leaf(a)),
        (Shape::List(None), Shape::List(other)) | (Shape::List(other), Shape::List(None)) => {
            Ok(Shape::List(other))
        },
        (Shape::List(Some(a)), Shape::List(Some(b))) => Ok(Shape::List(Some(Box::new(merge(*a, *b)?)))),
        (Shape::Array(_, a), Shape::Array(_, b)) if a != b => Err(Clash::Lengths(a, b)),
        (Shape::Array(a, len), Shape::Array(b, _)) => Ok(Shape::Array(Box::new(merge(*a, *b)?), len)),
        (a, b) => Err(Clash::Kinds(a.kind(), b.kind())),
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
/// in turn, which are slices, or fixed arrays when `width` is given. An integer is bound as `i64` and a float as `f64` unless `width` names another
/// type, which every number in the value is then bound as, and which a number that does not
/// fit is a [`Problem`] for. Anything else has no constant to bind as and is a [`Problem`]
/// naming `key`: an
/// array of mixed kinds, a table inside an array, and a datetime. They used to be bound as
/// the text of their `Debug` or `Display` output, so a row that was meant to be a record
/// built into a string, and failed, or did not, wherever it was read.
#[cold]
pub fn bind_value(
    key: &str,
    value: &Value,
    width: Option<Width>,
) -> Result<(Ty, Lit), Problem> {
    let (shape, lit) = shaped(key, value, width)?;
    Ok((shape.into_ty(), lit))
}

/// A width is for numbers. On anything else the pattern that gave it was too broad, and that
/// is said so rather than ignored.
fn not_a_number(key: &str, kind: &str, width: Width) -> Problem {
    refuse(
        key,
        format!("is {kind}, and the width `{}` applies to integers and floats only.", width.name()),
    )
}

fn shaped(key: &str, value: &Value, width: Option<Width>) -> Result<(Shape, Lit), Problem> {
    match value {
        Value::String(s) => match width {
            Some(width) => Err(not_a_number(key, "a string", width)),
            None => Ok((Shape::Leaf(Ty::Str), Lit::Str(s.clone()))),
        },
        Value::Boolean(b) => match width {
            Some(width) => Err(not_a_number(key, "a boolean", width)),
            None => Ok((Shape::Leaf(Ty::Bool), Lit::Bool(*b))),
        },
        Value::Integer(i) => match width {
            None => Ok((Shape::Leaf(Ty::Int(IntTy::I64)), Lit::Int(*i))),
            Some(Width::Int(ty)) => {
                let (min, max) = ty.bounds();
                if (min..=max).contains(&i128::from(*i)) {
                    Ok((Shape::Leaf(Ty::Int(ty)), Lit::Int(*i)))
                } else {
                    Err(refuse(
                        key,
                        format!("is {i}, which does not fit {} ({min} to {max}).", ty.name()),
                    ))
                }
            },
            Some(Width::Float(ty)) => Err(refuse(
                key,
                format!(
                    "is an integer, and `{}` is a float type. Write it with a decimal point in \
                     the file if a float is meant.",
                    ty.name()
                ),
            )),
        },
        Value::Float(f) => match width {
            None => Ok((Shape::Leaf(Ty::Float(FloatTy::F64)), Lit::Float(*f))),
            Some(Width::Float(ty)) => {
                // Infinity and NaN fit anything. A finite value too large to fit would become
                // infinity, which is not what the file says.
                if f.is_finite() && f.abs() > ty.largest() {
                    Err(refuse(
                        key,
                        format!(
                            "is {f:e}, which does not fit {} (magnitude up to {}).",
                            ty.name(),
                            ty.largest_text()
                        ),
                    ))
                } else {
                    Ok((Shape::Leaf(Ty::Float(ty)), Lit::Float(*f)))
                }
            },
            Some(Width::Int(ty)) => Err(refuse(
                key,
                format!("is a float, and `{}` is an integer type.", ty.name()),
            )),
        },
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
                let (elem_shape, elem_lit) = shaped(&format!("{key}[{i}]"), elem, width)?;
                shape = Some(match shape {
                    None => elem_shape,
                    Some(so_far) => merge(so_far, elem_shape).map_err(|clash| match clash {
                        Clash::Kinds(a, b) => refuse(
                            key,
                            format!(
                                "holds both {a} and {b}, and a constant has one type. Make \
                                 every element the same kind."
                            ),
                        ),
                        Clash::Lengths(a, b) => refuse(
                            key,
                            format!(
                                "holds arrays of different lengths, {a} and {b} (at [{i}]). A \
                                 fixed array needs them all the same; drop the width to bind it \
                                 as a slice."
                            ),
                        ),
                    })?,
                });
                elems.push(elem_lit);
            }
            Ok((
                match width {
                    // The width is given, so the lengths are the file's and the type is a
                    // fixed array of them.
                    Some(width) => Shape::Array(
                        Box::new(shape.unwrap_or(Shape::Leaf(width.ty()))),
                        elems.len(),
                    ),
                    None => Shape::List(shape.map(Box::new)),
                },
                Lit::List(elems),
            ))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Binds the value written as `value = <text>`.
    fn bound(text: &str) -> Result<(Ty, Lit), Problem> {
        bound_at(text, None)
    }

    /// The same at a width.
    fn bound_at(text: &str, width: Option<&str>) -> Result<(Ty, Lit), Problem> {
        let parsed: Value = format!("value = {text}").parse().unwrap();
        bind_value("value", parsed.get("value").unwrap(), width.map(|w| Width::parse(w).unwrap()))
    }

    const INT: Ty = Ty::Int(IntTy::I64);

    fn slice(of: Ty) -> Ty {
        Ty::Slice(Box::new(of))
    }

    #[test]
    fn test_string_value_conversion() {
        let (ty, val) = bind_value("k", &Value::String("test".to_string()), None).unwrap();
        assert_eq!(ty, Ty::Str);
        assert_eq!(val, Lit::Str("test".into()));
    }

    #[test]
    fn test_numeric_values() {
        assert_eq!(
            bind_value("k", &Value::Integer(42), None).unwrap(),
            (Ty::Int(IntTy::I64), Lit::Int(42))
        );
        assert_eq!(
            bind_value("k", &Value::Float(PI), None).unwrap(),
            (Ty::Float(FloatTy::F64), Lit::Float(PI))
        );
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
        assert_eq!(ty, slice(slice(INT)));
    }

    #[test]
    fn an_empty_array_takes_the_kind_of_the_arrays_beside_it() {
        // Whichever side it is on. Taking the first element's type for the lot typed the
        // first of these as arrays of strings, and the integers inside it would not compile.
        assert_eq!(bound("[[], [1, 2]]").unwrap().0, slice(slice(INT)));
        assert_eq!(bound("[[1, 2], []]").unwrap().0, slice(slice(INT)));
        assert_eq!(bound("[[], [[1]]]").unwrap().0, slice(slice(slice(INT))));
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

    #[test]
    fn an_integer_binds_at_the_width_named() {
        let (ty, val) = bound_at("17", Some("u8")).unwrap();
        assert_eq!(ty, Ty::Int(IntTy::U8));
        assert_eq!(val, Lit::Int(17));
        assert_eq!(bound_at("-12", Some("i16")).unwrap().0, Ty::Int(IntTy::I16));
    }

    #[test]
    fn an_integer_at_the_edge_of_its_width_fits_and_one_past_it_does_not() {
        assert!(bound_at("255", Some("u8")).is_ok());
        assert!(bound_at("0", Some("u8")).is_ok());
        assert!(bound_at("-128", Some("i8")).is_ok());
        assert!(bound_at("127", Some("i8")).is_ok());
        assert!(bound_at("9223372036854775807", Some("u64")).is_ok());

        for (text, width, bounds) in [
            ("256", "u8", "0 to 255"),
            ("-1", "u8", "0 to 255"),
            ("128", "i8", "-128 to 127"),
            ("-129", "i8", "-128 to 127"),
            ("65536", "u16", "0 to 65535"),
            ("-1", "u64", "0 to 18446744073709551615"),
        ] {
            let err = bound_at(text, Some(width)).unwrap_err();
            assert_eq!(err.key, "value");
            assert_eq!(
                err.reason,
                format!("is {text}, which does not fit {width} ({bounds})."),
                "{text} as {width}"
            );
        }
    }

    #[test]
    fn a_float_binds_at_the_width_named_and_a_float_too_large_does_not_fit() {
        assert_eq!(bound_at("0.5", Some("f32")).unwrap().0, Ty::Float(FloatTy::F32));
        assert_eq!(bound_at("1e300", Some("f64")).unwrap().0, Ty::Float(FloatTy::F64));
        assert!(bound_at("3.4028234e38", Some("f32")).is_ok());
        assert!(bound_at("-3.4028234e38", Some("f32")).is_ok());

        let err = bound_at("1e39", Some("f32")).unwrap_err();
        assert_eq!(
            err.reason,
            "is 1e39, which does not fit f32 (magnitude up to 3.4028235e38)."
        );
        assert!(bound_at("-1e39", Some("f32")).is_err());
        // Infinity is what the file says, and fits.
        assert!(bound_at("inf", Some("f32")).is_ok());
    }

    #[test]
    fn a_width_does_not_convert_between_integers_and_floats() {
        let err = bound_at("0.5", Some("u8")).unwrap_err();
        assert_eq!(err.reason, "is a float, and `u8` is an integer type.");
        let err = bound_at("3", Some("f32")).unwrap_err();
        assert!(err.reason.starts_with("is an integer, and `f32` is a float type."), "{err}");
    }

    #[test]
    fn a_width_on_anything_but_a_number_is_refused() {
        let err = bound_at(r#""text""#, Some("u32")).unwrap_err();
        assert_eq!(err.reason, "is a string, and the width `u32` applies to integers and floats only.");
        let err = bound_at("true", Some("u32")).unwrap_err();
        assert!(err.reason.starts_with("is a boolean"), "{err}");
    }

    fn array(of: Ty, len: usize) -> Ty {
        Ty::Array(Box::new(of), len)
    }

    #[test]
    fn the_width_reaches_every_number_in_an_array_and_names_the_one_that_does_not_fit() {
        let (ty, _) = bound_at("[[1, 2], [3, 4]]", Some("u8")).unwrap();
        assert_eq!(ty, array(array(Ty::Int(IntTy::U8), 2), 2));

        let err = bound_at("[[1, 2], [3, 300]]", Some("u8")).unwrap_err();
        assert_eq!(err.key, "value[1][1]");
        assert!(err.reason.contains("does not fit u8"), "{err}");
    }

    #[test]
    fn an_array_with_a_width_is_fixed_at_the_length_the_file_gave_it() {
        let (ty, val) = bound_at("[1, 2, 3]", Some("u8")).unwrap();
        assert_eq!(ty, array(Ty::Int(IntTy::U8), 3));
        assert_eq!(val, Lit::List(vec![Lit::Int(1), Lit::Int(2), Lit::Int(3)]));

        // Both lengths, the outer and the inner.
        let (ty, _) = bound_at("[[1, 2, 3], [4, 5, 6]]", Some("u8")).unwrap();
        assert_eq!(ty, array(array(Ty::Int(IntTy::U8), 3), 2));

        // Without a width the same value is the slices it always was.
        let (ty, _) = bound("[[1, 2, 3], [4, 5, 6]]").unwrap();
        assert_eq!(ty, slice(slice(INT)));
    }

    #[test]
    fn an_empty_array_with_a_width_is_an_empty_array_of_the_width() {
        assert_eq!(bound_at("[]", Some("u8")).unwrap().0, array(Ty::Int(IntTy::U8), 0));
        assert_eq!(bound_at("[[], []]", Some("f32")).unwrap().0, array(array(Ty::Float(FloatTy::F32), 0), 2));
    }

    #[test]
    fn rows_of_different_lengths_cannot_be_a_fixed_array() {
        let err = bound_at("[[1, 2, 3], [4, 5]]", Some("u8")).unwrap_err();
        assert_eq!(err.key, "value");
        assert!(err.reason.starts_with("holds arrays of different lengths, 3 and 2 (at [1])."), "{err}");

        // An empty row is a row of another length, not a gap to be filled in.
        assert!(bound_at("[[], [1]]", Some("u8")).is_err());
        // And the difference may be further in.
        assert!(bound_at("[[[1, 2]], [[1, 2, 3]]]", Some("u8")).is_err());
        // The same rows are fine as slices.
        assert!(bound("[[1, 2, 3], [4, 5]]").is_ok());
    }
}
