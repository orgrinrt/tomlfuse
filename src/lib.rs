//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use globset::{Glob, GlobSetBuilder};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use quote::{ToTokens, TokenStreamExt};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::path::{Path, PathBuf};
use std::{borrow::Borrow, env};
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, token, Ident, LitStr, Result as SynResult, Token};
use toml::Value;

mod utils;
mod tests;

use utils::*;

/// Generate constants from the workspace Cargo.toml file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::workspace;
///
/// workspace! {
///     [workspace]
///     workspace.*
/// }
/// ```
#[proc_macro]
pub fn workspace(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as MacroInput);

    // find workspace root
    let root_path = find_workspace_root();
    let cargo_path = root_path.join("Cargo.toml");

    generate_modules(&config, &cargo_path, true)
}

/// Generate constants from the current crate's Cargo.toml file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::package;
///
/// package! {
///     [package]
///     package.*
///     
///     [deps]
///     dependencies.*
/// }
/// ```
#[proc_macro]
pub fn package(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as MacroInput);

    // use manifest dir for crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let cargo_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    generate_modules(&config, &cargo_path, true)
}

/// Generate constants from an arbitrary TOML file.
///
/// # Example
/// ```
/// use once_cell::sync::Lazy;
/// use std::path::{Path, PathBuf};
/// use tomlfuse::file;
///
/// file!("tests/test.toml", {
///     [app]
///     app.*
///     
///     [logging]
///     logging.*
/// });
/// ```
#[proc_macro]
pub fn file(input: TokenStream) -> TokenStream {
    // parse the filepath and config
    let input_parser = syn::parse_macro_input!(input as FileInput);
    let file_path = input_parser.file_path;
    let config = input_parser.config;

    // resolve path relative to CARGO_MANIFEST_DIR
    // FIXME: we want to support absolute paths too, and relative to the toml file itself!
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let full_path = PathBuf::from(manifest_dir).join(file_path);

    generate_modules(&config, &full_path, false)
}

// shared implementation for all three macros
fn generate_modules(config: &MacroInput, toml_path: &Path, add_helpers: bool) -> TokenStream {
    // read and parse toml file
    let toml_content = fs::read_to_string(toml_path)
        .unwrap_or_else(|_| panic!("Failed to read TOML at {:?}", toml_path));

    let full_toml: Value = toml_content.parse()
        .unwrap_or_else(|e| panic!("Failed to parse TOML: {}", e));

    // extract comments
    let comments = extract_comments(&toml_content);

    // generate each section
    let mut generated_modules = Vec::new();

    for section in config.sections.iter() {
        let section_ident = format_ident!("{}", section.name);

        let (section_content, consts) = generate_section_module(
            &full_toml,
            section,
            &comments,
            &toml_content,
        );

        // generate documentation with a list of constants
        let mut const_doc = TokenStream2::new();
        if !consts.is_empty() {
            // sort the constants for consistent output
            let mut sorted_consts: Vec<_> = consts.iter().collect();
            sorted_consts.sort();

            for const_path in sorted_consts {
                let fmt = format!("- `{}`", const_path);
                const_doc.extend(quote! {
                    #[doc = concat!(#fmt)]
                });
            }
        }

        let section_name = section.name.clone();
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
        let relative_path = toml_path.strip_prefix(&manifest_dir).unwrap_or(toml_path);
        let toml_path_str = relative_path.to_string_lossy();
        generated_modules.push(quote! {
            #[doc = concat!("## `", #toml_path_str, "`")]
            #[doc = ""]
            #[doc = concat!("### `", #section_name, "`")]
            #const_doc
            #[doc = ""]
            #[doc = concat!("Generated with: [`tomlfuse`](::tomlfuse)")]
            #[doc = ""]
            #[doc = concat!("See also: [`tomlfuse::package!`](tomlfuse::package), [`tomlfuse::workspace!`](tomlfuse::workspace) and [`tomlfuse::file!`](tomlfuse::file)")]
            pub mod #section_ident {
                #section_content
            }
        });
    }

    // add helper utilities if requested
    // TODO: do we actually want to wholesale include these? I think we'd only like CARGO_MANIFEST_DIR 
    //       for package, but for workspace, we'd love both that and WORKSPACE_ROOT etc.
    //       so this needs a rethink later
    let helper_utils = if add_helpers {
        quote! {
            
                #[doc = "A statically evaluated path to the CARGO_MANIFEST_DIR"]
                pub static CARGO_MANIFEST_DIR: Lazy<PathBuf> = Lazy::new(|| {
                    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                        .expect("CARGO_MANIFEST_DIR not set");
                    PathBuf::from(manifest_dir)
                });

                #[doc = "A statically evaluated path to the workspace root "]
                pub static WORKSPACE_ROOT: Lazy<PathBuf> = Lazy::new(|| {
                    let mut path = CARGO_MANIFEST_DIR.clone();
                    while !path.join("Cargo.toml").exists() || 
                          !is_workspace_root(path.join("Cargo.toml").as_path()) {
                        if !path.pop() {
                            return CARGO_MANIFEST_DIR.clone();
                        }
                    }
                    path
                });

                #[doc = "A helper function to check if a path is the workspace root"]
                fn is_workspace_root(path: &Path) -> bool {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(parsed) = content.parse::<toml::Value>() {
                            return parsed.get("workspace").is_some();
                        }
                    }
                    false
                }
            }
    } else {
        TokenStream2::new()
    };

    // output modules directly without the fused wrapper
    quote! {
            #helper_utils
            #(#generated_modules)*
        }.into()
}

// file input parser for the file!() macro
struct FileInput {
    file_path: String,
    config: MacroInput,
}

impl Parse for FileInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // first parse the file path as a string literal
        let file_path_lit: LitStr = input.parse()?;
        let file_path = file_path_lit.value();

        // require a comma
        input.parse::<Token![,]>()?;

        // then parse the config in braces
        let content;
        syn::braced!(content in input);
        let config = content.parse()?;

        Ok(FileInput {
            file_path,
            config,
        })
    }
}

// Section configuration
struct MacroInput {
    sections: Vec<ConfigSection>,
}

struct ConfigSection {
    name: String,
    includes: Vec<PathPattern>,
    excludes: Vec<PathPattern>,
    aliases: HashMap<String, String>,
}

struct PathPattern {
    path: String,
    is_glob: bool,
}

impl ToTokens for PathPattern {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // create string literal for the path
        let path_lit = syn::LitStr::new(&self.path, proc_macro2::Span::call_site());
        let is_glob = self.is_glob;

        tokens.extend(quote! {
            PathPattern { 
                path: #path_lit.to_string(), 
                is_glob: #is_glob 
            }
        });
    }
}

// represents either an identifier or a glob symbol in a path
enum PathSegment {
    Ident(Ident),
    Star,     // *
    DoubleStar, // **
}

impl Parse for PathSegment {
    fn parse(input: ParseStream) -> SynResult<Self> {
        if input.peek(Token![*]) {
            // consume first star
            input.parse::<Token![*]>()?;

            // check for double star pattern (**)
            if input.peek(Token![*]) {
                // consume second star
                input.parse::<Token![*]>()?;
                Ok(PathSegment::DoubleStar)
            } else {
                Ok(PathSegment::Star)
            }
        } else {
            // regular identifier
            let ident = input.parse::<Ident>()?;
            Ok(PathSegment::Ident(ident))
        }
    }
}

impl ToTokens for PathSegment {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            PathSegment::Ident(ident) => ident.to_tokens(tokens),
            PathSegment::Star => quote!(* ).to_tokens(tokens),
            PathSegment::DoubleStar => quote!(** ).to_tokens(tokens),
        }
    }
}


impl Display for PathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PathSegment::Ident(ident) => write!(f, "{}", ident),
            PathSegment::Star => write!(f, "*"),
            PathSegment::DoubleStar => write!(f, "**"),
        }
    }
}

// parse dot-separated path (a.b.c.*, a.*.c, etc.)
struct DottedPath {
    segments: Punctuated<PathSegment, Token![.]>,
}

impl Parse for DottedPath {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut segments = Punctuated::new();

        // parse first segment
        if input.is_empty() {
            return Err(input.error("expected path segment"));
        }
        segments.push_value(input.parse::<PathSegment>()?);

        // parse remaining segments with dots
        while input.peek(Token![.]) {
            segments.push_punct(input.parse::<Token![.]>()?);
            segments.push_value(input.parse::<PathSegment>()?);
        }

        Ok(DottedPath { segments })
    }
}

impl DottedPath {
    // convert to a string representation
    fn _to_string(&self) -> String {
        self.segments.iter()
            .map(|seg| {
                match seg {
                    PathSegment::Ident(ident) => ident.to_string(),
                    PathSegment::Star => "*".to_string(),
                    PathSegment::DoubleStar => "**".to_string(),
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    // check if this is a glob pattern (contains * or **)
    fn is_glob(&self) -> bool {
        self.segments.iter().any(|seg|
            matches!(seg, PathSegment::Star | PathSegment::DoubleStar)
        )
    }
}

impl Display for DottedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self._to_string())
    }
}

// parse input with section syntax
impl Parse for MacroInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut sections = Vec::new();

        while !input.is_empty() {
            let section_name = {
                let content;
                syn::bracketed!(content in input);
                let ident: Ident = content.parse()?;
                ident
            };

            let mut includes = Vec::new();
            let mut excludes = Vec::new();
            let mut aliases = HashMap::new();

            // parse section content
            // look ahead for left bracket token (can't use Token![[]) directly
            while !input.is_empty() && !input.peek(token::Bracket) {
                let lookahead = input.lookahead1();

                if lookahead.peek(Token![!]) {
                    // exclusion pattern 
                    let _: Token![!] = input.parse()?;

                    // parse as dotted path with possible wildcards
                    let path =
                        if input.peek(Ident) {
                            let dotted_path: DottedPath = input.parse()?;
                            dotted_path.segments.iter()
                                .map(|id| id.to_string())
                                .collect::<Vec<_>>()
                                .join(".")
                        } else {
                            // fallback to string literal
                            let path_lit: LitStr = input.parse()?;
                            path_lit.value()
                        };

                    let is_glob = path.contains('*');
                    excludes.push(PathPattern { path, is_glob });
                } else if lookahead.peek(Ident) && input.fork().parse::<Ident>().is_ok_and(|i| i == "alias") {
                    // alias definition
                    let _: Ident = input.parse()?;

                    // src path (dotted or string)
                    let source =
                        if input.peek(LitStr) {
                            let src_lit: LitStr = input.parse()?;
                            src_lit.value()
                        } else {
                            let dotted_path: DottedPath = input.parse()?;
                            dotted_path.segments.iter()
                                .map(|id| id.to_string())
                                .collect::<Vec<_>>()
                                .join(".")
                        };

                    input.parse::<Token![=]>()?;
                    let target: Ident = input.parse()?;

                    aliases.insert(source, target.to_string());
                } else {
                    // normal path/pattern for inclusion
                    let path;
                    // TODO: we should forget support for literal strings here
                    if input.peek(LitStr) {
                        let path_lit: LitStr = input.parse()?;
                        path = path_lit.value();
                    } else if input.peek(Ident) {
                        // parse as dotted path
                        let dotted_path: DottedPath = input.parse()?;
                        path = dotted_path.segments.iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(".");
                    } else {
                        return Err(input.error("Expected identifier or string literal"));
                    }

                    let is_glob = path.contains('*');
                    includes.push(PathPattern { path, is_glob });
                }

                // optional comma separator (not sure if we should support or not?)
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            }

            sections.push(ConfigSection {
                name: section_name.to_string(),
                includes,
                excludes,
                aliases,
            });
        }

        Ok(MacroInput { sections })
    }
}

fn extract_comments(content: &str) -> HashMap<String, String> {
    let mut comments = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut current_comments = Vec::new();
    let mut current_path = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // section headers [section.subsection]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_path.clear();

            // parse [section] or [section.subsection] 
            let section_path = &trimmed[1..trimmed.len() - 1];
            current_path = section_path.split('.').map(String::from).collect();

            // reset comments at section start
            current_comments.clear();
            continue;
        }

        // comments
        if trimmed.starts_with('#') {
            current_comments.push(trimmed[1..].trim().to_string());
            continue;
        }

        // key-value pairs
        if let Some(pos) = trimmed.find('=') {
            if !trimmed.is_empty() {
                let key = trimmed[..pos].trim();

                // build full path to this key
                let mut full_path = current_path.clone();
                full_path.push(key.to_string());
                let path_str = full_path.join(".");

                // inline comment if present
                let mut key_comments = current_comments.clone();
                if let Some(comment_pos) = trimmed.find('#') {
                    if comment_pos > pos {  // comment after the = sign
                        let inline_comment = trimmed[comment_pos + 1..].trim();
                        if !inline_comment.is_empty() {
                            key_comments.push(inline_comment.to_string());
                        }
                    }
                }

                // add comments if we have any
                if !key_comments.is_empty() {
                    comments.insert(path_str, key_comments.join("\n"));
                }

                // reset comment accumulator
                current_comments.clear();
            }
        }
    }

    comments
}

fn generate_section_module(
    toml: &Value,
    section: &ConfigSection,
    comments: &HashMap<String, String>,
    raw_content: &str,
) -> (TokenStream2, HashSet<String>) {
    // build the glob matcher for includes
    let mut include_builder = GlobSetBuilder::new();
    for pattern in &section.includes {
        if pattern.is_glob {
            include_builder.add(Glob::new(&pattern.path).unwrap());
        } else {
            // exact match converted to glob
            include_builder.add(Glob::new(&format!("{}*", pattern.path)).unwrap());
        }
    }
    let include_matcher = include_builder.build().unwrap();

    // same for excludes
    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in &section.excludes {
        exclude_builder.add(Glob::new(&pattern.path).unwrap());
        // TODO: do we need similar logic as for inclusions?
    }
    let exclude_matcher = exclude_builder.build().unwrap();

    // find all paths in toml
    let mut all_paths = Vec::new();
    extract_all_paths(toml, "", &mut all_paths);

    // process matches and generate consts
    let mut processed_consts = HashSet::new();
    // store module structure based on paths
    let mut module_tree: HashMap<String, Vec<TokenStream2>> = HashMap::new();
    for (path, value) in &all_paths {
        // path reference for matching
        let path_ref: &str = path.as_str();

        // skip tables - only want leaf values
        if matches!(value, Value::Table(_)) {
            continue;
        }

        // check if included by any pattern
        if !include_matcher.is_match(path) {
            continue;
        }

        // check if excluded by any pattern
        if exclude_matcher.is_match(path) {
            continue;
        }

        // split path into parts for module hierarchy
        let split: Vec<_> = path_ref.split('.').collect();
        let parts: Vec<&str> = split.iter().copied().filter(|p| *p != split[0]).collect();

        // field name (last part of path)
        let last_part = path_ref.split('.').last().unwrap_or(path_ref);

        // check for alias
        let field_name = section.aliases.get(path_ref)
            .cloned()
            .unwrap_or_else(|| last_part.to_string());

        let is_aliased = field_name != last_part;

        // create const name from last part
        let const_name = format_ident!("{}", to_valid_ident(&field_name).to_uppercase());

        // get the module path parts (exclude the last part which is field name)
        let path_parts: Vec<&str> = parts[..parts.len().saturating_sub(1)]
            .iter()
            .filter(|&&p| !p.is_empty())
            .copied()
            .collect();

        // build the module path, properly handling the nested module structure
        let filtered_parts: Vec<&str> = {
            // find the position of section.name in the path
            let section_pos = path_parts.iter().position(|&part| part == section.name);

            if let Some(pos) = section_pos {
                // skip everything up to and including the section name
                path_parts.iter().skip(pos + 1).copied().collect()
            } else {
                // no match with section name, use all parts normally
                path_parts
            }
        };

        let module_path = filtered_parts
            .iter()
            .map(|p| to_valid_ident(p))
            .collect::<Vec<_>>()
            .join(".");

        // skip dupes
        let mut const_key = if is_aliased {
            format!("{}::{} (alias for {})", module_path, const_name, last_part)
        } else {
            format!("{}::{}", module_path, const_name)
        };
        if const_key.starts_with("::") {
            let _ = const_key.drain(0..2);
        }

        if !processed_consts.insert(const_key) {  // skip if already processed
            continue;
        }

        // extract comment if available
        let doc_comment = match comments.get(path_ref) {
            // keep comments original (preserve capitalization)
            Some(comment) => format!("Source: {}\n\n{}", path_ref, comment),
            None => format!("Source: {}", path_ref)
        };

        // determine value type and generate constant
        let (type_tokens, value_tokens) = convert_value_to_tokens(value);

        let constant = quote! {
            #[doc = #doc_comment]
            pub const #const_name: #type_tokens = #value_tokens;
        };

        // add constant to appropriate module
        module_tree.entry(module_path)
            .or_default()
            .push(constant);
    }

    // create a mapping of module prefixes to their content
    let mut module_map: HashMap<String, TokenStream2> = HashMap::new();

    // fill module map with direct content
    for (path, contents) in module_tree {
        let content_stream = quote! { #(#contents)* };
        module_map.insert(path, content_stream);
    }

    // build hierarchical modules starting from the leaves
    let paths: Vec<String> = module_map.keys().cloned().collect();
    let mut processed_modules = HashSet::new();
    // let module_map_clone = module_map.clone();

    // process all path depths, deepest first
    for depth in (1..=20).rev() { // reasonable max depth
        for path in &paths {
            let parts: Vec<&str> = path.split('.').collect();
            if parts.len() != depth || processed_modules.contains(path) {
                continue;
            }

            // special case for empty path
            if path.is_empty() {
                // module_map.entry("".to_string()).or_default().extend(module_map_clone.get(path).cloned().unwrap_or_default());
                continue;
            }

            processed_modules.insert(path.clone());

            // get content for this module
            let content = module_map.remove(path).unwrap_or_else(TokenStream2::new);

            // skip creating modules with empty names
            if parts.last().unwrap().is_empty() {
                continue;
            }

            // create module
            let module_name = format_ident!("{}", to_valid_ident(parts.last().unwrap()));
            let module_def = quote! {
                pub mod #module_name {
                    use super::*;
                    #content
                }
            };

            // determine parent path, handling empty segments
            let parent_path = if parts.len() > 1 {
                parts[..parts.len() - 1]
                    .iter()
                    .filter(|&p| !p.is_empty())
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            } else { "".to_string() };

            module_map.entry(parent_path).or_default().extend(module_def);
        }
    }
    // generate the final module content
    let mut module_content = TokenStream2::new();
    for (path, content) in module_map {
        // skip empty modules
        if content.is_empty() {
            continue;
        }

        module_content.extend(content);
    }
    (module_content, processed_consts)
}


fn extract_all_paths<'a>(
    value: &'a Value,
    prefix: &str,
    results: &mut Vec<(String, &'a Value)>,
) {
    match value {
        Value::Table(table) => {
            // add table itself
            if !prefix.is_empty() {
                results.push((prefix.to_string(), value));
            }

            // process all keys in the table
            for (key, val) in table.iter() {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                // recurse into this value
                extract_all_paths(val, &new_prefix, results);
            }
        }
        _ => {
            // add leaf value
            if !prefix.is_empty() {
                results.push((prefix.to_string(), value));
            }
        }
    }
}
