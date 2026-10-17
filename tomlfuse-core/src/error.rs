//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::fmt::{self, Display, Formatter};
use std::io;
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
        /// What the system said when a place was there and could not be read, such as a
        /// permission being refused. A place that was simply absent says nothing of the
        /// kind, since the list above already does.
        cause: Option<io::Error>,
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
    /// A width was given to a pattern that has no number in the file to apply it to.
    UnusedWidth {
        /// The pattern as written.
        pattern: String,
        /// The width it was given, as Rust spells it.
        width: &'static str,
        /// Whether the pattern matches tables, so that it is not a typo but a pattern too
        /// shallow to reach a number. Otherwise it matches nothing at all.
        only_tables: bool,
    },
    /// The environment a cargo-relative lookup needs is not there.
    NotUnderCargo {
        /// The variable that was missing.
        variable: &'static str,
    },
}

/// A value in the file that cannot be bound as a constant.
///
/// This is not an [`Error`]: the rest of the section binds, and the caller decides what a
/// value that does not is worth. The macros turn each into a `compile_error!` at the
/// invocation, so the message arrives at the file and not wherever the constant is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The key as it is written in the file, with the index of the element where one is at
    /// fault: `row.records[1]`.
    pub key: String,
    /// What is wrong with it, as a sentence that follows the key.
    pub reason: String,
}

impl Display for Problem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "tomlfuse: `{}` {}", self.key, self.reason)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreadable {
                given,
                tried,
                cause,
            } => {
                write!(f, "tomlfuse: could not read `{given}`")?;
                if let Some(cause) = cause {
                    write!(f, ": {cause}")?;
                }
                write!(
                    f,
                    ". Looked in:\n  {}\nA relative path is taken relative to the crate, then to \
                     the workspace root.",
                    tried
                        .iter()
                        .map(|c| c.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n  ")
                )
            },
            // The parser's own message, whole. It carries the line, the column and the line of
            // the file it stopped at, which is the only thing that finds a syntax error in a
            // large file without reading it by eye. Its message ends in a newline of its own.
            Error::Unparseable {
                given,
                source,
            } => write!(
                f,
                "tomlfuse: could not parse `{given}`:\n{}",
                source.to_string().trim_end()
            ),
            Error::BadPattern {
                pattern,
                reason,
            } => write!(
                f,
                "tomlfuse: `{pattern}` is not a valid pattern. {reason}\n\
                 Patterns are globs over dotted toml paths: `*` matches one segment, `**` any \
                 number, `{{a,b}}` either, and a leading `!` excludes."
            ),
            Error::UnusedWidth {
                pattern,
                width,
                only_tables: false,
            } => write!(
                f,
                "tomlfuse: the width `{width}` is given to `{pattern}`, which matches no key in \
                 the file, so it would apply to nothing. Check the pattern for a typo."
            ),
            Error::UnusedWidth {
                pattern,
                width,
                only_tables: true,
            } => write!(
                f,
                "tomlfuse: the width `{width}` is given to `{pattern}`, which matches only \
                 tables. A width is for the numbers inside one, so name those: `{pattern}.*`, or \
                 a key."
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
            Error::Unreadable {
                cause,
                ..
            } => cause.as_ref().map(|cause| cause as _),
            _ => None,
        }
    }
}
