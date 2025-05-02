//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
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

    // TODO: consider if this explicit impl is necessary, is this ultimately even different from the derived one?
    fn ne(&self, other: &Self) -> bool {
        if self.segments.len() != other.segments.len() {
            return true;
        }

        for (segment, other_segment) in self.segments.iter().zip(&other.segments) {
            if segment != other_segment {
                return true;
            }
        }

        false
    }
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
/// - `Braces`, `Brackets`: Grouping constructs (future)
///
/// - Various grouping constructs like braces or brackets
#[derive(Clone, Eq, Hash, PartialEq)]
enum PatternSegment {
    Ident(Ident),
    Star,       // *
    DoubleStar, // **
    Negation,   // ! // TODO: what kind of name would this be, negation seems wrong?
    #[allow(dead_code)] // NOTE: useful api for future
    Braces(Vec<PatternSegment>),
    #[allow(dead_code)] // NOTE: useful api for future
    Brackets(Vec<PatternSegment>),
    #[allow(dead_code)] // NOTE: useful api for future
    Parens,
    // TODO: what else do we support?
}

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

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let segments = &self.segments;
        segments.to_tokens(tokens)
    }
}

impl Parse for PatternSegment {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // FIXME: handle brackets and braces as potential glob syntax segments
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

            // create new identifier from combined segments
            Ok(PatternSegment::Ident(Ident::new(&combined, span)))
        }
    }
}

impl ToTokens for PatternSegment {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            PatternSegment::Ident(ident) => ident.to_tokens(tokens),
            PatternSegment::Star => quote!(*).to_tokens(tokens),
            PatternSegment::DoubleStar => quote!(**).to_tokens(tokens),
            PatternSegment::Negation => quote!(!).to_tokens(tokens),
            PatternSegment::Braces(segments) => {
                let segments = segments.iter().map(|seg| seg.to_token_stream());
                quote!({ #(#segments)* }).to_tokens(tokens)
            },
            PatternSegment::Brackets(segments) => {
                let segments = segments.iter().map(|seg| seg.to_token_stream());
                quote!([ #(#segments)* ]).to_tokens(tokens)
            },
            _ => {
                unimplemented!()
            },
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
            PatternSegment::Braces(segments) => {
                let segments: Vec<_> = segments.iter().map(|seg| seg.to_string()).collect();
                write!(f, "{{{}}}", segments.join(", "))
            },
            PatternSegment::Brackets(segments) => {
                let segments: Vec<_> = segments.iter().map(|seg| seg.to_string()).collect();
                write!(f, "[{}]", segments.join(", "))
            },
            _ => {
                unimplemented!()
            },
        }
    }
}
