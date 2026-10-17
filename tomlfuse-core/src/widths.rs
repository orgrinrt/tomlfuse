//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::{Error, Problem};
use crate::field::TomlField;
use crate::fields::TomlFields;
use crate::model::Width;
use crate::names::to_valid_ident;
use crate::pattern::Pattern;
use globset::GlobMatcher;
use toml::Value;

/// A width, and the pattern that says which keys it is for.
#[derive(Clone, Debug)]
pub(crate) struct WidthRule {
    /// The pattern as written, for saying which one an error is about.
    pub pattern: Pattern,
    /// The compiled pattern.
    pub matcher: GlobMatcher,
    /// The type to bind the matching keys at.
    pub width: Width,
}

impl TomlFields<'_> {
    /// The width the macro input names for a field, if it names one.
    ///
    /// A pattern is tried against the path the constant is made at and against the path of
    /// the key in the file, so that an alias is given its width by naming the original. Two
    /// patterns that name different widths for one key are a [`Problem`], since nothing
    /// says which was meant.
    pub fn width_of(&self, field: &TomlField<'_>) -> Result<Option<Width>, Problem> {
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
}

/// Checks that each width has a number in the file to apply to.
///
/// A pattern is matched against the path of every key in the document, as the walker names
/// it, whether or not the section selects the key. One that matches nothing is almost
/// certainly a typo, and one that matches only tables is too shallow to reach a number. Left
/// alone, both bind the keys they were meant for as `i64` or `f64` without a word.
pub(crate) fn check_used(rules: &[WidthRule], root: &Value) -> Result<(), Error> {
    let mut leaves = Vec::new();
    let mut tables = Vec::new();
    paths(root, "", &mut leaves, &mut tables);
    for rule in rules {
        if leaves.iter().any(|path| rule.matcher.is_match(path)) {
            continue;
        }
        return Err(Error::UnusedWidth {
            pattern: rule.pattern.to_string(),
            width: rule.width.name(),
            only_tables: tables.iter().any(|path| rule.matcher.is_match(path)),
        });
    }
    Ok(())
}

/// Every key path under `value`, split by whether it is a table, spelled as the walker does.
fn paths(value: &Value, at: &str, leaves: &mut Vec<String>, tables: &mut Vec<String>) {
    let Value::Table(table) = value else {
        return;
    };
    for (key, inner) in table {
        let path = to_valid_ident(&if at.is_empty() { key.clone() } else { format!("{at}.{key}") });
        if inner.is_table() {
            tables.push(path.clone());
            paths(inner, &path, leaves, tables);
        } else {
            leaves.push(path);
        }
    }
}
