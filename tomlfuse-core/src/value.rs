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
    /// Fixed arrays of different lengths, and how many arrays further in than the elements
    /// that were merged they are.
    Lengths(usize, usize, usize),
}

impl Clash {
    /// The same clash, one array further in.
    fn deeper(self) -> Clash {
        match self {
            Clash::Lengths(a, b, depth) => Clash::Lengths(a, b, depth + 1),
            kinds => kinds,
        }
    }
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
        (Shape::List(Some(a)), Shape::List(Some(b))) => {
            Ok(Shape::List(Some(Box::new(merge(*a, *b)?))))
        },
        (Shape::Array(_, a), Shape::Array(_, b)) if a != b => Err(Clash::Lengths(a, b, 0)),
        (Shape::Array(a, len), Shape::Array(b, _)) => Ok(Shape::Array(
            Box::new(merge(*a, *b).map_err(Clash::deeper)?),
            len,
        )),
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
/// in turn. An array is a slice, or a fixed array when `width` is given.
///
/// An integer is bound as `i64` and a float as `f64` unless `width` names another type,
/// which every number in the value is then bound as, and which a number that does not fit
/// is a [`Problem`] for. Anything else has no constant to bind as and is a [`Problem`]
/// naming `key`: an array of mixed kinds, a table inside an array, and a datetime. They used
/// to be bound as the text of their `Debug` or `Display` output, so a row that was meant to
/// be a record built into a string, and failed, or did not, wherever it was read.
#[cold]
pub fn bind_value(key: &str, value: &Value, width: Option<Width>) -> Result<(Ty, Lit), Problem> {
    let (shape, lit) = shaped(key, value, width)?;
    Ok((shape.into_ty(), lit))
}

/// A width is for numbers. On anything else the pattern that gave it was too broad, and that
/// is said so rather than ignored.
fn not_a_number(key: &str, kind: &str, width: Width) -> Problem {
    refuse(
        key,
        format!(
            "is {kind}, and the width `{}` applies to integers and floats only.",
            width.name()
        ),
    )
}

/// Checks that a finite float survives being narrowed to `ty`, and binds it if so.
///
/// Whether it fits is decided by narrowing it, not by comparing it with the type's largest
/// value as an `f64`: that comparison refuses everything between the limit as the message
/// prints it and its exact widening, and the limit itself is among them. A value that rounds
/// to the largest finite one is that one. One that rounds to infinity or to zero is not what
/// the file says, and stops the build like an integer that would wrap.
fn fit_float(key: &str, f: f64, ty: FloatTy) -> Result<(Shape, Lit), Problem> {
    let narrowed = match ty {
        FloatTy::F32 => f64::from(f as f32),
        FloatTy::F64 => f,
    };
    if narrowed.is_infinite() {
        return Err(refuse(
            key,
            format!(
                "is {f:e}, which does not fit {} (magnitude up to {}).",
                ty.name(),
                ty.largest_text()
            ),
        ));
    }
    if f != 0.0 && narrowed == 0.0 {
        return Err(refuse(
            key,
            format!(
                "is {f:e}, which is too small for {} (magnitude down to {}) and would become \
                 zero.",
                ty.name(),
                ty.smallest_text()
            ),
        ));
    }
    Ok((Shape::Leaf(Ty::Float(ty)), Lit::Float(f)))
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
            _ if !f.is_finite() => Err(refuse(
                key,
                format!(
                    "is {f}, which is not a finite number and has no literal to bind as. Write a \
                     finite value."
                ),
            )),
            None => Ok((Shape::Leaf(Ty::Float(FloatTy::F64)), Lit::Float(*f))),
            Some(Width::Float(ty)) => fit_float(key, *f, ty),
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
                        Clash::Lengths(a, b, depth) => refuse(
                            key,
                            format!(
                                "holds arrays of different lengths, {a} and {b} (at [{i}]{}). A \
                                 fixed array needs them all the same; drop the width to bind it \
                                 as a slice.",
                                "[0]".repeat(depth)
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
mod tests;
