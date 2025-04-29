//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::collections::HashMap;

pub struct Comment {
    // TODO: 
}

// FIXME: make this robust and not so hack-ish
pub fn extract_comments(content: &str) -> HashMap<String, String> {
    let mut comments = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut current_comments = Vec::new();
    let mut current_path = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // section headers [section.subsection]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_path.clear();

            // parse [section] or [section.subsection] 
            let section_path = &trimmed[1..trimmed.len() - 1];
            current_path = section_path.split('.').map(String::from).collect();

            // reset comments at section start
            current_comments.clear();
            continue;
        }

        // comments
        if trimmed.starts_with('#') {
            current_comments.push(trimmed[1..].trim().to_string());
            continue;
        }

        // key-value pairs
        if let Some(pos) = trimmed.find('=') {
            if !trimmed.is_empty() {
                let key = trimmed[..pos].trim();

                // build full path to this key
                let mut full_path = current_path.clone();
                full_path.push(key.to_string());
                let path_str = full_path.join(".");

                // inline comment if present
                let mut key_comments = current_comments.clone();
                if let Some(comment_pos) = trimmed.find('#') {
                    if comment_pos > pos {  // comment after the = sign
                        let inline_comment = trimmed[comment_pos + 1..].trim();
                        if !inline_comment.is_empty() {
                            key_comments.push(inline_comment.to_string());
                        }
                    }
                }

                // add comments if we have any
                if !key_comments.is_empty() {
                    comments.insert(path_str, key_comments.join("\n"));
                }

                // reset comment accumulator
                current_comments.clear();
            }
        }
    }

    comments
}
