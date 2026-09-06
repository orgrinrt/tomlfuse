//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::field::{Patterns, TomlField, ROOT};
use crate::get_doc_comment;
use crate::pattern::Pattern;
use crate::utils::{convert_value_to_tokens, snake_to_kebab, to_valid_ident};
use globset::GlobSet;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use toml::Value;

/// The fields a binding selected out of a toml document, and the module tree they become.
///
/// Built from a parsed document and the patterns of one section: it walks the document,
/// keeps what the patterns match, works out where each field sits relative to the pattern
/// that matched it, attaches the comments read from the source text, and then generates a
/// module per table and a constant per value along those relative paths. Borrows the
/// document for as long as it lives, so a field is a reference into it rather than a copy.
#[derive(Clone, Debug)]
pub struct TomlFields<'a> {
    pub root_value: Option<&'a Value>,
    pub fields: Vec<TomlField<'a>>,
    pub patterns: Patterns,
    pub aliases: Option<HashMap<Pattern, Pattern>>,
    pub comments: Option<HashMap<String, String>>,
}
impl<'a> TomlFields<'a> {
    pub fn new() -> Self {
        TomlFields {
            root_value: None,
            fields: Vec::new(),
            patterns: Patterns::new(),
            aliases: None,
            comments: None,
        }
    }

    /// Walks the document and keeps what the patterns match, then resolves every kept
    /// field's relative path, marks the aliased ones, and attaches their comments.
    pub fn build(mut self) -> Self {
        self.extract_matched_paths_from_value(
            self.root_value
                .expect("Expected a root value when building TomlFields"),
            ROOT,
            0,
        );

        for i in 0..self.fields.len() {
            if let Some(rel_path) = self.get_relative_path(&self.fields[i].path) {
                self.fields[i].relative_path = Some(rel_path);
            }
        }

        // The alias is already baked into the name and the path during extraction; this
        // records it on the field as well, which is what a later reader of the field sees.
        for (alias, orig) in self.aliases.as_ref().unwrap_or(&HashMap::new()) {
            if let Some(field) = self.fields.iter_mut().find(|f| f.path == orig.to_string()) {
                field.alias = Some(alias.to_string());
            }
        }

        for field in &mut self.fields {
            if let Some(comment) = self.comments.as_ref().and_then(|c| {
                let mut out = c.get(&field.path);
                if out.is_none() {
                    out = c.get(&snake_to_kebab(&field.path));
                }
                if out.is_none() {
                    if let Some(ref toml_path) = field.toml_path {
                        out = c.get(toml_path);
                        if out.is_none() {
                            out = c.get(&snake_to_kebab(toml_path));
                        }
                    }
                }
                out
            }) {
                field.comment = Some(comment.to_string());
            }
        }

        self
    }
    pub fn with_root(mut self, value: &'a Value) -> Self {
        self.root_value = Some(value);
        self
    }
    pub fn with_aliases(mut self, aliases: Option<HashMap<Pattern, Pattern>>) -> Self {
        self.aliases = aliases;
        self
    }
    pub fn with_inclusion_globs(mut self, inclusion_globs: Option<GlobSet>) -> Self {
        self.patterns = self.patterns.with_inclusions(inclusion_globs);
        self
    }
    pub fn with_exclusion_globs(mut self, exclusion_globs: Option<GlobSet>) -> Self {
        self.patterns = self.patterns.with_exclusions(exclusion_globs);
        self
    }
    pub fn with_pat_literals(mut self, patterns: Vec<String>) -> Self {
        self.patterns = self.patterns.with_literals(patterns);
        self
    }
    pub fn with_comments(mut self, comments: HashMap<String, String>) -> Self {
        self.comments = Some(comments);
        self
    }

    /// Where a path sits relative to the pattern that selected it.
    ///
    /// A pattern's literal segments are the ones a binding names itself, so they are
    /// stripped and what remains is where the field lands in the section's module. Over
    /// several inclusion patterns the shortest remainder wins, which is the pattern that
    /// named the most of the path.
    // FIXME: strips every segment a pattern names anywhere rather than only a leading run of
    // them, so `a.*.a` would strip a trailing `a` too. A segment-by-segment match against the
    // pattern is what this wants, once a pattern is kept as segments rather than as text.
    fn get_relative_path(&self, path: &str) -> Option<String> {
        let mut best_match: Option<String> = None;
        for pat in &self.patterns.literals {
            if pat.starts_with('!') {
                continue;
            }
            let pat_segs = pat.split('.').filter(|s| !s.is_empty()).collect::<Vec<_>>();
            let rel_path = path
                .split('.')
                .filter(|s| !s.is_empty() && !pat_segs.contains(s))
                .collect::<Vec<_>>()
                .join(".");
            if best_match
                .as_ref()
                .is_none_or(|best| best.len() > rel_path.len())
            {
                best_match = Some(rel_path);
            }
        }
        best_match
    }

    /// The field whose module this one's constant or module is generated inside.
    ///
    /// Follows the effective module path rather than the toml hierarchy, since a pattern
    /// can flatten tables away and the two then differ.
    pub fn get_relative_parent_of_field(
        &'a self,
        this_field: &'a TomlField<'a>,
    ) -> &'a TomlField<'a> {
        let _effective_path = this_field.effective_module_path();
        let effective_path = _effective_path[.._effective_path.len().saturating_sub(1)].to_vec();
        let relative_parent_name = if !effective_path.is_empty() {
            effective_path[effective_path.len().saturating_sub(1)].to_string()
        } else {
            ROOT.to_string()
        };
        self.get_by_name(&relative_parent_name).unwrap_or_else(|| {
            panic!(
                "Expected a valid relative parent field ({} didn't exist, processing {})",
                &relative_parent_name, this_field.name
            )
        })
    }

    /// The fields generated directly inside this one's module, by the module hierarchy
    /// rather than the toml one.
    pub fn get_relative_children_of(&'a self, this_idx: usize) -> TomlFields<'a> {
        let this_field = self.get_field(this_idx).expect("Expected a valid field");
        let children = self
            .fields
            .iter()
            .filter(|field| {
                let idx = self
                    .index_of(field)
                    .expect("Expected a valid index of a field that exists");
                idx != this_idx && self.get_relative_parent_of_field(field) == this_field
            })
            .cloned()
            .collect::<Vec<TomlField>>();
        TomlFields::<'a> {
            fields: children,
            patterns: self.patterns.clone(),
            root_value: self.root_value,
            aliases: self.aliases.clone(),
            comments: self.comments.clone(),
        }
    }

    pub fn get_field(&self, idx: usize) -> Option<&TomlField<'a>> {
        self.fields.get(idx)
    }

    /// The first field with this name.
    pub fn get_by_name(&self, name: &str) -> Option<&TomlField<'a>> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn index_of(&self, field: &TomlField<'a>) -> Option<usize> {
        self.fields.iter().position(|f| f == field)
    }

    /// Walks a value and keeps every leaf the inclusion patterns match and the exclusion
    /// patterns do not, and every table on the way to one, applying aliases as it goes.
    pub fn extract_matched_paths_from_value(
        &mut self,
        value: &'a Value,
        _path: &str,
        parent_idx: usize,
    ) {
        let orig_path = _path.to_string();
        let alias = self.aliases.as_ref().and_then(|aliases| {
            let _processed_path = to_valid_ident(_path);
            aliases.iter().find_map(move |(alias, orig)| {
                // The path arrives as written in the toml and the alias as written in the
                // macro, so a kebab-case key matches its alias only after normalising.
                if _path == orig.to_string() || _processed_path == orig.to_string() {
                    Some((alias.to_string(), orig.to_string()))
                } else {
                    None
                }
            })
        });
        let alias_name = alias
            .as_ref()
            .and_then(|(alias, _)| if alias == "*" { None } else { Some(alias) })
            .map(|alias| alias.to_string());
        let is_alias = alias.is_some();
        let aliased_path = if let Some((alias, _)) = alias {
            let _psvec = _path.split('.').collect::<Vec<_>>();
            let _ps = _psvec[.._psvec.len().saturating_sub(1)].to_vec().join(".");
            format!("{}.{}", _ps, alias)
        } else {
            _path.to_string()
        };
        let path = to_valid_ident(&aliased_path);
        let (mut field, field_idx) = if parent_idx == 0 && self.fields.is_empty() {
            (TomlField::root(value), 0)
        } else {
            let field = TomlField::new(
                path.split('.')
                    .next_back()
                    .expect("Expected a valid path to extract name from"),
                &path,
                value,
                Some(parent_idx),
            )
            .with_alias(alias_name.as_deref().unwrap_or(ROOT))
            .with_toml_path(&orig_path);

            (field, self.fields.len()) // idx where this field will be placed
        };

        match value {
            Value::Table(table) => {
                self.fields.push(field);
                for (key, val) in table.iter() {
                    let new_path =
                        if path.is_empty() { key.clone() } else { format!("{}.{}", path, key) };
                    self.extract_matched_paths_from_value(val, &new_path, field_idx);
                }
            },
            _ => {
                let mut skip = !((path == ROOT)
                    || ((self.patterns.inclusions.is_none()
                        || self
                            .patterns
                            .inclusions
                            .as_ref()
                            .expect("Expected inclusion globs")
                            .is_match(&path))
                        && (self.patterns.exclusions.is_none()
                            || !self
                                .patterns
                                .exclusions
                                .as_ref()
                                .expect("Expected exclusion globs")
                                .is_match(&path))));
                if is_alias && skip {
                    field.path = field.name.clone();
                    skip = false;
                }
                if !skip {
                    self.fields.push(field);
                }
            },
        }
    }
}

impl<'a> From<&'a Value> for TomlFields<'a> {
    fn from(value: &'a Value) -> Self {
        TomlFields::new().with_root(value)
    }
}

// There is deliberately no `From<Value>`. One existed, and it boxed the value and leaked
// the box to manufacture the borrow, once per section of every invocation, for the life of
// the compiler process. `RootModule` owns the parsed document now and builds this borrowing
// it, which is what the lifetime was for.

impl<'a> ToTokens for TomlFields<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.generate_modules(tokens);
    }
}

impl<'a> TomlFields<'a> {
    /// Generates the module tree, starting from the root field.
    fn generate_modules(&self, tokens: &mut TokenStream2) {
        self.generate_module(0, tokens);
    }

    /// Generates one module: a constant per value field directly inside it and a module
    /// per table, recursively, along the effective module paths.
    fn generate_module(&self, idx: usize, tokens: &mut TokenStream2) {
        let module_name = self
            .get_field(idx)
            .expect("Expected a valid index to an existing field")
            .path
            .split('.')
            .next_back()
            .expect("Expected there to be at least one node from split by '.'");

        let mod_ident: Option<syn::Ident> = if !module_name.is_empty() {
            Some(format_ident!(
                "{}",
                to_valid_ident(module_name).to_lowercase()
            ))
        } else {
            None
        };

        let mut mod_tokens = TokenStream2::new();
        let relative_children_fields_iter = self.get_relative_children_of(idx).fields;

        for field in relative_children_fields_iter
            .iter()
            .filter(|f| !f.is_table())
        {
            let (ty, val) = convert_value_to_tokens(field.value);
            let const_name = format_ident!("{}", to_valid_ident(&field.name).to_uppercase());
            let comment = get_doc_comment(field);
            mod_tokens.extend(quote! {
                #comment
                pub const #const_name: #ty = #val;
            });
        }

        for submod in relative_children_fields_iter
            .iter()
            .filter(|f| f.is_table())
        {
            self.generate_module(
                self.index_of(submod)
                    .expect("Expected a valid child that exists and thus has an index"),
                &mut mod_tokens,
            );
        }

        if !mod_tokens.is_empty() {
            // A field with no name of its own contributes its contents to the enclosing
            // module rather than opening one. Matching on the option carries that, where
            // testing it and then unwrapping it left the two facts in different places.
            tokens.extend(match mod_ident {
                Some(name) => {
                    let comment = get_doc_comment(
                        self.get_field(idx)
                            .expect("Expected this to be a valid field"),
                    );
                    quote! {
                        #comment
                        pub mod #name {
                            #mod_tokens
                        }
                    }
                },
                None => quote! { #mod_tokens },
            });
        }
    }
}
