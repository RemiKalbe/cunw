use futures::{stream::FuturesUnordered, StreamExt};
use item::CodebaseItem;
use std::{path::PathBuf, sync::Arc};

use globset::GlobSet;
use walkdir::WalkDir;

use crate::{
    error::{CunwError, CunwErrorKind, Result},
    gitignore::GitIgnore,
    logger::Logger,
    tree::Tree,
};

pub mod item;

pub struct CodebaseBuilder {
    excluded_paths: Option<GlobSet>,
    consider_gitignores: Option<bool>,
    max_depth: Option<usize>,
    follow_symlinks: Option<bool>,
    skip_hidden_on_windows: Option<bool>,
}

impl CodebaseBuilder {
    pub fn new() -> Self {
        Self {
            excluded_paths: None,
            consider_gitignores: None,
            max_depth: None,
            follow_symlinks: None,
            skip_hidden_on_windows: None,
        }
    }

    pub fn excluded_paths(mut self, excluded_paths: GlobSet) -> Self {
        self.excluded_paths = Some(excluded_paths);
        self
    }

    pub fn consider_gitignores(mut self, consider_gitignores: bool) -> Self {
        self.consider_gitignores = Some(consider_gitignores);
        self
    }

    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = Some(follow_symlinks);
        self
    }

    pub fn skip_hidden_on_windows(mut self, skip_hidden_on_windows: bool) -> Self {
        self.skip_hidden_on_windows = Some(skip_hidden_on_windows);
        self
    }

    pub async fn build(self, from: PathBuf) -> Result<Codebase> {
        Logger::debug(format!("Building 🏗️ codebase from {}", from.display()).as_str());

        let root_tree = Tree::new(from.clone(), None);
        let mut current_tree = root_tree.clone();
        let mut files_handles = FuturesUnordered::new();

        let mut walker = WalkDir::new(from.clone()).sort_by_file_name();
        if let Some(max_depth) = self.max_depth {
            walker = walker.max_depth(max_depth);
        }
        if let Some(follow_symlinks) = self.follow_symlinks {
            walker = walker.follow_links(follow_symlinks);
        }

        let mut it = walker.into_iter();

        while let Some(entry) = it.next() {
            match entry {
                Ok(entry) => {
                    Logger::trace(format!("Processing entry {}", entry.path().display()).as_str());

                    // Skip hidden files and directories on Windows.
                    // The reason for only doing this on Windows is that the
                    // hidden attribute does not exist on Unix systems.
                    // And just checking for a dot prefix could lead to false positives.
                    // Usually, hidden fiels on windows are hidden for a reason.
                    // The 'dot' prefix on the other hand is used for things that
                    // are not necessarily hidden; like .gitignore, .github, etc.
                    #[cfg(windows)]
                    if self.skip_hidden_on_windows().unwrap_or(true) {
                        if crate::os::is_hidden_dir_entry(&entry)? {
                            Logger::trace("Skipping hidden entry");
                            continue;
                        }
                    }

                    // Get the path of the entry
                    let path = entry.path().to_path_buf();

                    // Test if the path is a child of the current branch
                    if !path.starts_with(current_tree.current_dir()) {
                        Logger::trace("It is not a child of the current branch");

                        // If not, find the closest parent by traversing up the tree
                        // until we find a parent that is a prefix of the path
                        current_tree = current_tree
                            .backtrack_to_branch(path.parent().unwrap_or(&path))
                            .ok_or(CunwError::new(CunwErrorKind::CodebaseBuild(format!(
                                "Failed to find a parent for path: {}",
                                path.display()
                            ))))?;
                    }

                    // Check if the current directory has a .gitignore file (if enabled)
                    // Find the gitignore file that is a child of the parent of the current entry
                    let maybe_gitignore = match self.consider_gitignores {
                        Some(true) => {
                            let current_path_gitignore =
                                GitIgnore::from(current_tree.current_dir())?;
                            let current_branch_gitignore = current_tree.gitignore();
                            if current_path_gitignore.is_some()
                                && current_branch_gitignore
                                    .map(|g| {
                                        g.path != current_path_gitignore.as_ref().unwrap().path
                                    })
                                    .unwrap_or(true)
                            {
                                current_tree.set_gitignore(current_path_gitignore.unwrap().clone());
                            }
                            current_tree.gitignore()
                        }
                        _ => None,
                    };
                    if let Some(gitignore) = &maybe_gitignore {
                        Logger::trace(format!("Using gitignore: {:?}", gitignore.path).as_str());
                    } else {
                        Logger::trace("No gitignore impacting current branch");
                    }

                    // Is the entry excluded by the gitignore?
                    if maybe_gitignore.map_or(false, |gitignore| gitignore.is_excluded(&path)) {
                        Logger::debug("Entry is excluded by the gitignore");

                        // If it's a directory, skip it entirely
                        if entry.file_type().is_dir() {
                            Logger::debug("Skipping directory");

                            it.skip_current_dir();
                        }
                        continue;
                    }

                    // Is the entry excluded by the ignore patterns?
                    if let Some(excluded_paths) = &self.excluded_paths {
                        if excluded_paths.is_match(&path) {
                            Logger::debug("Entry is excluded by the ignore patterns");

                            // If it's a directory, skip it entirely
                            if entry.file_type().is_dir() {
                                Logger::debug("Skipping directory");

                                it.skip_current_dir();
                            }
                            continue;
                        }
                    }

                    // Edge case: Is this the root directory?
                    if entry.path() == from {
                        Logger::trace("It is the root directory; skipping");
                        continue;
                    }

                    // Create a new branch or leaf based on the metadata
                    if entry.file_type().is_dir() {
                        Logger::trace("Creating a new branch");

                        // Create a new branch
                        let new_tree = Tree::new(path, Some(Arc::downgrade(&current_tree)));
                        // Add the branch to the current branch
                        current_tree.add_branch(new_tree.clone());
                        // Move to the new branch
                        current_tree = new_tree;
                    } else if entry.file_type().is_file() {
                        Logger::trace("Creating a new leaf");

                        let new_leaf = CodebaseItem::new(path);
                        let read_handle = new_leaf.eventually_load_content();
                        files_handles.push(read_handle);
                        // Add the new leaf to the current branch
                        current_tree.add_leaf(new_leaf);
                    }
                }
                Err(err) => {
                    Logger::error(format!("Error while reading entry: {:#?}", err).as_str());
                }
            }
        }

        // Wait for all files to be read
        let mut any_error = false;
        while let Some(res) = files_handles.next().await {
            if let Err(err) = res.expect("Failed to await file content") {
                Logger::warn(format!("Error while reading file: {:#?}", err).as_str());
                any_error = true;
            }
        }
        if any_error {
            return Err(CunwError::new(CunwErrorKind::CodebaseBuild(
                "Failed to read file(s) content(s)".to_string(),
            )));
        }

        Ok(Codebase { tree: root_tree })
    }
}

#[derive(Debug)]
pub struct Codebase {
    pub(crate) tree: Arc<Tree<CodebaseItem>>,
}

impl Codebase {
    pub fn new(tree: Arc<Tree<CodebaseItem>>) -> Self {
        Self { tree }
    }
    pub(crate) fn push_formated_tree(&self, buffer: &mut String) {
        let formated_tree = format!(
            "<directory_tree>\n{}\n</directory_tree>",
            self.tree.to_string()
        );
        buffer.push_str(&formated_tree);
    }
    pub(crate) fn push_formated_leaves_representation(&self, buffer: &mut String) {
        let leaves = self.tree.collect_all_leaves();
        for leave in leaves {
            if let Some(content) = leave.content.get() {
                let formated_content = format!(
                    "<file path=\"{}\">\n{}\n</file>\n",
                    leave.path.display(),
                    content
                );
                buffer.push_str(&formated_content);
            }
        }
    }
    pub fn try_to_string(&self) -> Result<String> {
        let mut buffer = String::new();
        self.push_formated_tree(&mut buffer);
        buffer.push_str("\n\n");
        self.push_formated_leaves_representation(&mut buffer);
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use globset::{Glob, GlobSetBuilder};
    use std::io::Write;
    use std::{
        fs::{self, File},
        path::Path,
    };
    use tempfile::TempDir;

    fn ensure_logger() {
        // Set RUST_LOG to trace
        std::env::set_var("RUST_LOG", "trace");
        // Initialize the logger
        Logger::init(None);
    }

    fn create_test_directory() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();

        File::create(dir.path().join("src/main.rs"))
            .unwrap()
            .write_all(b"fn main() {}")
            .unwrap();
        File::create(dir.path().join("src/lib.rs"))
            .unwrap()
            .write_all(b"pub fn add(a: i32, b: i32) -> i32 { a + b }")
            .unwrap();
        File::create(dir.path().join("docs/readme.md"))
            .unwrap()
            .write_all(b"# Test Project")
            .unwrap();
        File::create(dir.path().join(".gitignore"))
            .unwrap()
            .write_all(b"*.log")
            .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_codebase_builder() {
        ensure_logger();
        let dir = create_test_directory();

        let codebase = CodebaseBuilder::new()
            .max_depth(3)
            .follow_symlinks(false)
            .build(dir.path().to_path_buf())
            .await
            .unwrap();

        let mut buffer = String::new();
        codebase.push_formated_tree(&mut buffer);
        assert!(buffer.contains("/src"));
        assert!(buffer.contains("/docs"));
        assert!(buffer.contains("main.rs"));
        assert!(buffer.contains("lib.rs"));
        assert!(buffer.contains("readme.md"));
        assert!(buffer.contains(".gitignore"));
    }

    #[tokio::test]
    async fn test_codebase_file_content() {
        ensure_logger();
        let dir = create_test_directory();

        let codebase = CodebaseBuilder::new()
            .build(dir.path().to_path_buf())
            .await
            .unwrap();

        let mut buffer = String::new();
        codebase.push_formated_leaves_representation(&mut buffer);

        assert!(buffer.contains("fn main() {}"));
        assert!(buffer.contains("pub fn add(a: i32, b: i32) -> i32 { a + b }"));
        assert!(buffer.contains("# Test Project"));
        assert!(buffer.contains("*.log"));
    }

    #[tokio::test]
    async fn test_codebase_exclude_patterns() {
        ensure_logger();
        let dir = create_test_directory();
        File::create(dir.path().join("excluded.txt"))
            .unwrap()
            .write_all(b"This should be excluded")
            .unwrap();

        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("*.txt").unwrap());
        let excluded_paths = builder.build().unwrap();

        let codebase = CodebaseBuilder::new()
            .excluded_paths(excluded_paths)
            .build(dir.path().to_path_buf())
            .await
            .unwrap();

        let mut buffer = String::new();
        codebase.push_formated_leaves_representation(&mut buffer);
        assert!(!buffer.contains("excluded.txt"));
    }

    // More complex tests

    fn create_file(path: &Path, content: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "{}", content).unwrap();
    }

    fn create_nested_structure(root: &Path) {
        // Root level
        create_file(&root.join(".gitignore"), "*.log\n!important.log");
        create_file(&root.join("root.txt"), "root content");
        create_file(&root.join("root.log"), "root log");
        create_file(&root.join("important.log"), "important root log");

        // First level: src
        fs::create_dir(root.join("src")).unwrap();
        create_file(&root.join("src/.gitignore"), "*.tmp\n!keep.tmp");
        create_file(&root.join("src/main.rs"), "fn main() {}");
        create_file(&root.join("src/lib.rs"), "pub fn lib_fn() {}");
        create_file(&root.join("src/test.tmp"), "temporary file");
        create_file(&root.join("src/keep.tmp"), "kept temporary file");

        // Second level: src/module
        fs::create_dir(root.join("src/module")).unwrap();
        create_file(&root.join("src/module/.gitignore"), "*.rs\n!mod.rs");
        create_file(&root.join("src/module/mod.rs"), "pub mod submodule;");
        create_file(
            &root.join("src/module/submodule.rs"),
            "pub fn submodule_fn() {}",
        );
        create_file(
            &root.join("src/module/ignored.rs"),
            "// This should be ignored",
        );

        // First level: docs
        fs::create_dir(root.join("docs")).unwrap();
        create_file(&root.join("docs/readme.md"), "# Project Documentation");
        create_file(&root.join("docs/config.log"), "documentation log");
    }

    #[tokio::test]
    async fn test_nested_gitignore_structure() {
        ensure_logger();
        let temp_dir = TempDir::new().unwrap();
        create_nested_structure(temp_dir.path());

        let codebase = CodebaseBuilder::new()
            .consider_gitignores(true)
            .build(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Root level checks
        let root_leaves: Vec<_> = codebase.tree.collect_local_leaves();
        assert!(root_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "important.log"));
        assert!(root_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "root.txt"));
        assert!(!root_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "root.log"));

        // src directory checks
        let root_branches = codebase.tree.collect_local_branches();
        let src_dir = root_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "src")
            .expect("src directory not found");

        let src_items: Vec<_> = src_dir.collect_local_leaves();
        assert!(src_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "main.rs"));
        assert!(src_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "lib.rs"));
        assert!(src_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "keep.tmp"));
        assert!(!src_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "test.tmp"));

        // src/module directory checks
        let src_branches = src_dir.collect_local_branches();
        let module_dir = src_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "module")
            .expect("module directory not found");

        let module_items: Vec<_> = module_dir.collect_local_leaves();
        assert!(module_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "mod.rs"));
        assert!(!module_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "submodule.rs"));
        assert!(!module_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "ignored.rs"));

        // docs directory checks
        let docs_dir = root_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "docs")
            .expect("docs directory not found");
        let docs_items: Vec<_> = docs_dir.collect_local_leaves();
        assert!(docs_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "readme.md"));
        assert!(!docs_items
            .iter()
            .any(|item| item.path.file_name().unwrap() == "config.log"));
    }

    #[tokio::test]
    async fn test_gitignore_override() {
        ensure_logger();
        let temp_dir = TempDir::new().unwrap();
        create_nested_structure(temp_dir.path());

        // Override the src/.gitignore to ignore all .rs files
        create_file(&temp_dir.path().join("src/.gitignore"), "*.rs");

        let codebase = CodebaseBuilder::new()
            .consider_gitignores(true)
            .build(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let root_branches = codebase.tree.collect_local_branches();
        let src_dir = root_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap().to_str().unwrap() == "src")
            .unwrap();

        let src_leaves = src_dir.collect_local_leaves();
        assert!(!src_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "main.rs"));
        assert!(!src_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "lib.rs"));

        // The module's .gitignore should still apply
        let src_branches = src_dir.collect_local_branches();
        let module_dir = src_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "module")
            .unwrap();

        let module_leaves = module_dir.collect_local_leaves();
        assert!(module_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "mod.rs"));
    }

    #[tokio::test]
    async fn test_gitignore_disabled() {
        ensure_logger();
        let temp_dir = TempDir::new().unwrap();
        create_nested_structure(temp_dir.path());

        let codebase = CodebaseBuilder::new()
            .consider_gitignores(false)
            .build(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let root_branches = codebase.tree.collect_local_branches();
        let root_leaves = codebase.tree.collect_local_leaves();

        // All files should be included when gitignore is disabled
        assert!(root_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "root.log"));

        let src_dir = root_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "src")
            .unwrap();

        let src_leaves = src_dir.collect_local_leaves();
        assert!(src_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "test.tmp"));

        let src_branches = src_dir.collect_local_branches();
        let module_dir = src_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "module")
            .unwrap();

        let module_leaves = module_dir.collect_local_leaves();
        assert!(module_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "submodule.rs"));
        assert!(module_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "ignored.rs"));

        let docs_dir = root_branches
            .iter()
            .find(|item| item.current_dir().file_name().unwrap() == "docs")
            .unwrap();

        let docs_leaves = docs_dir.collect_local_leaves();
        assert!(docs_leaves
            .iter()
            .any(|item| item.path.file_name().unwrap() == "config.log"));
    }
}
