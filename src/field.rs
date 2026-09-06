//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use globset::GlobSet;
use once_cell::sync::Lazy;
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
    /// Optional alias for the field, if specified in the macro
    pub alias: Option<String>,
    /// Index of parent field in the fields collection
    pub parent: Option<usize>,
    /// Comment associated with this field from the TOML file
    pub comment: Option<String>,
}

impl Default for TomlField<'_> {
    fn default() -> Self {
        static DEFAULT_VALUE: Lazy<Value> = Lazy::new(|| Value::from(""));

        TomlField {
            name: String::default(),
            value: &DEFAULT_VALUE,
            path: String::default(),
            relative_path: None,
            toml_path: None,
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
            alias: None,
            parent: None,
            comment: None,
        }
    }
    pub fn with_toml_path(mut self, toml_path: &str) -> Self {
        self.toml_path = Some(toml_path.to_string());
        self
    }
    pub fn with_alias(mut self, alias: &str) -> Self {
        if alias == ROOT {
            return self;
        }
        self.alias = Some(alias.to_string());
        self
    }
    pub fn with_value(mut self, value: &'a Value) -> Self {
        self.value = value;
        self
    }

    /// Whether this field is a table, which becomes a module rather than a constant.
    pub fn is_table(&self) -> bool {
        matches!(self.value, Value::Table(_))
    }

    /// The path this field takes in the generated module tree.
    ///
    /// The relative path where a pattern established one, which is what strips the
    /// segments the pattern named literally; the full toml path otherwise.
    pub fn effective_module_path(&self) -> Vec<String> {
        self.relative_path
            .as_deref()
            .unwrap_or(&self.path)
            .split('.')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
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
}
