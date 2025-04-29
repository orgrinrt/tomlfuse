//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::module::{RootModule, RootModuleSource};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Result as SynResult};

pub struct MacroInput {
    pub toml_path: Option<String>,
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
