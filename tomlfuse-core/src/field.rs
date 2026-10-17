//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use toml::Value;

/// The path of the root of the document, which has no name.
pub(crate) const ROOT: &str = "";

/// A single field extracted from a toml document.
///
/// Maps a toml node to its path, value and the metadata the module tree is built from. A
/// field is either a leaf value, which becomes a constant, or a table, which becomes a
/// module.
#[derive(Debug, Clone)]
pub(crate) struct TomlField<'a> {
    /// Name of the field, used as identifier in generated code
    pub name: String,
    /// Reference to the actual toml value
    pub value: &'a Value,
    /// Full dot-separated path to this field in normalized form
    pub path: String,
    /// Path relative to the matching pattern, for hierarchical organization
    pub relative_path: Option<String>,
    /// Original path in the toml file before normalization (preserves case)
    pub toml_path: Option<String>,
    /// The key as it is written in the file, quoted where a key needs it. Unlike the paths
    /// above, nothing in it has been through identifier normalization, so it is the one an
    /// error can name and a reader can search the file for.
    pub key: String,
    /// Comment associated with this field from the toml file
    pub comment: Option<String>,
}

impl<'a> TomlField<'a> {
    /// A field at `path`, with nothing known about it yet but its value.
    pub fn new(name: &str, path: &str, value: &'a Value) -> Self {
        TomlField {
            name: name.to_string(),
            value,
            path: path.to_string(),
            relative_path: None,
            toml_path: None,
            key: String::new(),
            comment: None,
        }
    }

    /// The document itself, which is the one field with no name.
    pub fn root(value: &'a Value) -> Self {
        TomlField::new(ROOT, ROOT, value)
    }

    pub fn with_toml_path(mut self, toml_path: &str) -> Self {
        self.toml_path = Some(toml_path.to_string());
        self
    }

    pub fn with_key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }

    /// Whether this field is a toml table, which is to say a module and not a constant.
    pub fn is_table(&self) -> bool {
        matches!(self.value, Value::Table(_))
    }

    /// The modules this field sits in, outermost first: its path relative to the pattern
    /// that selected it where there is one, and its full path where there is not.
    pub fn effective_module_path(&self) -> Vec<String> {
        self.relative_path
            .as_deref()
            .unwrap_or(&self.path)
            .split('.')
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect()
    }
}
