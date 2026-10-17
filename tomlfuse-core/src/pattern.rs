//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Error;
use globset::Glob;
use std::fmt::{self, Display, Formatter};

/// A pattern over dotted toml paths, as text.
///
/// The syntax is a glob: `*` matches one segment, `**` any number of them, `{a,b}` either
/// of two, and the segments are joined by dots. The macros read patterns out of a token
/// stream and hand the text over; a tool reads them from wherever it likes.
///
/// A pattern carries no meaning of its own until a [`Section`](crate::Section) says whether
/// it includes, excludes or renames.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pattern(String);

impl Pattern {
    /// Wraps pattern text. Nothing is checked until the pattern is compiled during a bind.
    pub fn new(text: impl Into<String>) -> Self {
        Pattern(text.into())
    }

    /// The pattern as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Compiles the pattern, saying which pattern and why when it will not.
    pub(crate) fn compile(&self) -> Result<Glob, Error> {
        Glob::new(&self.0).map_err(|e| Error::BadPattern {
            pattern: self.0.clone(),
            reason: e.to_string(),
        })
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
