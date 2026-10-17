//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Error;
use crate::field::TomlFields;
use crate::model::Module;
use crate::pattern::Pattern;
use crate::source::Source;
use globset::GlobSetBuilder;
use std::collections::HashMap;

/// What to take out of a [`Source`] and what to call it.
///
/// This is one `[name]` block of a macro invocation: the patterns that select keys, the ones
/// that remove them again, and the keys that go by another name.
#[derive(Clone, Debug, Default)]
pub struct Section {
    /// Name of the module the section becomes.
    pub name: String,
    /// Patterns for fields to include
    pub include: Vec<Pattern>,
    /// Patterns for fields to exclude. These win over the inclusions.
    pub exclude: Vec<Pattern>,
    /// Keys that go by another name, where the key is the alias and the value is the path of
    /// the original.
    pub aliases: HashMap<Pattern, Pattern>,
}

/// Walks `source` through the patterns of `section` and returns what is left, as a tree of
/// modules and constants.
///
/// Errors when a pattern is not a glob. A pattern that matches nothing is not an error, it
/// just selects nothing.
pub fn bind(source: &Source, section: &Section) -> Result<Module, Error> {
    let mut inclusions = GlobSetBuilder::new();
    let mut exclusions = GlobSetBuilder::new();
    let mut literals: Vec<String> = Vec::new();
    // The message names the pattern and what globset objected to, because it is the whole
    // diagnostic. "Expected a valid glob pat string" named neither, leaving nothing to act on.
    for pattern in &section.include {
        inclusions.add(pattern.compile()?);
        literals.push(pattern.to_string());
    }
    for pattern in &section.exclude {
        exclusions.add(pattern.compile()?);
        literals.push(format!("!{}", pattern));
    }
    let build_set = |builder: GlobSetBuilder, patterns: &[Pattern]| {
        builder.build().map_err(|e| Error::BadPattern {
            pattern: patterns.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
            reason: e.to_string(),
        })
    };
    let fields = TomlFields::from(source.value())
        .with_inclusion_globs(Some(build_set(inclusions, &section.include)?))
        .with_exclusion_globs(Some(build_set(exclusions, &section.exclude)?))
        .with_pat_literals(literals)
        .with_comments(source.comments().clone())
        .with_aliases(Some(section.aliases.clone()))
        .build();

    let mut module = fields.to_module();
    module.name = Some(section.name.clone());
    Ok(module)
}
