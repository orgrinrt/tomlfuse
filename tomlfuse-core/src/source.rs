//------------------------------------------------------------------------------
// Copyright (c) 2025                 orgrinrt           orgrinrt@ikiuni.dev
//                                    Hiisi Digital Oy   contact@hiisi.digital
// SPDX-License-Identifier: MPL-2.0    O. R. Toimela      N2963@student.jamk.fi
//------------------------------------------------------------------------------

use crate::comments::extract_comments;
use crate::error::Error;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs, io};
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
            Err(cause) => Err(Error::Unreadable {
                given: path.to_string_lossy().into_owned(),
                tried: vec![path.to_path_buf()],
                cause: Some(cause),
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

        let mut cause = None;
        for candidate in &candidates {
            match fs::read_to_string(candidate) {
                Ok(contents) => {
                    // Absolute, because the generated `include_bytes!` resolves relative to
                    // the file the macro was invoked in, not to the crate root. A relative
                    // path therefore doubled up: `tests/test.toml` became
                    // `tests/tests/test.toml`.
                    let absolute =
                        fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
                    return Source::parse(toml_path, absolute, &contents);
                },
                // A file that is not there is what the list of places says already. One that
                // is there and refused is not, and is worth keeping.
                Err(e) if e.kind() == io::ErrorKind::NotFound => {},
                Err(e) => cause = cause.or(Some(e)),
            }
        }

        Err(Error::Unreadable {
            given: toml_path.to_string(),
            tried: candidates,
            cause,
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
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Held by every test that sets `CARGO_MANIFEST_DIR`. Tests of one binary share a
    /// process and so an environment, and two of them setting it at once read each other's.
    static ENV: Mutex<()> = Mutex::new(());

    /// Runs `body` with `CARGO_MANIFEST_DIR` set to `dir`, or removed when there is none,
    /// and puts back what was there.
    fn with_manifest_dir<T>(dir: Option<&Path>, body: impl FnOnce() -> T) -> T {
        let _held = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = env::var_os("CARGO_MANIFEST_DIR");
        match dir {
            Some(dir) => env::set_var("CARGO_MANIFEST_DIR", dir),
            None => env::remove_var("CARGO_MANIFEST_DIR"),
        }
        let out = body();
        match before {
            Some(before) => env::set_var("CARGO_MANIFEST_DIR", before),
            None => env::remove_var("CARGO_MANIFEST_DIR"),
        }
        out
    }

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

        // should find workspace root from the project
        let found = with_manifest_dir(Some(&project), find_workspace_root);
        assert_eq!(found, ws_root);

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

    /// A workspace with one member, and a `name.toml` in each of the places `locate` looks.
    struct Layout {
        _temp: TempDir,
        workspace: PathBuf,
        member: PathBuf,
    }

    fn layout() -> Layout {
        let temp = TempDir::new().unwrap();
        let workspace = fs::canonicalize(temp.path()).unwrap().join("ws");
        let member = workspace.join("member");
        fs::create_dir_all(&member).unwrap();
        fs::write(
            workspace.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]",
        )
        .unwrap();
        fs::write(member.join("Cargo.toml"), "[package]\nname = \"member\"").unwrap();
        Layout {
            _temp: temp,
            workspace,
            member,
        }
    }

    fn from_of(source: &Source) -> String {
        source
            .value()
            .get("from")
            .and_then(Value::as_str)
            .unwrap()
            .to_string()
    }

    #[test]
    fn locate_tries_the_crate_before_the_workspace_and_reports_both_in_that_order() {
        let l = layout();
        fs::write(l.workspace.join("a.toml"), "from = \"workspace\"").unwrap();

        // Only the workspace has it, so that is where it is found.
        let found = with_manifest_dir(Some(&l.member), || Source::locate("a.toml")).unwrap();
        assert_eq!(from_of(&found), "workspace");

        // Both have it, and the crate's wins.
        fs::write(l.member.join("a.toml"), "from = \"crate\"").unwrap();
        let found = with_manifest_dir(Some(&l.member), || Source::locate("a.toml")).unwrap();
        assert_eq!(from_of(&found), "crate");

        // Neither, and the error lists the two in the order they were tried.
        let err = with_manifest_dir(Some(&l.member), || Source::locate("nope.toml")).unwrap_err();
        let Error::Unreadable {
            given,
            tried,
            ..
        } = &err
        else {
            panic!("{err}")
        };
        assert_eq!(given, "nope.toml");
        assert_eq!(
            tried,
            &[l.member.join("nope.toml"), l.workspace.join("nope.toml")]
        );
    }

    #[test]
    fn locate_gives_the_absolute_path_it_found_the_file_at() {
        let l = layout();
        fs::write(l.member.join("a.toml"), "from = \"crate\"").unwrap();

        let found = with_manifest_dir(Some(&l.member), || Source::locate("a.toml")).unwrap();

        // The generated `include_bytes!` resolves relative paths against the invoking file,
        // so a relative path here is a bug.
        assert!(found.path().is_absolute(), "{}", found.path().display());
        assert_eq!(found.path(), l.member.join("a.toml"));
    }

    #[test]
    fn locate_takes_an_absolute_path_as_given_and_tries_nothing_else() {
        let l = layout();
        let elsewhere = TempDir::new().unwrap();
        let file = elsewhere.path().join("b.toml");
        fs::write(&file, "from = \"elsewhere\"").unwrap();
        // The same name beside the crate, which a relative lookup would have found instead.
        fs::write(l.member.join("b.toml"), "from = \"crate\"").unwrap();

        let given = file.to_string_lossy().into_owned();
        let found = with_manifest_dir(Some(&l.member), || Source::locate(&given)).unwrap();
        assert_eq!(from_of(&found), "elsewhere");

        let missing = elsewhere.path().join("missing.toml");
        let given = missing.to_string_lossy().into_owned();
        let err = with_manifest_dir(Some(&l.member), || Source::locate(&given)).unwrap_err();
        assert!(
            matches!(&err, Error::Unreadable { tried, .. } if tried == &[missing]),
            "{err}"
        );
    }

    #[test]
    fn locate_names_a_place_once_when_the_crate_is_its_own_workspace() {
        let temp = TempDir::new().unwrap();
        let dir = fs::canonicalize(temp.path()).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"alone\"").unwrap();

        let err = with_manifest_dir(Some(&dir), || Source::locate("nope.toml")).unwrap_err();

        let Error::Unreadable {
            tried,
            ..
        } = &err
        else {
            panic!("{err}")
        };
        assert_eq!(tried, &[dir.join("nope.toml")]);
        assert_eq!(err.to_string().matches("nope.toml").count(), 2, "{err}");
    }

    #[test]
    fn locate_outside_cargo_says_which_variable_is_missing() {
        let err = with_manifest_dir(None, || Source::locate("a.toml")).unwrap_err();

        assert!(
            matches!(
                err,
                Error::NotUnderCargo {
                    variable: "CARGO_MANIFEST_DIR"
                }
            ),
            "{err}"
        );
        assert_eq!(
            err.to_string(),
            "tomlfuse: `CARGO_MANIFEST_DIR` is not set, so a path cannot be taken relative to \
             the crate. Run under cargo, or read the file by its full path."
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_keeps_the_reason_the_system_gave() {
        let temp = TempDir::new().unwrap();
        // A directory is there and cannot be read as a file, which is not the same thing as
        // being absent.
        let err = Source::read(temp.path()).unwrap_err();
        let reason = fs::read_to_string(temp.path()).unwrap_err();

        assert!(err.to_string().contains(&reason.to_string()), "{err}");
        let source = std::error::Error::source(&err).expect("the io error is the source");
        assert_eq!(source.to_string(), reason.to_string());
    }
}
