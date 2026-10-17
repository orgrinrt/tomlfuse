//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::error::Problem;
use crate::field::ROOT;
use crate::fields::TomlFields;
use crate::model::{Const, Module};
use crate::names::to_valid_ident;
use crate::value::bind_value;

impl TomlFields<'_> {
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
    /// derived from the toml structure and the applied patterns.
    fn module_at(&self, idx: usize, problems: &mut Vec<Problem>) -> Module {
        let field = &self.fields[idx];
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

        let children = self.relative_children_of(idx);

        // constants for this module
        let mut consts = Vec::new();
        for f in children
            .iter()
            .map(|&i| &self.fields[i])
            .filter(|f| !f.is_table())
        {
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
        let modules = children
            .iter()
            .filter(|&&i| self.fields[i].is_table())
            .map(|&i| self.module_at(i, problems))
            .filter(|m| !m.is_empty())
            .collect();

        Module {
            name,
            comment: field.comment.clone(),
            consts,
            modules,
        }
    }

    /// The fields that sit directly in the module `idx` becomes, which follows the module
    /// hierarchy the patterns gave and not the one of the toml document.
    fn relative_children_of(&self, idx: usize) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|&i| i != idx && self.relative_parent_of(i) == idx)
            .collect()
    }

    /// The index of the field whose module the field at `idx` sits in, going by effective
    /// module paths. Not necessarily the field's parent in the toml document.
    fn relative_parent_of(&self, idx: usize) -> usize {
        let field = &self.fields[idx];
        let path = field.effective_module_path();
        let parent_name = match path.len().checked_sub(2) {
            Some(last) => path[last].as_str(),
            None => ROOT,
        };
        self.fields
            .iter()
            .position(|f| f.name == parent_name)
            .unwrap_or_else(|| {
                panic!(
                    "Expected a valid relative parent field ({} didn't exist, processing {})",
                    parent_name, field.name
                )
            })
    }
}
