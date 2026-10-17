//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::{Error, Problem};
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

/// What binding a [`Section`] came to.
#[derive(Clone, Debug)]
pub struct Bound {
    /// Everything that could be bound, under the section's name.
    pub module: Module,
    /// The values that were selected and could not be bound. They are not in the module.
    pub problems: Vec<Problem>,
}

/// Walks `source` through the patterns of `section` and returns what is left, as a tree of
/// modules and constants.
///
/// Errors when a pattern is not a glob. A pattern that matches nothing is not an error, it
/// just selects nothing. A selected value that has no constant to be, a mixed array or a
/// datetime, is a [`Problem`] in the result, and the rest is bound regardless.
pub fn bind(source: &Source, section: &Section) -> Result<Bound, Error> {
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

    let (mut module, problems) = fields.to_module();
    module.name = Some(section.name.clone());
    Ok(Bound {
        module,
        problems,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn source(text: &str) -> Source {
        Source::parse("test.toml", PathBuf::from("test.toml"), text).unwrap()
    }

    fn section(patterns: &[&str]) -> Section {
        Section {
            name: "test".into(),
            include: patterns.iter().map(|p| Pattern::new(*p)).collect(),
            ..Section::default()
        }
    }

    #[test]
    fn a_value_with_no_constant_to_be_is_a_problem_and_the_rest_still_binds() {
        let source = source(
            "[row]\nfine = 1\nmixed = [1, \"two\"]\nwhen = 2026-10-17T09:30:00Z\nalso-fine = true\n",
        );

        let bound = bind(&source, &section(&["row.*"])).unwrap();

        let names: Vec<_> = bound.module.consts.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"FINE") && names.contains(&"ALSO_FINE"), "{names:?}");

        let mut keys: Vec<_> = bound.problems.iter().map(|p| p.key.as_str()).collect();
        keys.sort();
        assert_eq!(keys, ["row.mixed", "row.when"]);
    }

    #[test]
    fn a_problem_names_the_key_as_the_file_writes_it() {
        // Not as the constant is named: the dash is the file's, and so is the quoting.
        let source = source("[size]\nvoxel-px = [1, \"a\"]\n\"odd key\" = 2026-10-17T09:30:00Z\n");

        let bound = bind(&source, &section(&["size.*"])).unwrap();

        let mut keys: Vec<_> = bound.problems.iter().map(|p| p.key.clone()).collect();
        keys.sort();
        assert_eq!(keys, ["size.\"odd key\"", "size.voxel-px"]);
    }

    #[test]
    fn a_value_the_patterns_leave_out_is_not_a_problem() {
        let source = source("[row]\nmixed = [1, \"two\"]\nfine = 1\n");

        let bound = bind(&source, &section(&["row.fine"])).unwrap();

        assert!(bound.problems.is_empty(), "{:?}", bound.problems);
    }
}
