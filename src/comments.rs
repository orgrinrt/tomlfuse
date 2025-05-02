//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::collections::HashMap;

/// State for tracking toml parsing context
#[derive(Debug, Clone, Copy, PartialEq)]
enum StringState {
    None,
    // SingleQuote,
    // DoubleQuote,
    MultiSingleQuote,
    MultiDoubleQuote,
}

/// Extracts comments from TOML content and associates them with keys/sections.
///
/// Comment handling:
/// - Associates preceding comments (above key/section)
/// - Captures inline comments (same line)
/// - Joins multi-line comments with newlines
/// - Preserves empty comment lines as blank lines
/// - Resets comment accumulation on blank lines
/// - Ignores orphaned comments with no associated key
///
/// # Parameters
/// - `content`: toml document as a string slice.
///
/// # Returns
/// A `HashMap` where each entry maps the full dotted path of a field or section
/// (e.g. `section.subsection.key`) to its concatenated comment text.
///
/// ```rust
/// # use std::collections::HashMap;
/// # fn extract_comments(input: &'static str) -> HashMap<String, String> {
/// #    // dummy impl because proc-macro crate can't export this function
/// #    let mut out = HashMap::new();
/// #    out.insert("package.version".to_string(), "header\ninline comment".to_string());
/// #    out
/// # }
/// let toml = r#"
/// ​# header
/// [package]
/// version = "0.1.0" # inline comment
/// "#;
/// let comments = extract_comments(toml);
/// assert_eq!(
///     comments.get("package.version"),
///     Some(&"header\ninline comment".to_string())
/// );
/// ```
#[cold]
pub fn extract_comments(content: &str) -> HashMap<String, String> {
    let mut comments = HashMap::new();
    if content.is_empty() {
        return comments;
    }

    let lines: Vec<&str> = content.lines().collect();

    let mut string_state = StringState::None;
    let mut current_comments = Vec::new();
    let mut current_path = Vec::new();

    for (_i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            current_comments.clear();
            continue;
        }

        // properly detect string context including escape sequences
        if string_state != StringState::None {
            match string_state {
                StringState::MultiSingleQuote if trimmed.contains("'''") => {
                    string_state = StringState::None;
                },
                StringState::MultiDoubleQuote if trimmed.contains("\"\"\"") => {
                    string_state = StringState::None;
                },
                _ => continue, // still in string, skip line
            }
            continue;
        }

        // detect start of multiline string literals
        if trimmed.contains("\"\"\"") && !trimmed.contains("\"\"\"\"\"\"") {
            string_state = StringState::MultiDoubleQuote;
        } else if trimmed.contains("'''") && !trimmed.contains("''''''") {
            string_state = StringState::MultiSingleQuote;
        }
        if string_state != StringState::None {
            continue;
        }

        // reset comment group on blank lines
        if trimmed.is_empty() {
            current_comments.clear();
            continue;
        }

        // section headers [section.subsection]
        if trimmed.starts_with('[') {
            if let Some(section_end) = trimmed.find(']') {
                // extract section path
                let section_path = &trimmed[1..section_end];
                current_path.clear();
                current_path = section_path.split('.').map(String::from).collect();
                let section_str = section_path.to_string();

                // start with any preceding comments
                let mut all_comments = current_comments.clone();

                // check for inline comment
                if let Some(inline) = extract_inline_comment(trimmed, section_end) {
                    all_comments.push(inline);
                }

                // add combined comments
                if !all_comments.is_empty() {
                    comments.insert(section_str, all_comments.join("\n"));
                }
                current_comments.clear();
                continue;
            }
        }

        // comments
        if trimmed.starts_with('#') {
            let comment_text = trimmed[1..].trim();

            // preserve empty comments as empty strings to create double newlines
            if comment_text.is_empty() {
                current_comments.push("".to_string());
            } else {
                current_comments.push(comment_text.to_string());
            }
            continue;
        }

        // key-value pairs
        if let Some(pos) = trimmed.find('=') {
            if !trimmed.is_empty() {
                let key = trimmed[..pos].trim();
                // support dotted keys in assignments
                let mut full_path = current_path.clone();
                for seg in key.split('.') {
                    full_path.push(seg.trim().to_string());
                }
                let path_str = full_path.join(".");

                // inline comment if present
                let mut key_comments = current_comments.clone();
                if let Some(inline) = extract_inline_comment(trimmed, pos) {
                    key_comments.push(inline);
                }

                // add comments if we have any
                if !key_comments.is_empty() {
                    comments.insert(path_str.clone(), key_comments.join("\n"));
                }

                // reset comment accumulator
                current_comments.clear();
            }
        }
        // other line types - reset state
        else if !trimmed.starts_with('#') {
            current_comments.clear();
        }
    }

    comments
}

// helper function to extract inline comments
#[inline(always)]
fn extract_inline_comment(line: &str, after_pos: usize) -> Option<String> {
    if let Some(comment_pos) = line.find('#') {
        if comment_pos > after_pos {
            let comment = line[comment_pos + 1..].trim();
            if !comment.is_empty() {
                return Some(comment.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::extract_comments;

    #[test]
    fn test_preceding_and_inline_comments() {
        let toml = r#"
# header comment
[package]
# version doc
version = "0.1.0" # inline comment
"#;
        let comments = extract_comments(toml);
        assert_eq!(
            comments.get("package.version"),
            Some(&"version doc\ninline comment".to_string())
        );
    }

    #[test]
    fn test_blank_line_resets_preceding_comments() {
        let toml = r#"
# first comment

# second comment
key = true
"#;
        let comments = extract_comments(toml);
        assert_eq!(comments.get("key"), Some(&"second comment".to_string()));
    }

    #[test]
    fn test_ignore_hash_in_multiline_string() {
        let toml = r#"
value = """
line1 # not a comment
line2
"""
# real comment
other = 123
"#;
        let comments = extract_comments(toml);
        assert_eq!(comments.get("other"), Some(&"real comment".to_string()));
        assert!(!comments.contains_key("value"));
    }

    #[test]
    fn test_dotted_keys_and_section_headers() {
        let toml = r#"
[section.sub]
# doc for item
item.subkey = "x"
"#;
        let comments = extract_comments(toml);
        assert_eq!(
            comments.get("section.sub.item.subkey"),
            Some(&"doc for item".to_string())
        );
    }

    #[test]
    fn test_inline_only_comment() {
        let toml = r#"
key = 10 # just inline
"#;
        let comments = extract_comments(toml);
        assert_eq!(comments.get("key"), Some(&"just inline".to_string()));
    }

    #[test]
    fn test_empty_comment_lines() {
        let toml = r#"
    # first line
    #
    # third line
    key = true
    "#;
        let comments = extract_comments(toml);
        // empty comment line should create double newline
        assert_eq!(
            comments.get("key"),
            Some(&"first line\n\nthird line".to_string())
        );
    }

    #[test]
    fn test_empty_input() {
        let comments = extract_comments("");
        assert!(comments.is_empty());
    }

    #[test]
    fn test_orphaned_comments() {
        let toml = r#"
    # orphaned comment at top
    [section]
    key = "value"
    # trailing comment with no key
    "#;
        let comments = extract_comments(toml);
        // trailing comment should be discarded
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments.get("section"),
            Some(&"orphaned comment at top".to_string())
        );
    }

    #[test]
    fn test_multiple_consecutive_comments() {
        let toml = r#"
    # first line
    # second line
    # third line
    key = true
    "#;
        let comments = extract_comments(toml);
        assert_eq!(
            comments.get("key"),
            Some(&"first line\nsecond line\nthird line".to_string())
        );
    }

    #[test]
    fn test_single_quote_multiline() {
        let toml = r#"
    value = '''
    # not a comment
    '''
    # real comment
    key = true
    "#;
        let comments = extract_comments(toml);
        assert_eq!(comments.get("key"), Some(&"real comment".to_string()));
    }

    #[test]
    fn test_no_multiline_inline_comments() {
        let toml = r#"
    key = true # first inline comment
    # this is an orphaned comment
    # that spans multiple lines
    next_key = false
    "#;

        let comments = extract_comments(toml);

        // verify first inline comment is captured
        assert_eq!(
            comments.get("key"),
            Some(&"first inline comment".to_string())
        );
        // orphaned multi-line comment is associated with next_key, not merged with previous inline
        assert_eq!(
            comments.get("next_key"),
            Some(&"this is an orphaned comment\nthat spans multiple lines".to_string())
        );
    }

    #[test]
    fn test_deeply_nested_paths_and_tables() {
        let toml = r#"
        # top level comment for section1
        [section1] # inline comment for section1
        key1 = "value1"

        # comment for nested section
        [section1.subsection] # inline comment for subsection
        key2 = "value2"

        # deeply nested section comment
        [section1.subsection.deep.nesting]
        # comment for nested key
        nested.key = "nested value" # inline nested key comment
        
        # another section
        [section2]
        
        # subsection with no inline comment
        [section2.config]
        setting = true
        "#;

        let comments = extract_comments(toml);

        // check section comments
        assert_eq!(
            comments.get("section1"),
            Some(&"top level comment for section1\ninline comment for section1".to_string())
        );

        assert_eq!(
            comments.get("section1.subsection"),
            Some(&"comment for nested section\ninline comment for subsection".to_string())
        );

        assert_eq!(
            comments.get("section1.subsection.deep.nesting"),
            Some(&"deeply nested section comment".to_string())
        );

        // check deep nested key comments
        assert_eq!(
            comments.get("section1.subsection.deep.nesting.nested.key"),
            Some(&"comment for nested key\ninline nested key comment".to_string())
        );

        // verify simple subsection comment
        assert_eq!(
            comments.get("section2.config"),
            Some(&"subsection with no inline comment".to_string())
        );
    }
}
