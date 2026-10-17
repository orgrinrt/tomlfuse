//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

mod bind;
mod comments;
mod error;
pub mod field;
mod model;
mod names;
mod pattern;
mod source;
mod value;

pub use bind::{bind, Bound, Section};
pub use comments::extract_comments;
pub use error::{Error, Problem};
pub use model::{Const, FloatTy, IntTy, Lit, Module, Ty, Width};
pub use names::{join_key, kebab_to_snake, snake_to_kebab, to_valid_ident};
pub use pattern::Pattern;
pub use source::{find_workspace_root, is_workspace_root, Source};
pub use value::bind_value;
