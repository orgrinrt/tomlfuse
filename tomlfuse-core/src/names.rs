//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::field::ROOT;

/// Converts a TOML key into a valid Rust identifier string.
///
/// Transformations applied:
///
/// 1. Strips surrounding quotes if present
/// 2. Replaces dashes with underscores (kebab-case to snake_case)
/// 3. Returns `ROOT` constant for empty input
///
/// # Parameters
/// - `input`: Raw TOML key to normalize
#[inline]
pub fn to_valid_ident(input: &str) -> String {
    // handle potentially somehow still quoted keys by removing quotes
    let i = input.trim_start_matches('"').trim_end_matches('"');
    // empty input handling
    if i.is_empty() {
        return ROOT.to_string(); // default name
    }
    kebab_to_snake(i)
}

/// Converts kebab-case to snake_case by replacing all dashes with underscores.
///
/// # Parameters
/// - `input`: String potentially containing dashes
///
/// Used for converting kebab-case to snake_case in toml keys.
#[inline]
pub fn kebab_to_snake(input: &str) -> String {
    // TODO: more sophisticated conversion and covering edge cases that I assume have to exist
    input.replace('-', "_")
}

/// Converts snake_case to kebab-case by replacing all underscores with dashes.
///
/// # Parameters
/// - `input`: String potentially containing underscores
///
/// Used for converting snake_case to kebab-case in TOML keys.
#[inline]
pub fn snake_to_kebab(input: &str) -> String {
    input.replace('_', "-")
}
// TODO: unit test for `snake_to_kebab`, but in practice unless we expand or add to this, it should do what it says on the tin

/// Joins a key onto the dotted path of the table it sits in, quoting it when it needs it.
///
/// This is the path as it is written in the file, which is the one a reader of an error can
/// search for. The paths the patterns match on have been through [`to_valid_ident`] and no
/// longer say whether a key had a dash.
pub fn join_key(parent: &str, key: &str) -> String {
    let bare = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    let segment = if bare {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('\\', "\\\\").replace('"', "\\\""))
    };
    if parent.is_empty() {
        segment
    } else {
        format!("{parent}.{segment}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_key() {
        assert_eq!(join_key("", "size"), "size");
        assert_eq!(join_key("size", "voxel-pixels"), "size.voxel-pixels");
        assert_eq!(join_key("a", "b.c"), "a.\"b.c\"");
        assert_eq!(join_key("", "has space"), "\"has space\"");
    }

    #[test]
    fn test_to_valid_ident() {
        assert_eq!(to_valid_ident(""), ROOT.to_string());
        assert_eq!(to_valid_ident("normal"), "normal");
        assert_eq!(to_valid_ident("with-dash"), "with_dash");
        assert_eq!(to_valid_ident("\"quoted\""), "quoted");
        assert_eq!(to_valid_ident("\"quoted-with-dash\""), "quoted_with_dash");
    }

    #[test]
    fn test_fix_dashes() {
        assert_eq!(kebab_to_snake("no-dashes-here"), "no_dashes_here");
        assert_eq!(kebab_to_snake("already_good"), "already_good");
        assert_eq!(kebab_to_snake("mixed-case_style"), "mixed_case_style");
        assert_eq!(kebab_to_snake("-leading-dash"), "_leading_dash");
        assert_eq!(kebab_to_snake("trailing-dash-"), "trailing_dash_");
    }
}
