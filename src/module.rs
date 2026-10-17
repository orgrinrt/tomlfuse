//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::pattern::Pattern;
use proc_macro2::Ident;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{token, Result as SynResult, Token};
use tomlfuse_core::Section;

mod kw {
    syn::custom_keyword!(alias);
}

/// Source configuration for a root module used in macro input.
///
/// Represents the parsed pattern declarations from macro input that define
/// which TOML entries should be included in the generated code.
///
#[derive(Clone, Debug)]
pub struct RootModuleSource {
    /// Name of the module to be generated
    pub name: Ident,
    /// Patterns for fields to include in the generated code
    pub inclusion_pats: Vec<Pattern>,
    /// Patterns for fields to exclude from the generated code
    pub exclusion_pats: Vec<Pattern>,
    /// Map of pattern aliases where key is the alias and value is the original pattern
    pub aliases: HashMap<Pattern, Pattern>,
}

impl RootModuleSource {
    /// The same selection, in the terms `tomlfuse-core` binds by.
    pub fn section(&self) -> Section {
        let core = |p: &Pattern| tomlfuse_core::Pattern::new(p.to_string());
        Section {
            name: self.name.to_string(),
            include: self.inclusion_pats.iter().map(core).collect(),
            exclude: self.exclusion_pats.iter().map(core).collect(),
            aliases: self.aliases.iter().map(|(alias, orig)| (core(alias), core(orig))).collect(),
        }
    }
}

impl Parse for RootModuleSource {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let bracket_stream;
        let _bracket = syn::bracketed!(bracket_stream in input);
        let root_mod_name: Ident = bracket_stream.parse()?;
        let mut inclusion_pats = Vec::new();
        let mut exclusion_pats = Vec::new();
        let mut aliases: HashMap<Pattern, Pattern> = HashMap::new();

        while !input.peek(token::Bracket) && !input.is_empty() {
            if input.peek(kw::alias) {
                let _kw: kw::alias = input.parse()?;
                let alias: Pattern = input.parse()?;
                let _eq: token::Eq = input.parse()?;
                let path: Pattern = input.parse()?;
                aliases.insert(alias, path);
            } else if input.peek(Token![!]) {
                let _negation: Token![!] = input.parse()?;
                let pattern = Pattern::parse(input)?;
                exclusion_pats.push(pattern)
            } else {
                let pattern = Pattern::parse(input)?;
                inclusion_pats.push(pattern);
            }
        }
        Ok(RootModuleSource {
            name: root_mod_name,
            inclusion_pats,
            exclusion_pats,
            aliases,
        })
    }
}
