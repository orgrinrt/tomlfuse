//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Problem;
use crate::model::{Const, Module, Width};
use crate::names::{join_key, snake_to_kebab, to_valid_ident};
use crate::pattern::Pattern;
use crate::value::bind_value;
use globset::{GlobMatcher, GlobSet};
use std::collections::HashMap;
use std::string::ToString;
use std::sync::LazyLock;
use toml::Value;

pub const ROOT: &str = "";

/// Represents a single field extracted from a TOML document.
///
/// Maps a TOML node to its path, value and metadata required for code generation.
/// Fields can represent either leaf values (constants) or tables (modules).
/// Tracks both path and relationship information for hierarchical organization.
///
#[derive(Debug, Clone, PartialEq)]
pub struct TomlField<'a> {
    /// Name of the field, used as identifier in generated code
    pub name: String,
    /// Reference to the actual TOML value
    pub value: &'a Value,
    /// Full dot-separated path to this field in normalized form
    pub path: String,
    /// Path relative to the matching pattern, for hierarchical organization
    pub relative_path: Option<String>, // path relative to matched pattern
    /// Original path in the TOML file before normalization (preserves case)
    pub toml_path: Option<String>, // path in the actual toml file
    /// The key as it is written in the file, quoted where a key needs it. Unlike the paths
    /// above, nothing in it has been through identifier normalization, so it is the one an
    /// error can name and a reader can search the file for.
    pub key: String,
    /// Optional alias for the field, if specified in the macro
    pub alias: Option<String>,
    /// Index of parent field in the fields collection
    pub parent: Option<usize>,
    /// Comment associated with this field from the TOML file
    pub comment: Option<String>,
}

impl Default for TomlField<'_> {
    fn default() -> Self {
        static DEFAULT_VALUE: LazyLock<Value> = LazyLock::new(|| Value::from(""));

        TomlField {
            name: String::default(),
            value: &DEFAULT_VALUE,
            path: String::default(),
            relative_path: None,
            toml_path: None,
            key: String::default(),
            alias: None,
            parent: None,
            comment: None,
        }
    }
}

impl<'a> From<&'a Value> for TomlField<'a> {
    fn from(value: &'a Value) -> Self {
        Self::default().with_value(value)
    }
}

impl<'a> TomlField<'a> {
    pub fn new(name: &str, path: &str, value: &'a Value, parent: Option<usize>) -> Self {
        TomlField {
            name: name.to_string(),
            value,
            path: path.to_string(),
            relative_path: None,
            toml_path: None,
            key: String::default(),
            alias: None,
            parent,
            comment: None,
        }
    }

    pub fn root(value: &'a Value) -> Self {
        TomlField {
            name: ROOT.to_string(),
            value,
            path: ROOT.to_string(),
            relative_path: None,
            toml_path: None,
            key: String::default(),
            alias: None,
            parent: None,
            comment: None,
        }
    }
    // FIXME: unify construction to use builder pattern instead of whatever we do above and in From impls
    #[allow(dead_code)] // NOTE: might be useful later
    pub fn with_relative_path(mut self, relative_path: &str) -> Self {
        self.relative_path = Some(relative_path.to_string());
        self
    }
    pub fn with_toml_path(mut self, toml_path: &str) -> Self {
        self.toml_path = Some(toml_path.to_string());
        self
    }
    #[allow(dead_code)] // NOTE: might be useful later
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
    pub fn with_key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
    pub fn with_alias(mut self, alias: &str) -> Self {
        if alias == ROOT {
            return self;
        }
        self.alias = Some(alias.to_string());
        self
    }
    #[allow(dead_code)] // NOTE: might be useful later
    pub fn with_path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }
    #[allow(dead_code)] // NOTE: might be useful later
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    pub fn with_value(mut self, value: &'a Value) -> Self {
        self.value = value;
        self
    }
    #[allow(dead_code)] // NOTE: might be useful later
    pub fn with_parent(mut self, parent: usize) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Determines if this field represents a TOML table.
    ///
    /// # Returns
    /// `true` if the field's value is a TOML table, `false` otherwise.
    ///
    /// This helps guide the module generation process during code generation.
    pub fn is_table(&self) -> bool {
        matches!(self.value, Value::Table(_))
    }

    // get effective module path based on section/pattern matching
    pub fn effective_module_path(&self) -> Vec<String> {
        // println!(" >> Resolving effective module path for: {}", self.path);
        let output =
        // use relative path if available (pattern matching)
        if let Some(ref rel_path) = self.relative_path {
            rel_path
                .split('.')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        } else {
            // if no relative path, use full path
            self
                .path
                .split('.')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        };
        // println!("    >> Effective module path: {:?}", output);
        output
    }
}

/// Container for pattern matchers used to filter TOML fields.
///
/// Filtering logic:
/// - Inclusion patterns specify which fields to include
/// - Exclusion patterns override inclusions for specific fields
///
/// The literals are included for improved heuristics downstream.
///
#[derive(Clone, Debug, Default)]
pub struct Patterns {
    pub inclusions: Option<GlobSet>,
    pub exclusions: Option<GlobSet>,
    pub literals: Vec<String>,
}
impl Patterns {
    pub fn new() -> Self {
        Patterns::default()
    }
    pub fn with_inclusions(mut self, inclusions: Option<GlobSet>) -> Self {
        self.inclusions = inclusions;
        self
    }
    pub fn with_exclusions(mut self, exclusions: Option<GlobSet>) -> Self {
        self.exclusions = exclusions;
        self
    }
    pub fn with_literals(mut self, literals: Vec<String>) -> Self {
        self.literals = literals;
        self
    }
    // pub fn add_literal(&mut self, literal: String) {
    //     if self.literals.is_empty() {
    //         self.literals = Vec::new();
    //     }
    //     self.literals.push(literal);
    // }
}

/// A width, and the pattern that says which keys it is for.
#[derive(Clone, Debug)]
pub struct WidthRule {
    /// The pattern as written, for saying which one an error is about.
    pub pattern: Pattern,
    /// The compiled pattern.
    pub matcher: GlobMatcher,
    /// The type to bind the matching keys at.
    pub width: Width,
}

/// Collection of TOML fields with pattern matching capabilities.
///
/// Central structure responsible for:
/// 1. Extracting fields from TOML data matching specified patterns
/// 2. Associating comments with their respective fields
/// 3. Resolving both absolute and relative paths
/// 4. Managing parent-child relationships between fields
/// 5. Generating the Rust module structure
///
/// Fields can be accessed by index, name, or parent-child relationships.
#[derive(Clone, Debug, Default)]
// TODO: should probably rethink the name to better signal what it is. `ModuleImpl` maybe?
pub struct TomlFields<'a> {
    pub root_value: Option<&'a Value>,
    pub fields: Vec<TomlField<'a>>,
    pub patterns: Patterns,
    pub aliases: Option<HashMap<Pattern, Pattern>>,
    pub comments: Option<HashMap<String, String>>,
    pub widths: Vec<WidthRule>,
}
impl<'a> TomlFields<'a> {
    pub fn new() -> Self {
        TomlFields::default()
    }

    /// Builds the fields collection by extracting matched paths from the TOML document.
    ///
    /// Processing steps:
    /// 1. Extracts all fields matching configured patterns and pre-processes them
    /// 2. Resolves relative paths for hierarchical organization
    /// 3. Associates comments with the corresponding fields
    /// 4. Applies aliases to fields where specified
    pub fn build(mut self) -> Self {
        // println!("Building TomlFields...");
        self.extract_matched_paths_from_value(
            self.root_value
                .expect("Expected a root value when building TomlFields"),
            ROOT,
            0,
            ROOT,
        );

        for i in 0..self.fields.len() {
            if let Some(rel_path) = self.get_relative_path(&self.fields[i].path) {
                self.fields[i].relative_path = Some(rel_path);
            }
        }

        // TODO: this is redundant, since we already bake the aliases into the name and the path in the extract method,
        //       we should include the alias there I think
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
                // println!(" >> Found comment for field {}: {}", field.path, comment);
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
    // pub fn with_pat_literal(mut self, pattern: String) -> Self {
    //     self.patterns.add_literal(pattern);
    //     self
    // }
    pub fn with_widths(mut self, widths: Vec<WidthRule>) -> Self {
        self.widths = widths;
        self
    }
    pub fn with_comments(mut self, comments: HashMap<String, String>) -> Self {
        self.comments = Some(comments);
        self
    }

    // find the section a path belongs to and the relative path within that section
    fn get_relative_path(&self, path: &str) -> Option<String> {
        let mut best_match: Option<String> = None;
        for pat in &self.patterns.literals {
            if pat.starts_with('!') {
                continue;
            }
            let pat_segs = pat
                .split('.')
                .filter(
                    |s| !s.is_empty() || s == &"*" || s == &"**", // etc, we should probably do a better job of this
                )
                .collect::<Vec<_>>();
            let path_segs = path
                .split('.')
                .filter(|s| {
                    // println!("         >> Path seg: {} (is in pattern: {})", s, pat_segs.contains(s));
                    !s.is_empty() && !pat_segs.contains(s)
                })
                .collect::<Vec<_>>();
            let rel_path = path_segs
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(".");
            // println!("    >> Found match for path {}: {}", path, rel_path);
            if best_match.is_none() || best_match.as_ref().unwrap().len() > rel_path.len() {
                // FIXME: needs a bit better way to determine the best match ":D"
                best_match = Some(rel_path);
            }
        }
        // println!("    >> Best match for path {}: {}", path, best_match.clone().unwrap_or("".to_string()));
        best_match
    }

    #[allow(dead_code)] // NOTE: might be useful later
    /// Gets direct child fields based on the original TOML structure.
    ///
    /// # Parameters
    /// - `this_idx`: Index of the parent field
    ///
    /// # Returns
    /// A new `TomlFields` containing only direct children of the specified field
    pub fn get_toml_children_of(&self, this_idx: usize) -> TomlFields<'a> {
        TomlFields::<'a> {
            fields: self
                .fields
                .iter()
                .filter(|field| {
                    field.parent == Some(this_idx)
                        && self.index_of(field).unwrap_or(usize::MAX) != this_idx
                })
                .cloned()
                .collect::<Vec<_>>(),
            patterns: self.patterns.clone(),
            root_value: self.root_value,
            aliases: self.aliases.clone(),
            comments: self.comments.clone(),
            widths: self.widths.clone(),
        }
    }

    #[allow(dead_code)] // NOTE: useful api for future
    pub fn get_relative_parent_of(&'a self, this_idx: usize) -> &'a TomlField<'a> {
        let this_field = self.get_field(this_idx).expect("Expected a valid field");
        self.get_relative_parent_of_field(this_field)
    }

    /// Finds the relative parent field based on effective module paths.
    ///
    /// Uses pattern-based heuristics to determine parent-child relationships in
    /// the generated module structure, which will in most cases differ from the original TOML hierarchy.
    ///
    /// # Parameters
    /// - `this_field`: Field to find the relative parent for
    ///
    /// # Returns
    /// Reference to the module parent field based on effective path
    /// (NOTE: not necessarily the same as the TOML document parent)
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
        let relative_parent_field = self.get_by_name(&relative_parent_name).unwrap_or_else(|| {
            panic!(
                "Expected a valid relative parent field ({} didn't exist, processing {})",
                relative_parent_name, this_field.name
            )
        });
        // println!("    >> Found relative parent for {}; field: {}", this_field.name, relative_parent_field.name);
        relative_parent_field
    }

    /// Gets children fields in the module hierarchy (not TOML hierarchy).
    ///
    /// Different from `get_toml_children_of` as this returns fields that will appear
    /// in the generated module rather than following the original TOML structure.
    /// # Parameters
    /// - `this_idx`: Index of the parent field
    ///
    /// # Returns
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
            widths: self.widths.clone(),
        }
    }

    /// The width the macro input names for a field, if it names one.
    ///
    /// A pattern is tried against the path the constant is made at and against the path of
    /// the key in the file, so that an alias is given its width by naming the original. Two
    /// patterns that name different widths for one key are a [`Problem`], since nothing
    /// says which was meant.
    fn width_of(&self, field: &TomlField<'a>) -> Result<Option<Width>, Problem> {
        let original = to_valid_ident(field.toml_path.as_deref().unwrap_or(&field.path));
        let mut found: Option<&WidthRule> = None;
        for rule in &self.widths {
            if !(rule.matcher.is_match(&field.path) || rule.matcher.is_match(&original)) {
                continue;
            }
            match found {
                Some(first) if first.width != rule.width => {
                    return Err(Problem {
                        key: field.key.clone(),
                        reason: format!(
                            "is given the width `{}` by `{}` and `{}` by `{}`. Name one.",
                            first.width.name(),
                            first.pattern,
                            rule.width.name(),
                            rule.pattern
                        ),
                    })
                },
                Some(_) => {},
                None => found = Some(rule),
            }
        }
        Ok(found.map(|rule| rule.width))
    }

    pub fn get_field(&self, idx: usize) -> Option<&TomlField<'a>> {
        self.fields.get(idx)
    }

    /// Finds a field by its name.
    ///
    /// # Parameters
    /// - `name`: Name of the field to find
    ///
    /// # Returns
    /// Reference to the found field, or None if no field with that name exists
    pub fn get_by_name(&self, name: &str) -> Option<&TomlField<'a>> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn index_of(&self, field: &TomlField<'a>) -> Option<usize> {
        self.fields.iter().position(|f| f == field)
    }

    /// Extracts fields from a TOML value that match configured patterns.
    ///
    /// Recursively walks the TOML document structure, filtering fields based on:
    /// - Inclusion patterns
    /// - Exclusion patterns
    /// - Aliases
    ///
    /// # Parameters
    /// - `value`: TOML value to extract fields from
    /// - `_path`: Current path in the TOML hierarchy
    /// - `parent_idx`: Index of the parent field (for recursion)
    /// - `key_path`: Current path as the file writes it, for naming a field in an error
    ///
    pub fn extract_matched_paths_from_value(
        &mut self,
        value: &'a Value,
        _path: &str,
        parent_idx: usize,
        key_path: &str,
    ) {
        let orig_path = _path.to_string();
        let alias = self.aliases.as_ref().and_then(|aliases| {
            let _processed_path = to_valid_ident(_path);
            aliases.iter().find_map(move |(alias, orig)| {
                // println!("         >> Checking if alias matches for path {}: {}->{}", _path, orig, alias);

                // NOTE: checking both because this all got a bit messy and needs a bit of cleanup,
                //       and unsure presently if we clean these up before we pass them here or not
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
            let out = format!("{}.{}", _ps, alias);
            // println!("    >> Found alias for path {}: {}", _path, &out);
            out
        } else {
            _path.to_string()
        };
        let path = to_valid_ident(&aliased_path);
        // let path = aliased_path;
        // if path.contains('-') {
        //     println!("    >> Found unprocessed toml path in kebab-case: {}", path);
        // }
        // println!(" >> Extracting paths from: {}", path);
        let (mut field, field_idx) = if parent_idx == 0 && self.fields.is_empty() {
            (TomlField::root(value), 0)
        } else {
            let field = TomlField::new(
                path.split('.')
                    .next_back()
                    .expect("Expected a valid path to extract name from"),
                // path.split_once('.').unwrap_or((path, path)).1, // FIXME: this wont work with patterns like * or ** or **.** etc.
                &path,
                value,
                Some(parent_idx),
            )
            .with_alias(alias_name.as_deref().unwrap_or(ROOT))
            .with_toml_path(&orig_path)
            .with_key(key_path);

            (field, self.fields.len()) // idx where this field will be placed
        };

        match value {
            Value::Table(table) => {
                // NOTE: this is good for some additional logic we might want to add to tables (<=> modules)
                // println!("    >> Pushed table `{}`, recursing into it... ", &field.name);
                self.fields.push(field);
                for (key, val) in table.iter() {
                    let new_path =
                        if path.is_empty() { key.clone() } else { format!("{}.{}", path, key) };
                    let new_key_path = join_key(key_path, key);
                    self.extract_matched_paths_from_value(val, &new_path, field_idx, &new_key_path);
                }
            },
            _ => {
                // NOTE: this is good for some additional logic we might want to add to actual values (<=> consts)
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
                    // println!("    >> Pushed field `{}` (path: {})", &field.name, &path);
                    // if path.contains('-') {
                    //     println!("        >> A kebab-case ident got past our checks! ({})", path);
                    // }
                    self.fields.push(field);
                } else {
                    // println!("    >> Skipping field: {}", path);
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

impl<'a> TomlFields<'a> {
    /// Turns the fields into the tree of modules and constants they bind as.
    ///
    /// Starts from the root field, which has no name of its own.
    ///
    /// A value that cannot be bound is left out of the module and comes back as a
    /// [`Problem`], so one bad key does not hide what else is wrong or take the rest down.
    pub fn to_module(&self) -> (Module, Vec<Problem>) {
        let mut problems = Vec::new();
        let module = self.module_at(0, &mut problems);
        (module, problems)
    }

    /// Builds a single module from a field and its children.
    ///
    /// Recursively builds modules for table fields and constants for value fields.
    /// The structure of the result reflects the effective module paths
    /// derived from the TOML structure and the applied patterns.
    fn module_at(&self, idx: usize, problems: &mut Vec<Problem>) -> Module {
        let field = self
            .get_field(idx)
            .expect("Expected a valid index to an existing field");
        // get module name (last component of path)
        let module_name = field
            .path
            .split('.')
            .next_back()
            .expect("Expected there to be at least one node from split by '.'");

        let name = if !module_name.is_empty() {
            Some(to_valid_ident(module_name).to_lowercase())
        } else {
            None
        };

        let relative_children_fields = self.get_relative_children_of(idx).fields;

        // constants for this module
        let mut consts = Vec::new();
        for f in relative_children_fields.iter().filter(|f| !f.is_table()) {
            let bound = self
                .width_of(f)
                .and_then(|width| bind_value(&f.key, f.value, width));
            match bound {
                Ok((ty, value)) => consts.push(Const {
                    name: to_valid_ident(&f.name).to_uppercase(),
                    comment: f.comment.clone(),
                    ty,
                    value,
                }),
                Err(problem) => problems.push(problem),
            }
        }

        // submodules for recursive hierarchy
        let modules = relative_children_fields
            .iter()
            .filter(|f| f.is_table())
            .map(|submod| {
                self.module_at(
                    self.index_of(submod)
                        .expect("Expected a valid child that exists and thus has an index"),
                    problems,
                )
            })
            .filter(|m| !m.is_empty())
            .collect();

        Module {
            name,
            comment: field.comment.clone(),
            consts,
            modules,
        }
    }
}
