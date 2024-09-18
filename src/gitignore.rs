use std::path::{Path, PathBuf};

use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    Match,
};

use crate::{
    error::{CunwError, Result},
    logger::Logger,
};

/// Represents a `.gitignore` file and provides methods to check if paths are excluded.
///
/// This struct encapsulates the logic for parsing and applying gitignore rules
/// using the [`ignore`] crate.
#[derive(Debug, Clone)]
pub struct GitIgnore {
    pub path: PathBuf,
    root: PathBuf,
    gitignore: Gitignore,
}

impl GitIgnore {
    /// Creates a [`GitignoreBuilder`] and determines the root directory for a given path.
    ///
    /// **Arguments**
    ///
    /// * `path` - A reference to a [`Path`] that points to either a directory containing
    ///            a `.gitignore` file or directly to a `.gitignore` file.
    ///
    /// **Returns**
    ///
    /// A tuple containing the [`GitignoreBuilder`] and the root [`PathBuf`].
    fn builder_from(path: &Path) -> (GitignoreBuilder, PathBuf) {
        let (root, gitignore) = {
            if path.is_dir() {
                (path, path.join(".gitignore"))
            } else {
                (
                    path.parent().unwrap_or_else(|| Path::new("/")),
                    path.to_path_buf(),
                )
            }
        };
        let mut builder = GitignoreBuilder::new(root);
        builder.add(gitignore);
        (builder, root.to_path_buf())
    }

    /// Creates a new [`GitIgnore`] instance from a given path.
    ///
    /// This method attempts to create a [`GitIgnore`] instance from either a directory
    /// containing a `.gitignore` file or from a direct path to a `.gitignore` file.
    ///
    /// **Arguments**
    ///
    /// * `path` - A reference to a [`Path`] that points to either a directory containing
    ///            a `.gitignore` file or directly to a `.gitignore` file.
    ///
    /// **Returns**
    ///
    /// A [`Result`] containing an [`Option<GitIgnore>`]. Returns [`None`] if no `.gitignore`
    /// file is found or if the path doesn't exist.
    pub fn from(path: &Path) -> Result<Option<Self>> {
        if path.is_dir() {
            let gitignore_path = path.join(".gitignore");
            if !gitignore_path.exists() {
                return Ok(None);
            }
        } else if !path.exists() {
            return Ok(None);
        }

        let (builder, root) = Self::builder_from(path);
        let gitignore = builder
            .build()
            .map_err(|err| CunwError::new(err.into()).with_file(path.to_path_buf()))?;

        Logger::debug(&format!("Created GitIgnore from path: {:?}", path));
        Logger::debug(&format!("Root directory: {:?}", root));

        Ok(Some(Self {
            gitignore,
            path: path.to_path_buf(),
            root,
        }))
    }

    /// Checks if a given path should be excluded based on the gitignore rules.
    ///
    /// This method determines whether a path should be ignored according to the
    /// rules specified in the `.gitignore` file. It handles both absolute and relative
    /// paths, converting them to be relative to the gitignore root as needed.
    ///
    /// **Arguments**
    ///
    /// * `path` - A reference to a [`Path`] to check against the gitignore rules.
    ///
    /// **Returns**
    ///
    /// A boolean indicating whether the path should be excluded (`true`) or not (`false`).
    pub fn is_excluded(&self, path: &Path) -> bool {
        let relative_path = if path.is_absolute() {
            path.strip_prefix(&self.root).unwrap_or(path)
        } else {
            path
        };

        Logger::debug(&format!(
            "Checking if path is excluded: {:?}",
            relative_path
        ));

        let match_result = self
            .gitignore
            .matched_path_or_any_parents(relative_path, false);

        match match_result {
            Match::None => {
                Logger::debug("Path is not excluded (no match)");
                false
            }
            Match::Ignore(_) => {
                Logger::debug("Path is excluded (ignore match)");
                true
            }
            Match::Whitelist(_) => {
                Logger::debug("Path is not excluded (whitelist match)");
                false
            }
        }
    }
}

impl PartialEq for GitIgnore {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper function to create a `.gitignore` file with specified content in a temporary directory.
    ///
    /// **Arguments**
    ///
    /// * `dir` - A reference to a [`TempDir`] where the `.gitignore` file will be created.
    /// * `content` - A string slice containing the content to be written to the `.gitignore` file.
    ///
    /// **Returns**
    ///
    /// A [`PathBuf`] pointing to the created `.gitignore` file.
    fn create_gitignore(dir: &TempDir, content: &str) -> PathBuf {
        let gitignore_path = dir.path().join(".gitignore");
        let mut file = File::create(&gitignore_path).unwrap();
        writeln!(file, "{}", content).unwrap();
        gitignore_path
    }

    #[test]
    fn test_gitignore_from_dir() {
        let dir = TempDir::new().unwrap();
        create_gitignore(&dir, "*.txt\n!important.txt");

        let gitignore = GitIgnore::from(dir.path()).unwrap().unwrap();
        assert!(gitignore.is_excluded(Path::new("file.txt")));
        assert!(!gitignore.is_excluded(Path::new("important.txt")));
        assert!(!gitignore.is_excluded(Path::new("file.rs")));
    }

    #[test]
    fn test_gitignore_from_path() {
        let dir = TempDir::new().unwrap();
        let gitignore_path = create_gitignore(&dir, "*.log\ntemp/\n!temp/keep.txt");

        let gitignore = GitIgnore::from(&gitignore_path).unwrap().unwrap();
        assert!(gitignore.is_excluded(Path::new("error.log")));
        assert!(gitignore.is_excluded(Path::new("temp/file.txt")));
        assert!(!gitignore.is_excluded(Path::new("temp/keep.txt")));
        assert!(!gitignore.is_excluded(Path::new("src/main.rs")));
    }

    #[test]
    fn test_gitignore_patterns() {
        let dir = TempDir::new().unwrap();
        let gitignore_path = create_gitignore(&dir, "/root.txt\n/src/*.rs\n!/src/main.rs");

        let gitignore = GitIgnore::from(&gitignore_path).unwrap().unwrap();
        assert!(gitignore.is_excluded(Path::new("root.txt")));
        assert!(gitignore.is_excluded(Path::new("src/lib.rs")));
        assert!(!gitignore.is_excluded(Path::new("src/main.rs")));
        assert!(!gitignore.is_excluded(Path::new("doc/root.txt")));
    }
}
