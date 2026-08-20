//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use proc_macro2::Ident;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Result as SynResult, Token};

/// Represents a pattern for matching TOML paths.
///
/// Patterns use dot-separated segments with special syntax:
/// - Regular identifiers match exact paths
/// - `*` matches any single segment
/// - `**` matches any number of segments (recursive)
/// - `!` at start negates the pattern (for exclusion)
/// - Braces and brackets for grouping (future)
///
///
/// For example: `section.*` matches all direct children of "section".
pub struct Pattern {
    segments: Punctuated<PatternSegment, Token![.]>,
    spans: Vec<proc_macro2::Span>,
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        if self.segments.len() != other.segments.len() {
            return false;
        }

        for (segment, other_segment) in self.segments.iter().zip(&other.segments) {
            if segment != other_segment {
                return false;
            }
        }

        true
    }

    // The TODO here asked whether the hand-written `ne` was necessary or any different
    // from the default. It was neither: `ne` defaults to `!eq`, and this was that, spelled
    // out. Worse, a hand-written `ne` is a place where the two can drift apart, and two
    // values that are both equal and unequal break every container that holds them.
}

impl Eq for Pattern {}

impl Hash for Pattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for segment in &self.segments {
            segment.hash(state);
        }
    }
}

impl Clone for Pattern {
    fn clone(&self) -> Self {
        Pattern {
            segments: self.segments.clone(),
            spans: if self.spans.is_empty() {
                self.segments.iter().map(|seg| seg.span()).collect()
            } else {
                self.spans.clone()
            },
        }
    }
}

impl Debug for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Represents a single segment in a pattern.
///
/// Segment types:
/// - `Ident`: Normal identifiers for exact matching (e.g., "section", "key")
/// - `Star`: Single wildcard (`*`) matching any one segment
/// - `DoubleStar`: Recursive wildcard (`**`) matching any number of segments
/// - `Negation`: Exclusion prefix (`!`) for pattern negation
/// - `Braces`: Alternation (`{a,b}`), matching a segment that is any one alternative
#[derive(Clone, Eq, Hash, PartialEq)]
enum PatternSegment {
    Ident(Ident),
    Star,       // *
    DoubleStar, // **
    Negation,   // ! // TODO: what kind of name would this be, negation seems wrong?
    /// Alternation, `{a,b}`. Matches a segment that is any one of the alternatives.
    Braces(Vec<PatternSegment>),
}

// Glob character classes, `[a-z]`, are deliberately absent, and the reason is the syntax
// rather than the effort. A module header in this macro is `[name]`, and the input is a
// Rust token stream, which has no newlines in it. So `config.debug` on one line followed
// by `[classes]` on the next is the same tokens as `config.debug[classes]`, and a parser
// that reads a bracket after an identifier as a character class swallows the next module
// header instead. That was written, and it was the combined test that caught it: each
// pattern passed alone and the module after one went missing.
//
// The delimiter is spoken for. Alternation, `{a,b}`, has no such clash and is supported.

impl Parse for Pattern {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut segments = Punctuated::new();
        let mut spans = Vec::new();

        if input.is_empty() {
            return Err(input.error("Expected a valid pattern segment"));
        }

        spans.push(input.span());
        segments.push_value(input.parse::<PatternSegment>()?);

        while input.peek(Token![.]) {
            segments.push_punct(input.parse::<Token![.]>()?);
            spans.push(input.span());
            segments.push_value(input.parse::<PatternSegment>()?);
        }

        Ok(Pattern {
            segments,
            spans,
        })
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = self
            .segments
            .iter()
            .map(|seg| seg.to_string())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", s)
    }
}

impl Parse for PatternSegment {
    fn parse(input: ParseStream) -> SynResult<Self> {
        if input.peek(syn::token::Brace) {
            // `{debug,release}` arrives as a brace group holding comma-separated segments.
            // globset reads the same syntax, so this only has to survive the round trip
            // through Rust's tokeniser.
            let content;
            syn::braced!(content in input);
            let alternatives =
                Punctuated::<PatternSegment, Token![,]>::parse_terminated(&content)?;
            if alternatives.is_empty() {
                return Err(content.error("an alternation needs at least one alternative"));
            }
            return Ok(PatternSegment::Braces(alternatives.into_iter().collect()));
        }

        if input.peek(Token![*]) {
            // consume first star
            input.parse::<Token![*]>()?;

            // check for double star pattern (**)
            if input.peek(Token![*]) {
                // consume second star
                input.parse::<Token![*]>()?;
                Ok(PatternSegment::DoubleStar)
            } else {
                Ok(PatternSegment::Star)
            }
        } else if input.peek(Token![!]) {
            // consume negation
            input.parse::<Token![!]>()?;
            Ok(PatternSegment::Negation)
        } else {
            // parse first identifier
            let ident = input.parse::<Ident>()?;
            let span = ident.span();
            let mut combined = ident.to_string();

            // keep looking for dash + ident combinations
            while input.peek(Token![-]) {
                // consume dash
                input.parse::<Token![-]>()?;

                // parse the following identifier
                let next_ident = input.parse::<Ident>()?;

                // combine identifiers with underscore
                combined.push('_');
                combined.push_str(&next_ident.to_string());
            }

            Ok(PatternSegment::Ident(Ident::new(&combined, span)))
        }
    }
}

impl PatternSegment {
    /// Where this segment was written.
    ///
    /// Only an identifier carries one of its own; the rest are punctuation or text and
    /// report the call site. This used to arrive through `syn`'s blanket `Spanned` impl,
    /// which needs `ToTokens`, and the `ToTokens` impl existed for no other reason.
    fn span(&self) -> proc_macro2::Span {
        match self {
            PatternSegment::Ident(ident) => ident.span(),
            PatternSegment::Star
            | PatternSegment::DoubleStar
            | PatternSegment::Negation
            | PatternSegment::Braces(_) => proc_macro2::Span::call_site(),
        }
    }
}

impl Display for PatternSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternSegment::Ident(ident) => write!(f, "{}", ident),
            PatternSegment::Star => write!(f, "*"),
            PatternSegment::DoubleStar => write!(f, "**"),
            PatternSegment::Negation => write!(f, "!"),
            // No spaces in either. This string is handed to globset, where a space is a
            // character to be matched rather than punctuation to be ignored, so the
            // `", "` these used to join on would have made `{a, b}` match a segment
            // beginning with a space.
            PatternSegment::Braces(segments) => {
                let segments: Vec<_> = segments.iter().map(ToString::to_string).collect();
                write!(f, "{{{}}}", segments.join(","))
            },
        }
    }
}
