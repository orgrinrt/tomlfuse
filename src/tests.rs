//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    use crate::comments::extract_comments;
    use crate::field::resolve_field_paths;
    use crate::pattern::PatternString;
    use crate::utils::convert_value_to_tokens;
    use crate::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_meta_config() {
        // basic section parsing
        let config: MacroInput = parse_quote! {
            [package]
            version
            name
            !private_field
        };

        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].name, "package");
        assert_eq!(config.sections[0].includes.len(), 2);
        assert_eq!(config.sections[0].includes[0].path, "version");
        assert_eq!(config.sections[0].includes[1].path, "name");
        assert_eq!(config.sections[0].excludes.len(), 1);
        assert_eq!(config.sections[0].excludes[0].path, "private_field");
    }

    #[test]
    fn test_parse_aliases() {
        // alias definition
        let config: MacroInput = parse_quote! {
            [deps]
            dependencies.*
            alias dependencies.serde = serde_dep
        };

        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].aliases.len(), 1);
        assert_eq!(
            config.sections[0].aliases.get("dependencies.serde"),
            Some(&"serde_dep".to_string())
        );
    }

    #[test]
    fn test_parse_multiple_sections() {
        // parsing multiple sections
        let config: MacroInput = parse_quote! {
            [package]
            version
            name
            
            [deps]
            dependencies.*
        };

        assert_eq!(config.sections.len(), 2);
        assert_eq!(config.sections[0].name, "package");
        assert_eq!(config.sections[1].name, "deps");
    }

    #[test]
    fn test_string_literal_paths() {
        // string literal path parsing
        let config: MacroInput = parse_quote! {
            [test]
            "path.with.dots"
            !"excluded.path"
        };

        assert_eq!(config.sections[0].includes[0].path, "path.with.dots");
        assert_eq!(config.sections[0].excludes[0].path, "excluded.path");
    }

    #[test]
    fn test_extract_comments() {
        let toml_content = r#"
# Header comment
[package]
# Version comment
version = "0.1.0" # inline comment

# Name comment
# Second line
name = "test-crate"
        "#;

        let comments = extract_comments(toml_content);

        assert_eq!(comments.get("package.version"), Some(&"Version comment\ninline comment".to_string()));
        assert_eq!(comments.get("package.name"), Some(&"Name comment\nSecond line".to_string()));
    }

    #[test]
    fn test_glob_pattern_matching() {
        // glob pattern matching test
        let pattern = PatternString {
            path: "dependencies.*".to_string(),
            is_glob: true,
        };

        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new(&pattern.path).unwrap());
        let matcher = builder.build().unwrap();

        assert!(matcher.is_match("dependencies.serde"));
        assert!(matcher.is_match("dependencies.tokio"));
        assert!(!matcher.is_match("dev-dependencies.serde"));
    }

    #[test]
    fn test_convert_value_to_tokens() {
        // string conversion
        let str_value = Value::String("test".to_string());
        let (type_tokens, value_tokens) = convert_value_to_tokens(&str_value);
        assert_eq!(type_tokens.to_string(), "& 'static str");
        assert_eq!(value_tokens.to_string(), "\"test\"");

        // integer conversion
        let int_value = Value::Integer(42);
        let (type_tokens, value_tokens) = convert_value_to_tokens(&int_value);
        assert_eq!(type_tokens.to_string(), "i64");
        // numeric value verification instead of string repr
        let value_str = value_tokens.to_string();
        let num_val: i64 = value_str.trim_end_matches("i64").parse().unwrap();
        assert_eq!(num_val, 42);

        // test boolean conversion
        let bool_value = Value::Boolean(true);
        let (type_tokens, value_tokens) = convert_value_to_tokens(&bool_value);
        assert_eq!(type_tokens.to_string(), "bool");
        assert_eq!(value_tokens.to_string(), "true");
    }

    #[test]
    fn test_extract_all_paths() {
        // sample toml structure
        let toml_str = r#"
            [package]
            name = "test-crate"
            version = "0.1.0"
            
            [dependencies]
            serde = "1.0"
            
            [dependencies.tokio]
            version = "1.0"
            features = ["full"]
        "#;

        let toml: Value = toml_str.parse().unwrap();
        let mut paths = Vec::new();
        resolve_field_paths(&toml, "", &mut paths);

        // verify expected paths extraction
        let path_strings: Vec<_> = paths.iter()
            .map(|(path, _)| path.clone())
            .collect();

        assert!(path_strings.contains(&"package".to_string()));
        assert!(path_strings.contains(&"package.name".to_string()));
        assert!(path_strings.contains(&"package.version".to_string()));
        assert!(path_strings.contains(&"dependencies".to_string()));
        assert!(path_strings.contains(&"dependencies.serde".to_string()));
        assert!(path_strings.contains(&"dependencies.tokio".to_string()));
        assert!(path_strings.contains(&"dependencies.tokio.version".to_string()));
        assert!(path_strings.contains(&"dependencies.tokio.features".to_string()));
    }
}
