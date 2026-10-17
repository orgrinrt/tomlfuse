//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! What a binding comes out as, before any language has been chosen for it.
//!
//! The macros turn these into Rust tokens. A tool that wants the same constants in another
//! language, a shader header for one, reads the same tree and writes its own text.

/// An integer type a constant can be bound as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntTy {
    /// `i8`
    I8,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `i64`, which is what an integer is bound as when no width is named.
    I64,
    /// `u8`
    U8,
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `u64`
    U64,
}

impl IntTy {
    /// The type's name as Rust spells it.
    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
        }
    }

    /// The smallest and the largest value the type holds.
    pub fn bounds(self) -> (i128, i128) {
        match self {
            IntTy::I8 => (i8::MIN.into(), i8::MAX.into()),
            IntTy::I16 => (i16::MIN.into(), i16::MAX.into()),
            IntTy::I32 => (i32::MIN.into(), i32::MAX.into()),
            IntTy::I64 => (i64::MIN.into(), i64::MAX.into()),
            IntTy::U8 => (u8::MIN.into(), u8::MAX.into()),
            IntTy::U16 => (u16::MIN.into(), u16::MAX.into()),
            IntTy::U32 => (u32::MIN.into(), u32::MAX.into()),
            IntTy::U64 => (u64::MIN.into(), u64::MAX.into()),
        }
    }
}

/// A float type a constant can be bound as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatTy {
    /// `f32`
    F32,
    /// `f64`, which is what a float is bound as when no width is named.
    F64,
}

impl FloatTy {
    /// The type's name as Rust spells it.
    pub fn name(self) -> &'static str {
        match self {
            FloatTy::F32 => "f32",
            FloatTy::F64 => "f64",
        }
    }

    /// The largest magnitude of a finite value the type holds.
    pub fn largest(self) -> f64 {
        match self {
            FloatTy::F32 => f32::MAX.into(),
            FloatTy::F64 => f64::MAX,
        }
    }

    /// The same, as the shortest text that reads back as the type's own value. Converting
    /// the `f64` above and printing it would show the digits of the widening instead.
    pub fn largest_text(self) -> String {
        match self {
            FloatTy::F32 => format!("{:e}", f32::MAX),
            FloatTy::F64 => format!("{:e}", f64::MAX),
        }
    }
}

/// The type a key is to be bound at, when the macro input names one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Width {
    /// An integer type.
    Int(IntTy),
    /// A float type.
    Float(FloatTy),
}

impl Width {
    /// The names [`Width::parse`] reads, in the order an error lists them.
    pub const NAMES: [&'static str; 10] =
        ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64"];

    /// Reads a type name, or nothing when it is not one of [`Width::NAMES`].
    pub fn parse(name: &str) -> Option<Width> {
        Some(match name {
            "i8" => Width::Int(IntTy::I8),
            "i16" => Width::Int(IntTy::I16),
            "i32" => Width::Int(IntTy::I32),
            "i64" => Width::Int(IntTy::I64),
            "u8" => Width::Int(IntTy::U8),
            "u16" => Width::Int(IntTy::U16),
            "u32" => Width::Int(IntTy::U32),
            "u64" => Width::Int(IntTy::U64),
            "f32" => Width::Float(FloatTy::F32),
            "f64" => Width::Float(FloatTy::F64),
            _ => return None,
        })
    }

    /// The type's name as Rust spells it.
    pub fn name(self) -> &'static str {
        match self {
            Width::Int(ty) => ty.name(),
            Width::Float(ty) => ty.name(),
        }
    }

    /// The constant type a number of this width is.
    pub fn ty(self) -> Ty {
        match self {
            Width::Int(ty) => Ty::Int(ty),
            Width::Float(ty) => Ty::Float(ty),
        }
    }
}

/// The type a constant has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// A string.
    Str,
    /// A boolean.
    Bool,
    /// An integer.
    Int(IntTy),
    /// A float.
    Float(FloatTy),
    /// A list of one type, of whatever length the file gave it.
    Slice(Box<Ty>),
    /// A list of one type and a length the file gave it, which is what an array becomes when
    /// its numbers are given a width.
    Array(Box<Ty>, usize),
}

/// The value a constant has. Which of its forms it takes follows from its [`Ty`].
#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    /// A string.
    Str(String),
    /// A boolean.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A float.
    Float(f64),
    /// The elements of a list, in order.
    List(Vec<Lit>),
}

/// One bound value.
#[derive(Clone, Debug, PartialEq)]
pub struct Const {
    /// The constant's name, already an upper-case identifier.
    pub name: String,
    /// The comment above the key in the file, if it had one.
    pub comment: Option<String>,
    /// Its type.
    pub ty: Ty,
    /// Its value.
    pub value: Lit,
}

/// A group of constants and the groups inside it, which is to say a table of the file as
/// the patterns left it.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    /// The module's name, already a lower-case identifier. The root of a section has the
    /// section's name, and a module that contributes its contents to the one around it has
    /// none.
    pub name: Option<String>,
    /// The comment above the table in the file, if it had one.
    pub comment: Option<String>,
    /// The constants directly in this module.
    pub consts: Vec<Const>,
    /// The modules directly in this module. One with nothing in it is not listed.
    pub modules: Vec<Module>,
}

impl Module {
    /// Whether there is nothing in this module, nor in anything inside it.
    pub fn is_empty(&self) -> bool {
        self.consts.is_empty() && self.modules.is_empty()
    }
}
