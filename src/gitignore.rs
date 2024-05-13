use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet};
use miette::{IntoDiagnostic, Result, WrapErr};

use crate::logger::Logger;

pub struct GitIgnore {
    pub path: PathBuf,
    excluded_paths: GlobSet,
    // Note that includes take precedence over excludes
    included_paths: GlobSet,
}

impl GitIgnore {
    /// Constructs a new `GitIgnore` from a given directory.
    ///
    /// This function looks for a .gitignore file in the given directory and constructs
    /// a new `GitIgnore` instance that can be used to check if paths are excluded or not.
    ///
    /// # Arguments
    ///
    /// * `dir` - A `Path` representing the directory to search for a .gitignore file.
    ///
    /// # Returns
    ///
    /// A new `GitIgnore` instance if a .gitignore file is found in the directory, otherwise `None`.
    pub fn from_dir(dir: &Path) -> Result<Option<Self>> {
        Logger::debug(format!("Checking 🕵️‍♂️ for .gitignore file in directory: {:?}", dir).as_str());

        let gitignore_path = dir.join(".gitignore");
        if gitignore_path.exists() {
            Ok(Some(Self::from_path(gitignore_path)?))
        } else {
            Ok(None)
        }
    }

    /// Fix weird behavior with patterns that start with `/` or `.`
    fn maybe_pattern_to_variants(pattern: &str) -> Vec<String> {
        let mut variants = Vec::new();
        if pattern.starts_with("/") {
            variants.push(format!(".{}", pattern));
        }
        if pattern.starts_with(".") {
            variants.push(format!("./{}", pattern));
        }
        variants.push(pattern.to_string());
        variants
    }

    /// Constructs a new `GitIgnore` from a given .gitignore file path.
    ///
    /// This function reads the .gitignore file specified by the `path` and constructs
    /// a new `GitIgnore` instance that can be used to check if paths are excluded or not.
    ///
    /// # Arguments
    ///
    /// * `path` - A `PathBuf` representing the path to the .gitignore file.
    ///
    /// # Panics
    ///
    /// This function panics if the .gitignore file cannot be read or if it fails to
    /// build the glob sets. Glob patterns that are invalid will be logged as errors
    /// and skipped.
    pub fn from_path(path: PathBuf) -> Result<Self> {
        Logger::debug(format!("Reading 📖 .gitignore file at path: {:?}", path).as_str());

        let file = std::fs::read_to_string(&path).into_diagnostic()
            .wrap_err(format!(
                "Failed to read .gitignore at path: {:?} 😢. It may not be a valid UTF-8 file, or I messed up somewhere 🫢.",
                path
            ))?;

        let mut excluded_paths = GlobSet::builder();
        let mut included_paths = GlobSet::builder();
        for line in file.lines() {
            Logger::trace(format!("Parsing line in .gitignore: {:?}", line).as_str());

            if line.starts_with("#") {
                Logger::trace("Skipping line because it is a comment");

                continue;
            }
            if line.starts_with("!") {
                Logger::trace("Starting with !, parsing as included pattern");

                let included_pattern = line.trim_start_matches('!');
                let variants = Self::maybe_pattern_to_variants(included_pattern);
                for variant in variants {
                    let included_glob = Glob::new(&variant).into_diagnostic()
                        .wrap_err(format!(
                            "Failed to parse include pattern in .gitignore at {:?} 😢. The pattern {:?} will be ignored 🙅‍♀️.",
                            path, line
                        ));
                    match included_glob {
                        Ok(g) => {
                            included_paths.add(g);
                        }
                        Err(e) => {
                            Logger::error(e.to_string().as_str());
                        }
                    }
                }
            } else {
                Logger::trace("Parsing as excluded pattern");

                let variants = Self::maybe_pattern_to_variants(line);
                for variant in variants {
                    let excluded_glob = Glob::new(&variant).into_diagnostic()
                        .wrap_err(format!(
                            "Failed to parse exclude pattern in .gitignore at {:?} 😢. The pattern {:?} will be ignored 🙅‍♀️.",
                            path, line
                        ));
                    match excluded_glob {
                        Ok(g) => {
                            excluded_paths.add(g);
                        }
                        Err(e) => {
                            Logger::error(e.to_string().as_str());
                        }
                    }
                }
            }
        }
        Logger::trace("Finished parsing .gitignore file");

        Logger::debug("Building glob sets for excluded and included paths");

        let excluded_paths = excluded_paths.build().into_diagnostic().wrap_err(format!(
            "Failed to build glob set for excluded paths from .gitignore at path: {:?} 😢.",
            path
        ))?;
        let included_paths = included_paths.build().into_diagnostic().wrap_err(format!(
            "Failed to build glob set for included paths from .gitignore at path: {:?} 😢.",
            path
        ))?;

        Logger::debug("Finished building glob sets for excluded and included paths");

        Ok(Self {
            path,
            excluded_paths,
            included_paths,
        })
    }

    /// Determines if a given path is excluded by the .gitignore rules.
    ///
    /// This method checks if the specified `path` matches any of the exclude patterns
    /// and does not match any of the include patterns in the .gitignore file.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `Path` that will be checked against the .gitignore rules.
    ///
    /// # Returns
    ///
    /// Returns `true` if the path is excluded by the .gitignore rules, otherwise `false`.
    pub fn is_excluded(&self, path: &Path) -> bool {
        let t = self.excluded_paths.is_match(path) && !self.included_paths.is_match(path);
        Logger::trace(
            format!(
                "Checked if path {:?} is excluded in this .gitignore: {}",
                path, t
            )
            .as_str(),
        );
        t
    }
}
