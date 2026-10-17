//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

//! What a binding comes out as, before any language has been chosen for it.
//!
//! The macros turn these into Rust tokens. A tool that wants the same constants in another
//! language, a shader header for one, reads the same tree and writes its own text.

/// The type a constant has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// A string.
    Str,
    /// A boolean.
    Bool,
    /// An integer.
    Int,
    /// A float.
    Float,
    /// A list of one type, of whatever length the file gave it.
    Slice(Box<Ty>),
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
