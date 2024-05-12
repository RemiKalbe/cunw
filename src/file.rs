use std::{
    cell::RefCell,
    fs,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};
use ptree::{item::StringItem, TreeBuilder};

use crate::{error::Error, logger::Logger};

pub struct File {
    pub path: String,
    pub content: String,
}

impl File {
    pub fn new(path: String, content: String) -> Self {
        Self { path, content }
    }
    pub fn format(&self) -> String {
        format!(
            r#"<file path="{}">
            {}
            </file>"#,
            self.path, self.content
        )
    }
}

pub struct FileCollector {
    root: PathBuf,
    pub files: Vec<File>,
}

impl FileCollector {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Vec::new(),
        }
    }

    pub fn collect_file(&mut self, path: &Path) -> Result<(), Error> {
        Logger::log_message(
            log::Level::Debug,
            "Collecting file",
            Some(path.to_str().unwrap()),
        )
        .unwrap();
        Logger::log_message(
            log::Level::Trace,
            "Stripping prefix",
            Some(format!("{:?} - {:?}", path, self.root).as_str()),
        )
        .unwrap();

        let relative_path = path.strip_prefix(&self.root).unwrap();

        Logger::log_message(
            log::Level::Trace,
            "Path is now",
            Some(relative_path.to_str().unwrap()),
        )
        .unwrap();

        let file = fs::read(path)?;
        let content = String::from_utf8(file).unwrap();

        Logger::log_message(log::Level::Trace, "File content read", None).unwrap();

        self.files.push(File {
            path: relative_path.to_str().unwrap().to_string(),
            content,
        });

        Ok(())
    }

    pub fn collect_files(&mut self, files: Vec<PathBuf>) -> Result<(), Error> {
        let progress = ProgressBar::new(files.len() as u64);
        progress.enable_steady_tick(Duration::from_millis(100));
        progress.set_style(
            ProgressStyle::with_template(
                "{spinner:.144} {pos}/{len} {msg}", // 144: NavajoWhite3 #afaf87
            )
            .unwrap()
            .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
        );

        progress.set_message("Collecting files' content");

        Logger::log_message(
            log::Level::Debug,
            "Collecting files",
            Some(format!("{:#?}", files).as_str()),
        )
        .unwrap();

        for file in files {
            self.collect_file(&file)?;
            progress.inc(1);
        }

        progress.finish_with_message("Files collected");

        Logger::log_message(log::Level::Debug, "Files collected", None).unwrap();

        Ok(())
    }
}

pub struct DirectoryTree {
    path: PathBuf,
    parent: Option<PathBuf>,
    childs: Option<Vec<DirectoryTree>>,
    siblings: Option<Vec<PathBuf>>,
    orphans: Option<Vec<PathBuf>>,
}

impl DirectoryTree {
    pub fn from(
        root: PathBuf,
        parent: PathBuf,
        path: PathBuf,
        paths: Option<Rc<RefCell<Vec<PathBuf>>>>,
        progress: &ProgressBar,
    ) -> Self {
        if let Some(ref paths) = paths {
            progress.set_length(paths.borrow().len() as u64);
        }

        Logger::log_message(
            log::Level::Debug,
            "Creating directory tree",
            Some(path.to_str().unwrap()),
        )
        .unwrap();

        Logger::log_message(log::Level::Trace, "Removing parent from path", None).unwrap();

        paths.as_ref().map(|ps| {
            let mut ps = ps.borrow_mut();
            let f = ps
                .iter()
                .filter(|path| *path != &parent) // Remove parent from paths (just in case)
                .map(|path| path.to_path_buf())
                .collect::<Vec<_>>();
            *ps = f;
        });

        Logger::log_message(log::Level::Trace, "Sorting paths", None).unwrap();

        // Sort paths in reverse order
        paths.as_ref().map(|ps| {
            let mut ps = ps.borrow_mut();
            ps.sort();
        });
        paths.as_ref().map(|ps| {
            let mut ps = ps.borrow_mut();
            ps.reverse();
        });

        Logger::log_message(
            log::Level::Debug,
            "Paths sorted",
            Some(format!("{:#?}", paths).as_str()),
        )
        .unwrap();

        let parent = parent.clone();
        let mut childs = Vec::new();
        let mut siblings = Vec::new();

        Logger::log_message(log::Level::Trace, "Looking for orphans", None).unwrap();

        // Orphans are paths that do not have the root as common ancestor
        // There should only be on Vector of orphans, at the beginning.
        let orphans = paths.as_ref().map(|ps| {
            let ps = ps.borrow();
            ps.iter()
                .filter(|path| !path.starts_with(&root))
                .map(|path| path.to_path_buf())
                .collect::<Vec<_>>()
        });

        if !orphans.as_ref().unwrap().is_empty() {
            Logger::log_message(
                log::Level::Trace,
                "Orphans found",
                Some(format!("{:#?}", orphans).as_str()),
            )
            .unwrap();
        }

        if let Some(ref paths_ref) = paths {
            while !(paths_ref.borrow().is_empty() || paths.is_none()) {
                // We need to determine if the next path is:
                // - a child of the current path
                // - a sibling of the current path
                // - an orphan
                Logger::log_message(log::Level::Trace, "Paths is not empty", None).unwrap();
                let paths = paths.as_ref().clone();
                let next_path = &paths
                    .as_ref()
                    .map(|ps| ps.borrow_mut().pop())
                    .flatten()
                    .unwrap(); // We know that the path exists, because we checked if the vector is empty before
                Logger::log_message(
                    log::Level::Trace,
                    "Next path",
                    Some(next_path.to_str().unwrap()),
                )
                .unwrap();

                let maybe_next_path_relative = next_path.strip_prefix(&parent);
                match maybe_next_path_relative {
                    Ok(next_path_relative) => {
                        Logger::log_message(
                            log::Level::Trace,
                            "Next path relative",
                            Some(next_path_relative.to_str().unwrap()),
                        )
                        .unwrap();
                        // The next path is a child or a sibling
                        // Which one is it? -> Check if it's a file
                        if next_path.is_file() {
                            Logger::log_message(
                                log::Level::Trace,
                                "Next path is a file",
                                Some("Treating it as a sibling"),
                            )
                            .unwrap();

                            // It's a file, so it's a sibling
                            siblings.push(next_path_relative.to_path_buf());
                            continue;
                        } else {
                            Logger::log_message(
                                log::Level::Trace,
                                "Next path is a directory",
                                Some("Treating it as a child"),
                            )
                            .unwrap();

                            // The new parent is the next path
                            let parent = next_path.to_path_buf();

                            // It's a directory, so it's a child
                            childs.push(DirectoryTree::from(
                                root.clone(),
                                parent.clone(),
                                next_path.to_path_buf(),
                                paths.cloned(),
                                progress,
                            ));
                            continue;
                        }
                    }
                    Err(e) => {
                        Logger::log_message(
                            log::Level::Trace,
                            "Next path is not a child - Nor a sibling",
                            Some("Backtracking"),
                        )
                        .unwrap();
                        // We have finished this branch.
                        break;
                    }
                }
            }
        }

        Self {
            path,
            parent: Some(parent),
            childs: Some(childs),
            siblings: Some(siblings),
            orphans,
        }
    }
    pub fn get_branch(&self, tree_builder: &mut TreeBuilder) {
        Logger::log_message(log::Level::Trace, "Building branch", None).unwrap();

        tree_builder.begin_child(self.path.to_str().unwrap().to_string());
        if let Some(childs) = &self.childs {
            for child in childs {
                child.get_branch(tree_builder);
            }
        }

        if let Some(siblings) = &self.siblings {
            for sibling in siblings {
                tree_builder.add_empty_child(sibling.to_str().unwrap().to_string());
            }
        }

        Logger::log_message(log::Level::Trace, "Ending branch", None).unwrap();

        tree_builder.end_child();
    }
    pub fn get_tree(&self) -> StringItem {
        Logger::log_message(log::Level::Debug, "Building tree", None).unwrap();

        let mut tree_builder = TreeBuilder::new(String::from("."));

        if let Some(orphans) = &self.orphans {
            for orphan in orphans {
                tree_builder.add_empty_child(orphan.to_str().unwrap().to_string());
            }
        }

        self.get_branch(&mut tree_builder);

        Logger::log_message(log::Level::Debug, "Tree built", None).unwrap();

        let res = tree_builder.build();
        res
    }
    pub fn write_tree(&self, w: &mut impl Write) -> std::io::Result<()> {
        Logger::log_message(log::Level::Debug, "Writing tree", None).unwrap();

        let tree = self.get_tree();
        ptree::write_tree_with(&tree, w, &ptree::print_config::PrintConfig::default())?;
        Ok(())
    }
}

pub struct OutputGenerator {
    pub files: Vec<File>,
    pub tree: DirectoryTree,
}

impl OutputGenerator {
    pub fn new(files: Vec<File>, tree: DirectoryTree) -> Self {
        Self { files, tree }
    }

    pub fn write_file(&self, output_file: &Path) -> Result<(), Error> {
        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(Duration::from_millis(200));
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap()
                .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
        );
        progress.set_message("Writing output file");

        Logger::log_message(log::Level::Debug, "Writing output file", None).unwrap();

        let mut f = fs::File::create(output_file)?;

        writeln!(f, "<directory_structure>")?;
        self.tree.write_tree(&mut f)?;
        writeln!(f, "</directory_structure>")?;

        for file in &self.files {
            writeln!(f, "{}", file.format().as_str())?;
        }

        progress.finish_with_message("Output file written 🎉");

        Logger::log_message(log::Level::Debug, "Output file written", None).unwrap();

        Ok(())
    }
}
