//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::comments::extract_comments;
use crate::error::Error;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value;

/// A toml file that has been found, read and parsed, with the comments that sit above its keys.
///
/// This is the one thing every binding starts from. It holds no opinion about what is to be
/// bound out of it, so one source serves any number of [`Section`](crate::Section)s.
#[derive(Clone, Debug)]
pub struct Source {
    path: PathBuf,
    value: Value,
    comments: HashMap<String, String>,
}

impl Source {
    /// Reads the file at exactly `path`, which is taken as given.
    ///
    /// This is the way in for a tool or a build script that knows where its file is. The
    /// macros use [`Source::locate`], which searches the way a path written in source is
    /// expected to.
    pub fn read(path: &Path) -> Result<Source, Error> {
        match fs::read_to_string(path) {
            Ok(text) => Source::parse(&path.to_string_lossy(), path.to_path_buf(), &text),
            Err(_) => Err(Error::Unreadable {
                given: path.to_string_lossy().into_owned(),
                tried: vec![path.to_path_buf()],
            }),
        }
    }

    /// Finds the file the way a path written in a crate's source is expected to resolve, and
    /// reads it.
    ///
    /// Three places are tried, in this order: the path as given, then relative to the
    /// workspace root, then relative to `CARGO_MANIFEST_DIR`. Which one succeeded matters,
    /// because the generated code registers it with cargo so that editing the file rebuilds
    /// what was generated from it, and that needs the path it was actually found at.
    ///
    /// The candidates used to be the three arms of nested `unwrap_or` calls, which evaluate
    /// their argument whether or not it is needed, so every module read the file from all
    /// three places every time. `or_else` reads one.
    ///
    /// A path matching none of them used to become `unwrap_or_default()`, an empty string,
    /// which parses as an empty toml and generates an empty module. A mistyped path was
    /// therefore silent.
    pub fn locate(toml_path: &str) -> Result<Source, Error> {
        let manifest_dir =
            PathBuf::from(
                env::var("CARGO_MANIFEST_DIR").map_err(|_| Error::NotUnderCargo {
                    variable: "CARGO_MANIFEST_DIR",
                })?,
            );

        // Relative to the crate first, then to the workspace. Not relative to the current
        // directory, which this used to try before either: a proc macro runs inside the compiler
        // and the current directory is wherever cargo happened to be invoked. That worked by
        // coincidence, because the default target directory sits inside the crate and
        // `find_workspace_root` climbs back out of it, and it stopped the moment
        // `CARGO_TARGET_DIR` pointed anywhere else, which CI routinely does.
        //
        // An absolute path is taken as given, since neither join would change it.
        let given = PathBuf::from(toml_path);
        let mut candidates: Vec<PathBuf> = if given.is_absolute() {
            vec![given]
        } else {
            vec![manifest_dir.join(toml_path), find_workspace_root().join(toml_path)]
        };

        // `find_workspace_root` falls back to the manifest directory, so for a crate that is not
        // in a workspace the two are the same path. Printing it twice under prose promising two
        // different strategies is what the old message did.
        candidates.dedup();

        for candidate in &candidates {
            if let Ok(contents) = fs::read_to_string(candidate) {
                // Absolute, because the generated `include_bytes!` resolves relative to the
                // file the macro was invoked in, not to the crate root. A relative path
                // therefore doubled up: `tests/test.toml` became `tests/tests/test.toml`.
                let absolute = fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
                return Source::parse(toml_path, absolute, &contents);
            }
        }

        Err(Error::Unreadable {
            given: toml_path.to_string(),
            tried: candidates,
        })
    }

    /// Parses `text` as the contents of the file at `path`.
    ///
    /// `given` is the path the way the caller wrote it, which is what an error should name.
    pub fn parse(given: &str, path: PathBuf, text: &str) -> Result<Source, Error> {
        let value: Value = text.parse().map_err(|source| Error::Unparseable {
            given: given.to_string(),
            source,
        })?;
        Ok(Source {
            path,
            value,
            comments: extract_comments(text),
        })
    }

    /// Where the file was found. Absolute when it came from [`Source::locate`].
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The parsed document.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// The comments above each key, by the key's full dotted path.
    pub fn comments(&self) -> &HashMap<String, String> {
        &self.comments
    }
}

/// Finds the workspace root by traversing upward from `CARGO_MANIFEST_DIR`.
///
/// Searches parent directories until it finds one with a Cargo.toml file
/// that contains a `[workspace]` table. This allows finding the workspace
/// root from any crate within the workspace.
///
/// # Returns
/// Falls back to the original manifest directory if no workspace root is found.
///
/// # Panics
/// When `CARGO_MANIFEST_DIR` is not set, which is to say outside cargo.
#[cold]
pub fn find_workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(manifest_dir);

    // search upwards for workspace root
    let mut path = manifest_path.clone();

    while !is_workspace_root(&path.join("Cargo.toml")) {
        if !path.pop() {
            // fallback to pkg dir if no workspace found
            return manifest_path;
        }
    }
    path
}

/// Determines if a path contains a workspace Cargo.toml file.
///
/// Checks if the file exists, can be read as TOML, and contains
/// a `[workspace]` table.
/// # Parameters
/// - `path`: Path to a potential Cargo.toml file
///
/// # Returns
/// `true` if the path exists, can be read, parsed as TOML, and has a workspace table.
#[cold]
pub fn is_workspace_root(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(toml) = content.parse::<Value>() {
            return toml.get("workspace").is_some();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_workspace_root_setup() -> Result<(), Box<dyn std::error::Error>> {
        // create a temporary directory structure
        let temp = TempDir::new()?;
        let root = temp.path().to_path_buf();

        // create workspace structure
        let ws_root = root.join("workspace");
        let project = ws_root.join("project");
        fs::create_dir_all(&project)?;

        // create workspace Cargo.toml
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"project\"]",
        )?;

        // create project Cargo.toml
        fs::write(project.join("Cargo.toml"), "[package]\nname = \"test\"")?;

        // test with original manifest dir set to project
        let orig_dir = env::var("CARGO_MANIFEST_DIR").ok();
        env::set_var("CARGO_MANIFEST_DIR", project.to_string_lossy().to_string());

        // should find workspace root
        let found = find_workspace_root();
        assert_eq!(found, ws_root);

        // restore original env var
        if let Some(dir) = orig_dir {
            env::set_var("CARGO_MANIFEST_DIR", dir);
        }

        Ok(())
    }

    #[test]
    fn test_is_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let ws_toml = temp.path().join("Cargo.toml");
        let pkg_toml = temp.path().join("pkg").join("Cargo.toml");

        fs::create_dir_all(temp.path().join("pkg"))?;
        fs::write(&ws_toml, "[workspace]\nmembers = [\"pkg\"]")?;
        fs::write(&pkg_toml, "[package]\nname = \"pkg\"")?;

        assert!(is_workspace_root(&ws_toml));
        assert!(!is_workspace_root(&pkg_toml));
        assert!(!is_workspace_root(&temp.path().join("nonexistent.toml")));

        Ok(())
    }

    #[test]
    fn a_file_read_by_its_path_carries_its_comments() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let file = temp.path().join("a.toml");
        fs::write(&file, "# the answer\nvalue = 42\n")?;

        let source = Source::read(&file)?;

        assert_eq!(source.path(), file);
        assert_eq!(
            source.value().get("value").and_then(Value::as_integer),
            Some(42)
        );
        assert_eq!(
            source.comments().get("value"),
            Some(&"the answer".to_string())
        );
        Ok(())
    }

    #[test]
    fn a_syntax_error_carries_the_parsers_line_and_column() {
        let err = Source::parse("bad.toml", "bad.toml".into(), "ok = 1\nbad = = 2\n").unwrap_err();
        let message = err.to_string();

        assert!(
            message.contains("bad.toml"),
            "the message does not name the file:\n{message}"
        );
        assert!(
            message.contains("line 2, column 7"),
            "the message drops the position the parser reported:\n{message}"
        );
        assert!(
            message.contains("bad = = 2"),
            "the message drops the line the parser pointed at:\n{message}"
        );
    }

    #[test]
    fn a_file_that_is_not_there_says_where_it_looked() {
        let err = Source::read(Path::new("/nowhere/at/all.toml")).unwrap_err();
        assert!(
            matches!(&err, Error::Unreadable { tried, .. } if tried == &[PathBuf::from("/nowhere/at/all.toml")]),
            "{err}"
        );
    }
}
