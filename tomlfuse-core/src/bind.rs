//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::{Error, Problem};
use crate::fields::TomlFields;
use crate::model::{Module, Width};
use crate::pattern::Pattern;
use crate::source::Source;
use crate::widths::WidthRule;
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
    /// The type to bind the numbers of each key at, where it is not `i64` or `f64`. A pattern
    /// is matched against the key's path in the file, or against the path of the original
    /// when the key goes by an alias.
    pub widths: Vec<(Pattern, Width)>,
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
            pattern: patterns
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            reason: e.to_string(),
        })
    };
    let mut widths = Vec::with_capacity(section.widths.len());
    for (pattern, width) in &section.widths {
        widths.push(WidthRule {
            pattern: pattern.clone(),
            matcher: pattern.compile()?.compile_matcher(),
            width: *width,
        });
    }
    let fields = TomlFields::from(source.value())
        .with_inclusion_globs(Some(build_set(inclusions, &section.include)?))
        .with_exclusion_globs(Some(build_set(exclusions, &section.exclude)?))
        .with_pat_literals(literals)
        .with_comments(source.comments().clone())
        .with_aliases(Some(section.aliases.clone()))
        .with_widths(widths)
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
    use crate::model::{FloatTy, IntTy, Ty};
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

        let names: Vec<_> = bound
            .module
            .consts
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(
            names.contains(&"FINE") && names.contains(&"ALSO_FINE"),
            "{names:?}"
        );

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

    fn at(widths: &[(&str, &str)], patterns: &[&str]) -> Section {
        Section {
            widths: widths
                .iter()
                .map(|(p, w)| (Pattern::new(*p), Width::parse(w).unwrap()))
                .collect(),
            ..section(patterns)
        }
    }

    fn ty_of(bound: &Bound, name: &str) -> Ty {
        bound
            .module
            .consts
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .ty
            .clone()
    }

    #[test]
    fn a_width_reaches_the_keys_its_pattern_matches_and_no_others() {
        let source = source("[size]\nvoxel = 17\nframes = 30\nratio = 0.5\n");

        let bound = bind(
            &source,
            &at(
                &[("size.{voxel,frames}", "u8"), ("size.ratio", "f32")],
                &["size.*"],
            ),
        )
        .unwrap();

        assert!(bound.problems.is_empty(), "{:?}", bound.problems);
        assert_eq!(ty_of(&bound, "VOXEL"), Ty::Int(IntTy::U8));
        assert_eq!(ty_of(&bound, "FRAMES"), Ty::Int(IntTy::U8));
        assert_eq!(ty_of(&bound, "RATIO"), Ty::Float(FloatTy::F32));

        let bound = bind(&source, &at(&[("size.voxel", "u8")], &["size.*"])).unwrap();
        assert_eq!(ty_of(&bound, "FRAMES"), Ty::Int(IntTy::I64));
        assert_eq!(ty_of(&bound, "RATIO"), Ty::Float(FloatTy::F64));
    }

    #[test]
    fn a_key_with_a_dash_takes_the_width_of_the_pattern_that_spells_it_with_an_underscore() {
        let source = source("[tuning]\ntone-soft = 0.55\n");

        let bound = bind(&source, &at(&[("tuning.tone_soft", "f32")], &["tuning.*"])).unwrap();

        assert_eq!(ty_of(&bound, "TONE_SOFT"), Ty::Float(FloatTy::F32));
    }

    #[test]
    fn an_alias_takes_the_width_given_for_the_original() {
        let source = source("[size]\nvoxel = 17\n");
        let mut section = at(&[("size.voxel", "u16")], &[]);
        section
            .aliases
            .insert(Pattern::new("px"), Pattern::new("size.voxel"));

        let bound = bind(&source, &section).unwrap();

        assert!(bound.problems.is_empty(), "{:?}", bound.problems);
        assert_eq!(ty_of(&bound, "PX"), Ty::Int(IntTy::U16));
    }

    #[test]
    fn two_different_widths_for_one_key_are_a_problem_and_the_same_one_twice_is_not() {
        let source = source("[size]\nvoxel = 17\n");

        let bound = bind(
            &source,
            &at(&[("size.*", "u8"), ("size.voxel", "u32")], &["size.*"]),
        )
        .unwrap();
        assert_eq!(bound.problems.len(), 1);
        assert_eq!(bound.problems[0].key, "size.voxel");
        assert_eq!(
            bound.problems[0].reason,
            "is given the width `u8` by `size.*` and `u32` by `size.voxel`. Name one."
        );
        assert!(bound.module.consts.is_empty());

        let bound = bind(
            &source,
            &at(&[("size.*", "u8"), ("size.voxel", "u8")], &["size.*"]),
        )
        .unwrap();
        assert!(bound.problems.is_empty(), "{:?}", bound.problems);
    }

    #[test]
    fn a_nan_is_a_problem_and_loses_neither_its_siblings_nor_its_table() {
        // A float that is not equal to itself used to make its own field unfindable, and the
        // table around it too, so the walker panicked looking for either.
        let source = source("[odd]\nfine = 1\nnot-a-number = nan\n[odd.inner]\nalso = 2\n");

        let bound = bind(&source, &section(&["odd.**"])).unwrap();

        assert_eq!(bound.problems.len(), 1, "{:?}", bound.problems);
        assert_eq!(bound.problems[0].key, "odd.not-a-number");
        let names = |module: &Module| {
            module
                .consts
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&bound.module), ["FINE"]);
        assert_eq!(bound.module.modules.len(), 1);
        assert_eq!(names(&bound.module.modules[0]), ["ALSO"]);
    }
}
