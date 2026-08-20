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


/// Finds the toml file and reads it, returning where it was found alongside its contents.
///
/// Three places are tried, in this order: the path as given, then relative to the
/// workspace root, then relative to `CARGO_MANIFEST_DIR`. Which one succeeded matters,
/// because the generated code registers it with cargo so that editing the file rebuilds
/// what was generated from it.
///
/// The candidates used to be the three arms of nested `unwrap_or` calls, which evaluate
/// their argument whether or not it is needed, so every module read the file from all
/// three places every time. `or_else` reads one.
///
/// A path matching none of them used to become `unwrap_or_default()`, an empty string,
/// which parses as an empty toml and generates an empty module. A mistyped path was
/// therefore silent.
fn resolve_toml(toml_path: &str) -> (PathBuf, String) {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Expected CARGO_MANIFEST_DIR to be in env"),
    );
    let candidates = [
        PathBuf::from(toml_path),
        utils::find_workspace_root().join(toml_path),
        manifest_dir.join(toml_path),
    ];

    for candidate in &candidates {
        if let Ok(contents) = fs::read_to_string(candidate) {
            // Absolute, because the generated `include_bytes!` resolves relative to the
            // file the macro was invoked in, not to the crate root. A relative path
            // therefore doubled up: `tests/test.toml` became `tests/tests/test.toml`.
            let absolute = fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
            return (absolute, contents);
        }
    }

    panic!(
        "tomlfuse: could not read `{toml_path}`. Looked in:\n  {}\nThe path is taken as \
         given, then relative to the workspace root, then relative to the crate root.",
        candidates
            .iter()
            .map(|c| c.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    )
}

/// Turns one pattern into a glob, saying which pattern and why when it will not.
fn compile_glob(pattern: &crate::pattern::Pattern) -> Glob {
    let text = pattern.to_string();
    Glob::new(&text).unwrap_or_else(|e| {
        panic!(
            "tomlfuse: `{text}` is not a valid pattern. {e}\n\
             Patterns are globs over dotted toml paths: `*` matches one segment, `**` any \
             number, `{{a,b}}` either, `[a-z]` one character, and a leading `!` excludes."
        )
    })
}
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
    /// Where the toml was actually found, once it has been read.
    ///
    /// Kept so the generated code can register the file with cargo. Without that, editing
    /// the toml changes nothing until something else forces the crate to rebuild, and the
    /// constants a build produces are silently the previous file's.
    pub resolved_path: Option<PathBuf>,
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
        let (resolved, toml_raw) = resolve_toml(toml_path);
        let toml: Value = toml_raw
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse toml file: {}", toml_path));
        source.comments = extract_comments(&toml_raw);
        source.resolved_path = Some(resolved);
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
        // A proc-macro panic reaches the user with its message attached, so the message is
        // the whole diagnostic. "Expected a valid glob pat string" named neither the
        // pattern nor what globset objected to, leaving nothing to act on.
        for pattern in &self.source.inclusion_pats {
            inclusions.add(compile_glob(pattern));
            literals.push(pattern.to_string());
        }
        for pattern in &self.source.exclusion_pats {
            exclusions.add(compile_glob(pattern));
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
            // Filled in when the file is read, which happens after parsing.
            resolved_path: None,
        })
    }
}

impl<'a> ToTokens for RootModule<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let fields = &self.fields;
        let root_mod_name = &self.source.name;

        // Register the toml with cargo, so that editing it rebuilds what was generated
        // from it. A procedural macro reads files behind cargo's back: nothing in a macro
        // invocation tells it that the output depends on anything but the source file the
        // invocation sits in. Without this, editing the toml and rebuilding leaves the old
        // constants in place, and the build reports success.
        //
        // `include_bytes!` is what does it. `proc_macro::tracked_path` is the direct way
        // and is unstable. The bytes are bound to an anonymous constant, so this costs a
        // name nobody can collide with and nothing at runtime.
        let tracker = self.source.resolved_path.as_ref().map(|path| {
            let path = path.to_string_lossy();
            let path = syn::LitStr::new(&path, proc_macro2::Span::call_site());
            quote! {
                const _: &[u8] = include_bytes!(#path);
            }
        });

        tokens.extend(quote! {
            #tracker
            pub mod #root_mod_name {
                #fields
            }
        });
    }
}
