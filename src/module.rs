//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::comments::extract_comments;
use crate::field::TomlFields;
use crate::pattern::Pattern;
use crate::utils;
use globset::{Glob, GlobSetBuilder};
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};
use syn::parse::{Parse, ParseStream};
use syn::{token, Result as SynResult, Token};
use toml::Value;

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
    /// Comments extracted from the TOML file, keyed by field path
    pub comments: HashMap<String, String>,
}

/// Root module that generates code from TOML data.
///
/// Combines the configuration from `RootModuleSource` with parsed TOML data
/// to generate a Rust module with constants reflecting the TOML structure.
///
#[derive(Clone, Debug)]
pub struct RootModule<'a> {
    /// Source configuration from macro input
    pub source: RootModuleSource,
    pub toml: Value,
    pub fields: TomlFields<'a>,
}

impl<'a> RootModule<'a> {
    pub fn new(mut source: RootModuleSource, toml_path: &'a str) -> Self {
        // attempt to read the TOML file from:
        // 1. direct path
        // 2. relative to workspace root
        // 3. relative to CARGO_MANIFEST_DIR
        //
        // this allows for flexibility in specifying the TOML path while
        // still providing reasonable defaults without requiring absolute paths
        // for common scenarios like referencing Cargo.toml
        let toml_raw = fs::read_to_string(toml_path).unwrap_or(
            fs::read_to_string(utils::find_workspace_root().join(toml_path)).unwrap_or(
                fs::read_to_string(
                    PathBuf::from(
                        env::var("CARGO_MANIFEST_DIR")
                            .expect("Expected CARGO_MANIFEST_DIR to be in env"),
                    )
                    .join(toml_path),
                )
                .unwrap_or_default(),
            ),
        );
        let toml: Value = toml_raw
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse toml file: {}", toml_path));
        source.comments = extract_comments(&toml_raw);
        RootModule::from(source).with_toml(toml).build()
    }

    /// Sets the parsed TOML value for this module.
    pub fn with_toml(self, toml: Value) -> Self {
        RootModule {
            toml,
            ..self
        }
    }

    /// Builds the final module by applying patterns and extracting fields.
    ///
    /// This method:
    /// 1. Converts patterns to glob matchers
    /// 2. Extracts fields matching the patterns from the TOML data
    pub fn build(self) -> Self {
        let mut inclusions = GlobSetBuilder::new();
        let mut exclusions = GlobSetBuilder::new();
        let mut literals: Vec<String> = Vec::new();
        for pattern in &self.source.inclusion_pats {
            inclusions
                .add(Glob::new(&pattern.to_string()).expect("Expected a valid glob pat string"));
            // println!("Added inclusion pattern: {}", pattern);
            literals.push(pattern.to_string());
        }
        for pattern in &self.source.exclusion_pats {
            exclusions
                .add(Glob::new(&pattern.to_string()).expect("Expected a valid glob pat string"));
            // println!("Added exclusion pattern: {}", pattern);
            literals.push(format!("!{}", pattern));
        }
        let fields = TomlFields::from(self.toml.clone())
            .with_inclusion_globs(Some(
                inclusions
                    .build()
                    .expect("Expected a succesful glob set build"),
            ))
            .with_exclusion_globs(Some(
                exclusions
                    .build()
                    .expect("Expected a succesful glob set build"),
            ))
            .with_pat_literals(literals)
            .with_comments(
                self.source
                    .comments
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )
            .with_aliases(Some(self.source.aliases.clone()));
        RootModule {
            fields: fields.build(),
            ..self
        }
    }
}

impl<'a> From<RootModuleSource> for RootModule<'a> {
    fn from(source: RootModuleSource) -> Self {
        RootModule {
            source,
            toml: Value::Table(Default::default()),
            fields: TomlFields::new(),
        }
    }
}
impl<'a> From<&'a RootModuleSource> for RootModule<'a> {
    fn from(source: &'a RootModuleSource) -> Self {
        RootModule {
            source: source.clone(),
            toml: Value::Table(Default::default()),
            fields: TomlFields::new(),
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
            comments: HashMap::new(),
        })
    }
}

impl<'a> ToTokens for RootModule<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let fields = &self.fields;
        let root_mod_name = &self.source.name;
        tokens.extend(quote! {
            pub mod #root_mod_name {
                #fields
            }
        });
    }
}
