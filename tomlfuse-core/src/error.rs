//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

/// Why a toml file could not be turned into constants.
///
/// Each variant renders as the message a reader of a build log wants, so a caller that only
/// wants to stop and say why can print it as it is.
#[derive(Debug)]
pub enum Error {
    /// No place the file could be in held a readable file.
    Unreadable {
        /// The path as it was given.
        given: String,
        /// Every place that was tried, in order.
        tried: Vec<PathBuf>,
    },
    /// The file was found and is not toml.
    Unparseable {
        /// The path as it was given.
        given: String,
        /// The parser's own error.
        source: toml::de::Error,
    },
    /// A pattern is not a glob over dotted toml paths.
    BadPattern {
        /// The pattern as written.
        pattern: String,
        /// What the glob compiler objected to.
        reason: String,
    },
    /// The environment a cargo-relative lookup needs is not there.
    NotUnderCargo {
        /// The variable that was missing.
        variable: &'static str,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreadable {
                given,
                tried,
            } => write!(
                f,
                "tomlfuse: could not read `{given}`. Looked in:\n  {}\nA relative path is taken \
                 relative to the crate, then to the workspace root.",
                tried
                    .iter()
                    .map(|c| c.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
            Error::Unparseable {
                given,
                ..
            } => write!(f, "Failed to parse toml file: {given}"),
            Error::BadPattern {
                pattern,
                reason,
            } => write!(
                f,
                "tomlfuse: `{pattern}` is not a valid pattern. {reason}\n\
                 Patterns are globs over dotted toml paths: `*` matches one segment, `**` any \
                 number, `{{a,b}}` either, and a leading `!` excludes."
            ),
            Error::NotUnderCargo {
                variable,
            } => write!(
                f,
                "tomlfuse: `{variable}` is not set, so a path cannot be taken relative to the \
                 crate. Run under cargo, or read the file by its full path."
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Unparseable {
                source,
                ..
            } => Some(source),
            _ => None,
        }
    }
}
