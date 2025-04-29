//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use tomlfuse::file;

// generate constants from test.toml
file! {
    "tests/test.toml"
    
    [main]
    section.*
    
    // test hierarchies with glob patterns
    [config_vals]
    config.*          // should keep settings.timeout as settings::TIMEOUT
    nested.inner.*    // should flatten these as VALUE and STRING

    // test deep hierarchies
    [deep_stuff]
    deep.*
    !deep.level1.alternative.*  // exclude a branch

    // test mixed sources in one module
    [mixed]
    deep.standalone
    mixed-types.*
    special-chars.*

    // test direct paths vs globbed paths
    [direct]
    deep.level1.level2.level3.value
    deep.level1.level2.other

    // test duplicated key name at different levels
    [dupes]
    duplicates.*

    // test aliases
    [renamed]
    alias renamed_key = section.key
    alias short_path = deep.level1.level2.level3.value
    alias clean_name = special-chars.with-dash

    // original test case
    [original]
    config.*
    nested.inner.*
}

#[test]
fn test_generated_file_constants() {
    // verify generated constants match test data
    // basic section
    assert_eq!(main::KEY, "value");
    assert_eq!(main::NUMBER, 42);
    assert_eq!(main::ARRAY.len(), 3);
    assert_eq!(main::ARRAY[0], "item1");

    // config section with hierarchy preserved
    assert!(!config_vals::DEBUG); // should break if the type is not properly parsed as bool
    assert_eq!(config_vals::settings::TIMEOUT, 500);
    assert!(config_vals::VALUE); // should break if the type is not properly parsed as bool
    assert_eq!(config_vals::STRING, "nested string");
    assert_eq!(config_vals::settings::RETRIES, 3);
    assert_eq!(config_vals::logging::LEVEL, "info");
    assert_eq!(config_vals::logging::FORMAT, "json");

    // nested values flattened in root

    // deep hierarchy tests
    assert!(deep_stuff::level1::level2::level3::VALUE);
    assert_eq!(deep_stuff::level1::level2::OTHER, "sibling");
    assert_eq!(deep_stuff::STANDALONE, "top-level");
    // this should not exist due to negation pattern:
    // deep_stuff::level1::alternative::PATH

    // mixed sources test
    assert_eq!(mixed::STANDALONE, "top-level");
    assert_eq!(mixed::STRING, "text");
    assert_eq!(mixed::NUMBER, 42);
    assert_eq!(mixed::FLOAT, 3.14);
    assert!(mixed::BOOL);
    assert_eq!(mixed::WITH_DASH, "dashed");
    assert_eq!(mixed::WITH_UNDERSCORE, "underscore");
    assert_eq!(mixed::quoted::KEY, "quoted");

    // direct paths
    assert!(direct::VALUE);
    assert_eq!(direct::OTHER, "sibling");

    // duplicate keys at different levels
    assert_eq!(dupes::KEY, "top");
    assert_eq!(dupes::nested::KEY, "middle");
    assert_eq!(dupes::nested::deeper::KEY, "bottom");
    assert_eq!(dupes::FIRST, 1);
    assert_eq!(dupes::SECOND, 2);

    // aliases
    assert_eq!(renamed::RENAMED_KEY, "value");
    assert_eq!(renamed::SHORT_PATH, true);
    assert_eq!(renamed::CLEAN_NAME, "dashed");

    // verify original test case still works
    assert!(!original::DEBUG);
    assert_eq!(original::settings::TIMEOUT, 500);
}
