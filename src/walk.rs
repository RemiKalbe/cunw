use indicatif::{ProgressBar, ProgressStyle};
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use std::{fs, path::PathBuf};

use globset::{Candidate, Glob, GlobSet, GlobSetBuilder};
use ignore_files::{from_origin, IgnoreFile as IgnoreFilesIgnoreFile};
use walkdir::{DirEntry, WalkDir};

use crate::error::{Crate, Error, ErrorContext};
use crate::logger::Logger;

pub struct IgnoreFile {
    /// The path where the .gitignore file applies.
    applies_in: Option<PathBuf>,
    /// The exclude patterns.
    exclude_patterns: Vec<Result<Glob, Error>>,
    /// The include patterns.
    include_patterns: Vec<Result<Glob, Error>>,
}

impl TryFrom<IgnoreFilesIgnoreFile> for IgnoreFile {
    type Error = Error;

    fn try_from(ignore_file: IgnoreFilesIgnoreFile) -> Result<Self, Self::Error> {
        let path = ignore_file.path;
        let path = Rc::new(path);

        Logger::log_message(
            log::Level::Debug,
            "Reading ignore file",
            Some(path.display().to_string().as_str()),
        )
        .unwrap();

        let applies_in = ignore_file.applies_in;
        // Read the .gitignore file
        let mut gitignore_content = String::new();
        {
            let mut gitignore_file = fs::File::open(path.as_path())?;
            gitignore_file.read_to_string(&mut gitignore_content)?;
        }
        // Split the content into lines
        // Filter out comments and empty lines
        // Split the lines into exclude and include patterns
        // Create a Pattern from each line
        // Return the patterns
        let (exclude, include) = gitignore_content
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .fold((Vec::new(), Vec::new()), |(mut exclude, mut include), l| {
                let l = l.to_string();

                Logger::log_message(
                    log::Level::Debug,
                    "Found pattern in ignore file",
                    Some(l.as_str()),
                )
                .unwrap();

                if l.starts_with('!') {
                    Logger::log_message(log::Level::Trace, "Is include pattern", None).unwrap();

                    let l = l.trim_start_matches('!');
                    let g: Result<Glob, Error> = Glob::new(l.as_ref())
                        .map_err(|e| {
                            Error::from(e).with_context(
                                ErrorContext::new(log::Level::Warn)
                                    .with_message_title(
                                        "Found invalid include pattern in ignore file".to_string(),
                                    )
                                    .with_message(format!("'{}' in '{}'", l, path.display()))
                                    .with_from_crate(Crate::IgnoreFiles),
                            )
                        })
                        .map(|g| g);
                    include.push(g);
                } else {
                    Logger::log_message(log::Level::Trace, "Is exclude pattern", None).unwrap();

                    let g: Result<Glob, Error> = Glob::new(l.as_ref())
                        .map_err(|e| {
                            Error::from(e).with_context(
                                ErrorContext::new(log::Level::Warn)
                                    .with_message_title(
                                        "Found invalid exclude pattern in ignore file".to_string(),
                                    )
                                    .with_message(format!("'{}' in '{}'", l, path.display()))
                                    .with_from_crate(Crate::IgnoreFiles),
                            )
                        })
                        .map(|g| g);
                    exclude.push(g);
                }
                (exclude, include)
            });

        Logger::log_message(
            log::Level::Debug,
            "Finished reading ignore file",
            Some(path.display().to_string().as_str()),
        )
        .unwrap();

        Ok(Self {
            applies_in,
            exclude_patterns: exclude,
            include_patterns: include,
        })
    }
}

struct IgnoreFiles<'a> {
    /// The starting path of the walker.
    starting_path: &'a Path,
    /// The ignore files found.
    ignore_files: Vec<Result<IgnoreFile, Error>>,
    /// The patterns to exclude relative to the starting path.
    exclude_patterns: Vec<Glob>,
    /// The patterns to include relative to the starting path.
    include_patterns: Vec<Glob>,
}

impl<'a> IgnoreFiles<'a> {
    pub fn from(starting_path: &'a Path) -> Self {
        Logger::log_message(
            log::Level::Debug,
            "Reading ignore files from path",
            Some(starting_path.display().to_string().as_str()),
        )
        .unwrap();

        let executor = match tokio::runtime::Builder::new_current_thread()
            .thread_name("ignore_files blocking wrapper")
            .build()
        {
            Ok(e) => e,
            Err(e) => {
                let e = Error::from(e).with_context(
                    ErrorContext::new(log::Level::Error)
                        .with_message_title("Failed to create tokio runtime".to_string())
                        .with_from_crate(Crate::Tokio),
                );
                Logger::log_err(e);
                panic!("Failed to create tokio runtime")
            }
        };

        Logger::log_message(
            log::Level::Debug,
            "Created tokio runtime",
            Some("ignore_files blocking wrapper"),
        )
        .unwrap();

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(Duration::from_millis(100));
        progress.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
        );
        progress.set_message("Collecting ignore files; you can disable this behavior with the `--do-not-consider-ignore-files` flag");

        let (ignore_files, errs) = executor.block_on(from_origin(starting_path));
        let ignore_files: Vec<Result<IgnoreFile, Error>> = ignore_files
            .into_iter()
            .map(|f| IgnoreFile::try_from(f))
            .collect();

        // Log any errors that occurred while reading ignore files
        errs.into_iter().for_each(|e| {
            let err = Error::from(e).with_context(
                ErrorContext::new(log::Level::Warn)
                    .with_message_title("Failed to read ignore file".to_string())
                    .with_from_crate(Crate::IgnoreFiles),
            );
            Logger::log_err(err);
        });

        /*let ignore_files_considered: String =
        ignore_files.iter().fold(String::default(), |mut acc, f| {
            if let Ok(f) = f {
                acc.push_str(
                    format!("           - {:?}\n", f.applies_in.as_ref().unwrap()).as_ref(),
                );
            }
            acc
        });*/

        progress.finish_with_message("Finished collecting ignore files; you can disable this behavior with the `--do-not-consider-ignore-files` flag");

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(Duration::from_millis(100));
        progress.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
        );
        progress.set_message("Parsing ignore files");

        // For each ignore file, if a apply_in is Some convert each pattern to a pattern relative to
        // starting_path. If apply_in is None, this means it's a global ignore file and the patterns
        // can be left as is.
        let (exclude_patterns, include_patterns) =
            ignore_files
                .iter()
                .fold((Vec::new(), Vec::new()), |(mut acc_ex, mut acc_in), f| {
                    if let Ok(f) = f {
                        Logger::log_message(
                            log::Level::Debug,
                            "Splitting patterns into exclude and include",
                            None,
                        )
                        .unwrap();

                        f.exclude_patterns
                            .iter()
                            .map(|p| {
                                if let Ok(g) = p {
                                    acc_ex.push(g.clone());

                                    Logger::log_message(
                                        log::Level::Debug,
                                        "Added exclude pattern as is",
                                        Some(g.to_string().as_str()),
                                    )
                                    .unwrap();
                                }
                            })
                            .for_each(drop);
                        f.include_patterns
                            .iter()
                            .map(|p| {
                                if let Ok(g) = p {
                                    acc_in.push(g.clone());

                                    Logger::log_message(
                                        log::Level::Debug,
                                        "Added include pattern as is",
                                        Some(g.to_string().as_str()),
                                    )
                                    .unwrap();
                                }
                            })
                            .for_each(drop);
                        (acc_ex, acc_in)
                    } else {
                        (acc_ex, acc_in)
                    }
                });

        Logger::log_message(log::Level::Debug, "Finished processing ignore files", None).unwrap();

        progress.finish_with_message("Finished parsing ignore files");

        Self {
            starting_path,
            ignore_files,
            exclude_patterns,
            include_patterns,
        }
    }
}

pub struct ArgWalker<'a> {
    /// The starting path for the walker.
    starting_path: &'a Path,
    /// The exclude combiner.
    maybe_exclude_combiner: Option<GlobSet>,
    /// The include combiner.
    maybe_include_combiner: Option<GlobSet>,
    /// The maximum depth to walk.
    max_depth: usize,
    /// Whether to follow symlinks.
    follow_symlinks: bool,
}

#[derive(Debug)]
enum ArgWalkerFilterEntryResult {
    Keep,
    DiscardFile,
    DiscardDir,
}

impl<'a> ArgWalker<'a> {
    pub fn try_from(
        starting_path: &'a Path,
        consider_ignore_files: bool,
        exclude_patterns: Option<Vec<Glob>>,
        include_patterns: Option<Vec<Glob>>,
        max_depth: Option<usize>,
        follow_symlinks: bool,
    ) -> Result<Self, Error> {
        let max_depth = max_depth.unwrap_or(usize::MAX);
        let follow_symlinks = follow_symlinks;

        Logger::log_message(
            log::Level::Debug,
            "\nBuilding ArgWalke, with the following parameters",
            Some(format!("\n- Starting Path: {:#?}\n- Consider Ignore Files: {:#?}\n- Exclude Patterns: {:#?}\n- Include Patterns: {:#?}\n- Max Depth: {:#?}\n- Follow Symlinks: {:#?}",
                starting_path.display().to_string(),
                consider_ignore_files,
                exclude_patterns,
                include_patterns,
                max_depth,
                follow_symlinks
            ).as_str()),
        )
        .unwrap();

        let ignore_files = match consider_ignore_files {
            true => Some(IgnoreFiles::from(starting_path)),
            false => None,
        };

        let exclude_patterns: Vec<Glob> = exclude_patterns.unwrap_or(Vec::new());
        let include_patterns: Vec<Glob> = include_patterns.unwrap_or(Vec::new());

        let ignore_files = ignore_files;

        let include_patterns = include_patterns
            .into_iter()
            .chain(
                ignore_files
                    .as_ref()
                    .map(|f| f.include_patterns.clone().into_iter())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>();

        let exclude_patterns = exclude_patterns
            .into_iter()
            .chain(
                ignore_files
                    .as_ref()
                    .map(|f| f.exclude_patterns.clone().into_iter())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>();

        Logger::log_message(
            log::Level::Debug,
            "Patterns have been combined",
            Some(
                format!(
                    "Include Patterns: {:#?}\nExclude Patterns: {:#?}",
                    include_patterns, exclude_patterns
                )
                .as_str(),
            ),
        )
        .unwrap();

        let maybe_include_combiner = match include_patterns.is_empty() {
            true => None,
            false => {
                let mut builder = GlobSetBuilder::new();
                for g in include_patterns.iter() {
                    builder.add(g.clone());
                }
                Some(builder.build().map_err(|e| Error::from(e))?)
            }
        };
        let maybe_exclude_combiner = match exclude_patterns.is_empty() {
            true => None,
            false => {
                let mut builder = GlobSetBuilder::new();
                for g in exclude_patterns.iter() {
                    builder.add(g.clone());
                }
                Some(builder.build().map_err(|e| Error::from(e))?)
            }
        };

        Logger::log_message(log::Level::Debug, "Finished building ArgWalker", None).unwrap();

        Ok(Self {
            starting_path,
            maybe_exclude_combiner,
            maybe_include_combiner,
            max_depth,
            follow_symlinks,
        })
    }

    fn path_as_relative(&self, path: &Path) -> PathBuf {
        let start_with_dotslash = path.starts_with("./");
        let root_is_dot = self.starting_path == Path::new(".");

        let is_dir = fs::metadata(path).unwrap().is_dir();

        let relative_path = path.strip_prefix(self.starting_path).unwrap();
        let relative_path =
            if is_dir && start_with_dotslash && root_is_dot && !relative_path.starts_with("/") {
                Path::new("/").join(relative_path)
            } else {
                relative_path.to_path_buf()
            };
        relative_path
    }

    fn path_into_candidate(&self, path: &'a Path) -> Candidate<'a> {
        Candidate::new(path)
    }

    fn filter_entry(&self, entry: &DirEntry) -> ArgWalkerFilterEntryResult {
        Logger::log_message(
            log::Level::Debug,
            "Walking through the tree",
            Some(entry.path().to_str().unwrap()),
        )
        .unwrap();

        // Convert the entry path to a relative path
        let relative_path = self.path_as_relative(entry.path());
        let candidate = self.path_into_candidate(&relative_path);

        Logger::log_message(
            log::Level::Trace,
            "Converted entry path to candidate",
            Some(format!("{:#?}", candidate).as_str()),
        )
        .unwrap();

        // Does the entry match any of the exclude patterns?
        if let Some(ref combiner) = self.maybe_exclude_combiner {
            Logger::log_message(
                log::Level::Trace,
                "[maybe_exclude_combiner] Exclude combiner is present",
                None,
            )
            .unwrap();

            if combiner.is_match_candidate(&candidate) {
                Logger::log_message(
                    log::Level::Trace,
                    "[maybe_exclude_combiner -> match] Entry matches at least one exclude pattern",
                    None,
                )
                .unwrap();

                let metadata = entry.metadata().unwrap();
                // BUT is it a file that matches an include pattern?
                if metadata.is_file() {
                    Logger::log_message(
                        log::Level::Trace,
                        "[maybe_exclude_combiner -> match -> is_file] Entry is a file",
                        None,
                    )
                    .unwrap();

                    if let Some(ref combiner) = self.maybe_include_combiner {
                        Logger::log_message(
                            log::Level::Trace,
                            "[maybe_exclude_combiner -> match -> is_file -> maybe_include_combiner] Include combiner is present",
                            None,
                        ).unwrap();

                        if combiner.is_match_candidate(&candidate) {
                            Logger::log_message(
                                log::Level::Debug,
                                "[maybe_exclude_combiner -> match -> is_file -> maybe_include_combiner -> match] Entry matches at least one include pattern",
                                Some("Entry is retained\n"),
                            )
                            .unwrap();

                            return ArgWalkerFilterEntryResult::Keep; // <-- We follow the gitignore pattern
                        }
                    }
                    Logger::log_message(
                        log::Level::Debug,
                        "[maybe_exclude_combiner -> match -> is_file -> maybe_include_combiner -> no_match] Entry does not match any include pattern",
                        Some("Entry is discarded\n"),
                    )
                    .unwrap();

                    return ArgWalkerFilterEntryResult::DiscardFile; // <-- Discard the file.
                }
                Logger::log_message(
                    log::Level::Debug,
                    "[maybe_exclude_combiner -> match -> is_dir] Entry is a directory",
                    Some("Entry (including its children) is discarded\n"),
                )
                .unwrap();

                return ArgWalkerFilterEntryResult::DiscardDir; // <-- Discard the file and its directory tree.
            }
            Logger::log_message(
                log::Level::Debug,
                "[maybe_exclude_combiner -> no_match] Entry does not match any exclude pattern",
                Some("Entry is retained\n"),
            )
            .unwrap();

            return ArgWalkerFilterEntryResult::Keep; // <-- We follow the gitignore pattern
        };
        // If the execution reaches this point, it means we have no exclude patterns.
        // If we have a include pattern, the entry must match it.
        Logger::log_message(
            log::Level::Trace,
            "No exclude combiner present",
            Some("Checking for include patterns"),
        )
        .unwrap();

        if let Some(ref combiner) = self.maybe_include_combiner {
            Logger::log_message(
                log::Level::Trace,
                "[maybe_include_combiner] Include combiner is present",
                None,
            )
            .unwrap();

            if !combiner.is_match_candidate(&candidate) {
                Logger::log_message(
                    log::Level::Debug,
                    "[maybe_include_combiner -> no_match] Entry does not match any include pattern",
                    Some("Entry is discarded\n"),
                )
                .unwrap();

                return ArgWalkerFilterEntryResult::DiscardFile; // <-- Discard the file.
            }
        }
        // If the execution reaches this point, it means:
        // - We have no exclude patterns and no include patterns.
        // - We have no exclude patterns and the entry matches the include pattern.
        Logger::log_message(
            log::Level::Debug,
            "No include combiner present or entry matches include pattern",
            Some("Entry is retained\n"),
        )
        .unwrap();

        ArgWalkerFilterEntryResult::Keep
    }

    pub fn try_sprint(&self) -> Result<Vec<Result<DirEntry, Error>>, Error> {
        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(Duration::from_millis(200));
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
        );
        progress.set_message("Walking through the directory tree");

        let walker = WalkDir::new(self.starting_path)
            .max_depth(self.max_depth as usize)
            .follow_links(self.follow_symlinks);
        let mut it = walker.into_iter();

        let mut entries = Vec::new();

        loop {
            let entry = match it.next() {
                Some(Ok(entry)) => entry,
                Some(Err(err)) => return Err(err.into()),
                None => break,
            };
            match self.filter_entry(&entry) {
                ArgWalkerFilterEntryResult::Keep => entries.push(Ok(entry)),
                ArgWalkerFilterEntryResult::DiscardFile => (),
                ArgWalkerFilterEntryResult::DiscardDir => it.skip_current_dir(),
            }
        }

        progress.finish_with_message("Directory tree walked successfully");

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_walker_include_exclude() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create some files and directories
        fs::create_dir(dir_path.join("dir1")).unwrap();
        fs::create_dir(dir_path.join("dir2")).unwrap();
        fs::File::create(dir_path.join("file1.txt")).unwrap();
        fs::File::create(dir_path.join("file2.txt")).unwrap();
        fs::File::create(dir_path.join("dir1/file3.txt")).unwrap();
        fs::File::create(dir_path.join("dir2/file4.txt")).unwrap();

        // Create exclude and include patterns
        let exclude_patterns = vec![Glob::new("*.txt").unwrap()];
        let include_patterns = vec![Glob::new("dir1/*.txt").unwrap()];

        // Create the walker
        let walker = ArgWalker::try_from(
            dir_path,
            false,
            Some(exclude_patterns),
            Some(include_patterns),
            None,
            false,
        )
        .unwrap();

        // Walk the directory
        let entries = walker.try_sprint().unwrap();

        // Check the results
        assert_eq!(entries.len(), 4); // <-- We should have 4 entries (3 + root directory)
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir2")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/file3.txt")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path)); // <-- The root directory is included
    }

    #[test]
    fn test_walker_with_gitignore() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create some files and directories
        fs::create_dir(dir_path.join("dir1")).unwrap();
        fs::create_dir(dir_path.join("dir2")).unwrap();
        fs::File::create(dir_path.join("file1.txt")).unwrap();
        fs::File::create(dir_path.join("file2.txt")).unwrap();
        fs::File::create(dir_path.join("dir1/file3.txt")).unwrap();
        fs::File::create(dir_path.join("dir2/file4.txt")).unwrap();

        // Create a .gitignore file
        let gitignore_content = "*.txt\n!dir1/*.txt\n";
        fs::write(dir_path.join(".gitignore"), gitignore_content).unwrap();

        // Create the walker
        let walker = ArgWalker::try_from(dir_path, true, None, None, None, false).unwrap();

        // Walk the directory
        let entries = walker.try_sprint().unwrap();

        // Check the results
        assert_eq!(entries.len(), 5); // <-- We should have 5 entries (3 + root directory + .gitignore)
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir2")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/file3.txt")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path)); // <-- The root directory is included
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join(".gitignore")));
    }

    #[test]
    fn test_walker_nested_directories() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        fs::create_dir(dir_path.join("dir1")).unwrap(); // Should be included
        fs::create_dir(dir_path.join("dir1/subdir1")).unwrap(); // Should be included
        fs::create_dir(dir_path.join("dir1/subdir2")).unwrap(); // Should be included
        fs::create_dir(dir_path.join("dir2")).unwrap(); // Should be included
        fs::File::create(dir_path.join("file1.txt")).unwrap(); // Should be excluded
        fs::File::create(dir_path.join("dir1/file2.txt")).unwrap(); // Should be excluded
        fs::File::create(dir_path.join("dir1/subdir1/file3.txt")).unwrap(); // Should be included
        fs::File::create(dir_path.join("dir1/subdir2/file4.txt")).unwrap(); // Should be included
        fs::File::create(dir_path.join("dir2/file5.txt")).unwrap(); // Should be excluded

        let exclude_patterns = vec![Glob::new("**/*.txt").unwrap()];
        let include_patterns = vec![Glob::new("dir1/*/*.txt").unwrap()];

        let walker = ArgWalker::try_from(
            dir_path,
            false,
            Some(exclude_patterns),
            Some(include_patterns),
            None,
            false,
        )
        .unwrap();

        let entries = walker.try_sprint().unwrap();

        assert_eq!(entries.len(), 7);
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/subdir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/subdir2")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/subdir1/file3.txt")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/subdir2/file4.txt")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir2")));
    }

    #[test]
    fn test_walker_symlinks() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        fs::create_dir(dir_path.join("dir1")).unwrap();
        fs::create_dir(dir_path.join("dir2")).unwrap();
        fs::File::create(dir_path.join("file1.txt")).unwrap();
        fs::File::create(dir_path.join("dir1/file2.txt")).unwrap();
        fs::File::create(dir_path.join("dir2/file3.txt")).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(dir_path.join("dir1"), dir_path.join("symlink")).unwrap();

        let exclude_patterns = vec![Glob::new("**/*.txt").unwrap()];
        let include_patterns = vec![Glob::new("dir1/*.txt").unwrap()];

        let walker = ArgWalker::try_from(
            dir_path,
            false,
            Some(exclude_patterns),
            Some(include_patterns),
            None,
            true,
        )
        .unwrap();

        let entries = walker.try_sprint().unwrap();

        #[cfg(unix)]
        assert_eq!(entries.len(), 5);
        #[cfg(not(unix))]
        assert_eq!(entries.len(), 4);

        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/file2.txt")));

        #[cfg(unix)]
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("symlink")));
    }

    #[test]
    fn test_walker_max_depth() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        fs::create_dir(dir_path.join("dir1")).unwrap();
        fs::create_dir(dir_path.join("dir1/subdir1")).unwrap();
        fs::create_dir(dir_path.join("dir2")).unwrap();
        fs::File::create(dir_path.join("file1.txt")).unwrap();
        fs::File::create(dir_path.join("dir1/file2.txt")).unwrap();
        fs::File::create(dir_path.join("dir1/subdir1/file3.txt")).unwrap();
        fs::File::create(dir_path.join("dir2/file4.txt")).unwrap();

        let exclude_patterns = vec![Glob::new("**/*.txt").unwrap()];
        let include_patterns = vec![Glob::new("dir1/**/*.txt").unwrap()];

        let walker = ArgWalker::try_from(
            dir_path,
            false,
            Some(exclude_patterns),
            Some(include_patterns),
            Some(1), // <-- This means dir1/file2.txt will NOT be included even though it matches the include pattern
            false,
        )
        .unwrap();

        let entries = walker.try_sprint().unwrap();

        assert_eq!(entries.len(), 3);
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1")));
        assert!(entries
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir2")));
        assert!(!entries // <--- "!" here
            .iter()
            .any(|e| e.as_ref().unwrap().path() == dir_path.join("dir1/file2.txt")));
    }
}
