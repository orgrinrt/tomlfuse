//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use std::path::PathBuf;
use tomlfuse::package;
// creates compile-time constants from the closest Cargo.toml file
package! {
    [package]
    package.*
    !package.metadata.*

    [deps]
    dependencies.*
    alias syn_dep = dependencies.syn

    [metadata]
    package.metadata.*
    !package.metadata.defaults.*

    [defaults]
    package.metadata.defaults.*
    
    [all]
    *
}

#[test]
fn test_generated_constants() {
    // check basic lookups work
    // read from ../Cargo.toml for up-to-date values and 1:1 parity
    let cargo_toml_path = PathBuf::from("Cargo.toml");
    let cargo_toml_content =
        std::fs::read_to_string(cargo_toml_path).expect("Failed to read Cargo.toml");
    let cargo_data: toml::Table =
        toml::from_str(&cargo_toml_content).expect("Failed to parse Cargo.toml");
    let pkg = cargo_data
        .get("package")
        .expect("No [package] section in Cargo.toml")
        .as_table()
        .expect("package is not a table");
    let authors_from_file = pkg
        .get("authors")
        .expect("No authors field in Cargo.toml")
        .as_array()
        .expect("authors is not an array");
    let edition_from_file = pkg
        .get("edition")
        .expect("No edition field in Cargo.toml")
        .as_str()
        .expect("edition is not a string");
    assert_eq!(authors_from_file.len(), package::AUTHORS.len());
    assert_eq!(edition_from_file, package::EDITION);
    assert_eq!(
        package::AUTHORS.join(", "),
        authors_from_file
            .iter()
            .map(|v| v.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(metadata::FOO, "bar");
    assert_eq!(defaults::VALUE, 1);

    // log generated values
    println!("Package authors: {}", package::AUTHORS.join(", "));
    println!("Package edition: {}", package::EDITION);
    println!("Metadata test1: {} (should be \"bar\")", metadata::FOO);
    println!("Metadata test2: {} (should be 1)", defaults::VALUE);
}
