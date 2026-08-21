//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::module::{RootModule, RootModuleSource};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Result as SynResult};

/// Parsed representation of the input to `tomlfuse` macros.
///
/// Stores the macro parameters:
/// 1. Path to the TOML file (optional for some convenience macros)
/// 2. Module source configurations (patterns, sections, aliases)
///
/// This structure is created during macro parsing and used to drive
/// the code generation process.
pub struct MacroInput {
    /// Optional path to the TOML file
    pub toml_path: Option<String>,
    /// Collection of module configurations from the macro input
    /// Each represents a separate module to generate
    pub root_module_sources: Vec<RootModuleSource>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let toml_path = if input.peek(LitStr) {
            let toml_path_lit: LitStr = input.parse()?;
            Some(toml_path_lit.value())
        } else {
            None
        };

        let mut module_sources = Vec::new();
        while !input.is_empty() {
            let module_source: RootModuleSource = input.parse()?;
            module_sources.push(module_source);
        }

        Ok(MacroInput {
            toml_path,
            root_module_sources: module_sources,
        })
    }
}

impl ToTokens for MacroInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let module_sources = self.root_module_sources.iter();
        let modules = module_sources.map(move |source| {
            RootModule::new(source.clone(), self.toml_path.as_deref().unwrap_or(""))
        });

        tokens.extend(quote! {
            #(#modules)*
        });
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::MacroInput;

    /// The generated tokens for one binding over `tests/ui/fixture.toml`.
    ///
    /// Paths resolve against `CARGO_MANIFEST_DIR`, which for a unit test is this crate's
    /// root, so the fixture the `trybuild` cases use is reachable from here unchanged.
    fn generated(body: &str) -> String {
        let input: MacroInput = syn::parse_str(body).expect("the binding parses");
        input.to_token_stream().to_string()
    }

    /// A comment in the toml reaches the item it was written above.
    ///
    /// `extract_comments` is tested where it lives and so is `get_doc_comment`, and between
    /// them they cover reading a comment and building an attribute out of one. What neither
    /// covers is whether the attribute ends up on the generated item, which is the whole
    /// feature and is what a reader of `cargo doc` sees.
    ///
    /// It used to be checkable by denying `missing_docs` in a consumer. That stopped being
    /// possible when the generated code started allowing the lint, which it does because a
    /// table brought into being by a dotted key has nowhere to carry a comment, so the lint
    /// was unsatisfiable rather than demanding.
    #[test]
    fn a_toml_comment_becomes_documentation_on_the_item_it_sits_above() {
        let tokens = generated("\"tests/ui/fixture.toml\" [conf] app.*");

        // The fixture reads: `# What it calls itself.` above `name = "orrery"`.
        assert!(
            tokens.contains("What it calls itself."),
            "the comment above `name` did not reach the constant it documents:\n{tokens}"
        );
        // And `# The application's own identity.` above `[app]`, which documents nothing
        // here because `app.*` binds the keys rather than the table. The next test is the
        // one that takes the table.
        assert!(
            tokens.contains("doc ="),
            "nothing in the expansion is a doc attribute at all:\n{tokens}"
        );
    }

    /// A table's comment documents the module the table becomes.
    #[test]
    fn a_table_comment_becomes_documentation_on_the_module() {
        let tokens = generated("\"tests/ui/fixture.toml\" [conf] **");

        assert!(
            tokens.contains("The application's own identity."),
            "the comment above `[app]` did not reach the `app` module:\n{tokens}"
        );
        assert!(
            tokens.contains("How it reaches the network."),
            "the comment above `[server]` did not reach the `server` module:\n{tokens}"
        );
    }

    /// An uncommented key generates no doc attribute, which is what makes the two tests
    /// above mean something.
    ///
    /// Without this, a change that documented every item with a fixed string would satisfy
    /// them: the assertion is `contains`, and a doc attribute holding the wrong text is
    /// still a doc attribute. `bare.value` carries no comment in the fixture, so a binding
    /// that takes only it must produce no `doc` at all beyond the section's own line.
    #[test]
    fn an_uncommented_key_generates_no_documentation_of_its_own() {
        let tokens = generated("\"tests/ui/fixture.toml\" [only_bare] bare.value");

        // The section module carries a generated line naming the file it came from. That is
        // the one doc attribute this binding may produce, and the constant may not have one.
        let docs = tokens.matches("doc =").count();
        assert_eq!(
            docs, 1,
            "a binding over one uncommented key produced {docs} doc attributes where only \
             the section module's own line was expected:\n{tokens}"
        );
        assert!(
            tokens.contains("Constants bound from `fixture.toml`."),
            "the one doc attribute is not the section module's:\n{tokens}"
        );
    }
}
