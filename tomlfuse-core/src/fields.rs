//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::field::{TomlField, ROOT};
use crate::names::{join_key, snake_to_kebab, to_valid_ident};
use crate::pattern::Pattern;
use crate::widths::WidthRule;
use globset::GlobSet;
use std::collections::HashMap;
use toml::Value;

/// The patterns that filter which fields of a document are kept.
///
/// An exclusion overrides an inclusion for the same field. The literals are the patterns as
/// they were written, an exclusion with its `!`, which is what the relative paths are worked
/// out from.
#[derive(Clone, Debug, Default)]
pub(crate) struct Patterns {
    pub inclusions: Option<GlobSet>,
    pub exclusions: Option<GlobSet>,
    pub literals: Vec<String>,
}

/// The fields of a toml document that a section selected, and what is needed to turn them
/// into modules.
///
/// Walking the document fills `fields` in document order, a table before what is in it.
/// Everything else here refers to a field by its index in that list, never by comparing it
/// with another: a float that is NaN is not equal to itself, so a field holding one cannot be
/// found that way, nor can any table around it.
#[derive(Clone, Debug, Default)]
pub(crate) struct TomlFields<'a> {
    pub root_value: Option<&'a Value>,
    pub fields: Vec<TomlField<'a>>,
    pub patterns: Patterns,
    pub aliases: Option<HashMap<Pattern, Pattern>>,
    pub comments: Option<HashMap<String, String>>,
    pub widths: Vec<WidthRule>,
}

impl<'a> From<&'a Value> for TomlFields<'a> {
    fn from(value: &'a Value) -> Self {
        TomlFields::default().with_root(value)
    }
}

impl<'a> TomlFields<'a> {
    pub fn with_root(mut self, value: &'a Value) -> Self {
        self.root_value = Some(value);
        self
    }

    pub fn with_aliases(mut self, aliases: Option<HashMap<Pattern, Pattern>>) -> Self {
        self.aliases = aliases;
        self
    }

    pub fn with_inclusion_globs(mut self, inclusion_globs: Option<GlobSet>) -> Self {
        self.patterns.inclusions = inclusion_globs;
        self
    }

    pub fn with_exclusion_globs(mut self, exclusion_globs: Option<GlobSet>) -> Self {
        self.patterns.exclusions = exclusion_globs;
        self
    }

    pub fn with_pat_literals(mut self, patterns: Vec<String>) -> Self {
        self.patterns.literals = patterns;
        self
    }

    pub fn with_widths(mut self, widths: Vec<WidthRule>) -> Self {
        self.widths = widths;
        self
    }

    pub fn with_comments(mut self, comments: HashMap<String, String>) -> Self {
        self.comments = Some(comments);
        self
    }

    /// Builds the fields collection by extracting matched paths from the toml document.
    ///
    /// Processing steps:
    /// 1. Extracts all fields matching configured patterns and pre-processes them
    /// 2. Resolves relative paths for hierarchical organization
    /// 3. Associates comments with the corresponding fields
    pub fn build(mut self) -> Self {
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

        for field in &mut self.fields {
            let found = self.comments.as_ref().and_then(|c| {
                c.get(&field.path)
                    .or_else(|| c.get(&snake_to_kebab(&field.path)))
                    .or_else(|| {
                        let toml_path = field.toml_path.as_ref()?;
                        c.get(toml_path)
                            .or_else(|| c.get(&snake_to_kebab(toml_path)))
                    })
            });
            if let Some(comment) = found {
                field.comment = Some(comment.to_string());
            }
        }

        self
    }

    /// Finds the section a path belongs to and the relative path within that section.
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
            let rel_path = path
                .split('.')
                .filter(|s| !s.is_empty() && !pat_segs.contains(s))
                .collect::<Vec<_>>()
                .join(".");
            if best_match
                .as_ref()
                .is_none_or(|best| best.len() > rel_path.len())
            {
                // FIXME: needs a bit better way to determine the best match ":D"
                best_match = Some(rel_path);
            }
        }
        best_match
    }

    /// Whether the patterns keep a leaf at `path`.
    fn keeps(&self, path: &str) -> bool {
        path == ROOT
            || (self
                .patterns
                .inclusions
                .as_ref()
                .is_none_or(|set| set.is_match(path))
                && self
                    .patterns
                    .exclusions
                    .as_ref()
                    .is_none_or(|set| !set.is_match(path)))
    }

    /// Extracts fields from a toml value that match configured patterns.
    ///
    /// Recursively walks the toml document structure, filtering fields based on:
    /// - Inclusion patterns
    /// - Exclusion patterns
    /// - Aliases
    ///
    /// # Parameters
    /// - `value`: toml value to extract fields from
    /// - `_path`: Current path in the toml hierarchy
    /// - `parent_idx`: Index of the parent field (for recursion)
    /// - `key_path`: Current path as the file writes it, for naming a field in an error
    fn extract_matched_paths_from_value(
        &mut self,
        value: &'a Value,
        _path: &str,
        parent_idx: usize,
        key_path: &str,
    ) {
        let orig_path = _path.to_string();
        let alias = self.aliases.as_ref().and_then(|aliases| {
            // NOTE: checking both because this all got a bit messy and needs a bit of cleanup,
            //       and unsure presently if we clean these up before we pass them here or not
            let processed_path = to_valid_ident(_path);
            aliases.iter().find_map(|(alias, orig)| {
                (_path == orig.as_str() || processed_path == orig.as_str())
                    .then(|| alias.to_string())
            })
        });
        let is_alias = alias.is_some();
        let aliased_path = if let Some(alias) = alias {
            let segments = _path.split('.').collect::<Vec<_>>();
            let parent = segments[..segments.len().saturating_sub(1)].join(".");
            format!("{parent}.{alias}")
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
            )
            .with_toml_path(&orig_path)
            .with_key(key_path);

            (field, self.fields.len()) // idx where this field will be placed
        };

        match value {
            Value::Table(table) => {
                self.fields.push(field);
                for (key, val) in table.iter() {
                    let new_path =
                        if path.is_empty() { key.clone() } else { format!("{}.{}", path, key) };
                    let new_key_path = join_key(key_path, key);
                    self.extract_matched_paths_from_value(val, &new_path, field_idx, &new_key_path);
                }
            },
            _ => {
                let mut skip = !self.keeps(&path);
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
