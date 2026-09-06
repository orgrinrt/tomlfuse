//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::collections::HashMap;

/// Which multi-line string, if any, the scan is inside of.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StringState {
    None,
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

    for line in lines.iter() {
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

        // A table header, `[section.sub]`, or an array-of-tables header, `[[section]]`.
        // Both name a path; the second names it once per element, and every element's
        // comment lands on the same path, last one winning, because the constants an
        // array of tables becomes carry one name.
        if let Some((section_path, header_end)) = section_header(trimmed) {
            current_path = section_path
                .split('.')
                .map(|segment| segment.trim().trim_matches('"').to_string())
                .collect();
            let section_str = current_path.join(".");

            let mut all_comments = current_comments.clone();
            if let Some(inline) = extract_inline_comment(trimmed, header_end) {
                all_comments.push(inline);
            }
            if !all_comments.is_empty() {
                comments.insert(section_str, all_comments.join("\n"));
            }
            current_comments.clear();
            continue;
        }

        // comments
        if let Some(body) = trimmed.strip_prefix('#') {
            let comment_text = body.trim();

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
                    full_path.push(seg.trim().trim_matches('"').to_string());
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

/// The path a header line names and where the header ends, or `None` for a line that is
/// not one.
///
/// `[[a.b]]` is an array-of-tables header and names `a.b`; `[a.b]` names the same. A line
/// opening a bracket and never closing it is not a header.
fn section_header(line: &str) -> Option<(&str, usize)> {
    if let Some(rest) = line.strip_prefix("[[") {
        let end = rest.find("]]")?;
        return Some((&rest[..end], 2 + end + 2));
    }
    let rest = line.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some((&rest[..end], 1 + end + 1))
}

/// Where a comment starts on a line, ignoring any `#` inside a quoted string.
///
/// `key = "a # b" # the comment` has two hashes and only the second opens a comment. A
/// basic string may carry a backslash escape, so a `\"` inside one does not close it; a
/// literal string has no escapes at all.
fn comment_start(line: &str) -> Option<usize> {
    let mut in_basic = false;
    let mut in_literal = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if in_basic {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_basic = false;
            }
        } else if in_literal {
            if ch == '\'' {
                in_literal = false;
            }
        } else {
            match ch {
                '"' => in_basic = true,
                '\'' => in_literal = true,
                '#' => return Some(index),
                _ => {},
            }
        }
    }
    None
}

/// The comment on a line after the position the key or header ended, if any.
fn extract_inline_comment(line: &str, after_pos: usize) -> Option<String> {
    let comment_pos = comment_start(line)?;
    if comment_pos < after_pos {
        return None;
    }
    let comment = line[comment_pos + 1..].trim();
    if comment.is_empty() {
        None
    } else {
        Some(comment.to_string())
    }
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
    fn a_hash_inside_a_quoted_value_is_not_a_comment() {
        // Two hashes on the delimiter, because the toml itself holds a `"#`.
        let toml = r##"
url = "https://example.test/#fragment" # the real comment
literal = 'a # b'
escaped = "say \"#\" here" # after the escape
plain = 1 # one
"##;
        let comments = extract_comments(toml);
        assert_eq!(comments.get("url"), Some(&"the real comment".to_string()));
        assert!(
            !comments.contains_key("literal"),
            "the hash inside a literal string opened a comment: {comments:?}"
        );
        assert_eq!(
            comments.get("escaped"),
            Some(&"after the escape".to_string())
        );
        assert_eq!(comments.get("plain"), Some(&"one".to_string()));
    }

    #[test]
    fn an_array_of_tables_header_names_the_table_path() {
        // `[[servers]]` used to read as a header whose path was `[servers`, with the
        // opening bracket kept and the closing one lost, so no comment ever reached the
        // path a consumer could name.
        let toml = r#"
# the fleet
[[servers]] # inline on the header
# where it listens
host = "a"

[[servers]]
host = "b"
"#;
        let comments = extract_comments(toml);
        assert_eq!(
            comments.get("servers"),
            Some(&"the fleet\ninline on the header".to_string())
        );
        assert_eq!(
            comments.get("servers.host"),
            Some(&"where it listens".to_string())
        );
        assert!(
            !comments.keys().any(|k| k.contains('[')),
            "a bracket survived into a path: {comments:?}"
        );
    }

    #[test]
    fn a_quoted_key_is_recorded_without_its_quotes() {
        // The fields are looked up by their normalised path, which has no quotes in it.
        let toml = r#"
["special-chars"]
# a dashed key
"with-dash" = 1
"#;
        let comments = extract_comments(toml);
        assert_eq!(
            comments.get("special-chars.with-dash"),
            Some(&"a dashed key".to_string())
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
