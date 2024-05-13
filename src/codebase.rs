use futures::AsyncReadExt;
use miette::{IntoDiagnostic, Result, WrapErr};
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;

use arc_swap::ArcSwap;
use globset::GlobSet;
use nuclei::Task;
use termtree::Tree;
use walkdir::WalkDir;

use crate::{gitignore::GitIgnore, logger::Logger};

pub fn search_parent(path: PathBuf, root: &mut Tree<CodebaseItem>) -> &mut Tree<CodebaseItem> {
    Logger::trace(format!("Searching for parent of {} 👀", path.display()).as_str());
    if path.starts_with(&root.root.path()) {
        Logger::trace(format!("Found parent 🎉 {}", root.root.path().display()).as_str());

        return root;
    }

    for child in &mut root.leaves {
        let result = search_parent(path.clone(), child);
        if result.root == root.root {
            Logger::trace(format!("Found parent 🎉 {}", root.root.path().display()).as_str());

            return root;
        }
    }

    Logger::trace(format!("No parent found for {} 😢", path.display()).as_str());

    root
}

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

    pub fn build(self, from: PathBuf) -> Result<Codebase> {
        Logger::debug(format!("Building 🏗️ codebase from {}", from.display()).as_str());

        let root = CodebaseItem::Directory(Arc::new(CodebaseDir {
            path: from.clone(),
            parent: None,
        }));
        let mut tree = Tree::new(root);

        let mut walker = WalkDir::new(from).sort_by_file_name();
        if let Some(max_depth) = self.max_depth {
            walker = walker.max_depth(max_depth);
        }
        if let Some(follow_symlinks) = self.follow_symlinks {
            walker = walker.follow_links(follow_symlinks);
        }

        let mut it = walker.into_iter();
        // Keep track of the parents of the current branch.
        // This allows us to rewind once we reach a leaf.
        let mut current = &mut tree;
        let initial_gitignore = Arc::new(GitIgnore::from_dir(current.root.path())?);
        // This could be improved, here the Option is used to satisfy the compiler
        // but it is guaranteed to be Some.
        let mut gitignores: Vec<Arc<Option<GitIgnore>>> = Vec::new();
        if initial_gitignore.is_some() {
            gitignores.push(initial_gitignore.clone());
        }

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
                        crate::os::is_hidden_dir_entry(&entry)?;
                    }

                    // Get the path of the entry
                    let path = entry.path().to_path_buf();

                    // Test if the path is a child of the current branch
                    if !path.starts_with(current.root.path()) {
                        Logger::trace("It is not a child of the current branch");

                        // If not, find the closest parent by traversing up the tree
                        // until we find a parent that is a prefix of the path
                        current = search_parent(path.clone(), &mut tree);
                    }

                    //
                    // Check if the current directory has a .gitignore file (if enabled)
                    //
                    // 1. Get the path of the parent of the current entry
                    let parent_path = current.root.path();

                    Logger::trace(format!("Parent path: {}", parent_path.display()).as_str());

                    // 2. Find the gitignore file that is a child of the parent of the current entry
                    let maybe_gitignore = {
                        if self.consider_gitignores.unwrap_or(false) {
                            match gitignores.iter().find(|gitignore| {
                                gitignore.as_ref().as_ref().map_or(false, |gitignore| {
                                    gitignore.path.parent() == Some(parent_path)
                                    // <-- Check if the gitignore is a child of the parent
                                })
                            }) {
                                None => {
                                    Logger::trace("No gitignore found in the current branch");

                                    // If no gitignore is found, let's first check if the current entry's parent has a gitignore
                                    let maybe_gitignore = GitIgnore::from_dir(parent_path)?;
                                    match maybe_gitignore {
                                        Some(gitignore) => {
                                            Logger::trace(
                                                format!(
                                                    "Found gitignore in {}",
                                                    parent_path.display()
                                                )
                                                .as_str(),
                                            );

                                            let gitignore = Arc::new(Some(gitignore));
                                            gitignores.push(gitignore.clone());
                                            gitignore
                                        }
                                        None => {
                                            Logger::trace(
                                                format!(
                                                    "No gitignore found in {}",
                                                    parent_path.display()
                                                )
                                                .as_str(),
                                            );

                                            // If no gitignore is found, rewind the gitignores until we find a gitignore
                                            // that is a child of the parent of the current entry
                                            let mut maybe_candidate_gitignore = gitignores.pop();
                                            while let Some(c) = maybe_candidate_gitignore.as_ref() {
                                                if let Some(uc) = c.as_ref() {
                                                    // ^-- This is guaranteed to be Some

                                                    Logger::trace(
                                                        format!(
                                                            "Checking if the rewinded gitignore is a child of the parent: {}",
                                                            uc.path.display()
                                                        )
                                                        .as_str(),
                                                    );

                                                    if uc.path.parent() == Some(parent_path) {
                                                        // ^-- Check if the gitignore is a child of the parent
                                                        // If a gitignore is found, push it back to the gitignores stack

                                                        Logger::trace(
                                                            "It is a child of the parent 🥰, using this gitignore"
                                                        );

                                                        let gitignore = c.clone();
                                                        gitignores.push(Arc::clone(&gitignore));
                                                        break;
                                                    }
                                                    // Pop the last gitignore and try again
                                                    maybe_candidate_gitignore = gitignores.pop();
                                                }
                                            }
                                            Logger::trace("No gitignore found in the current branch, using the initial gitignore (if any)");

                                            // If no gitignore is found, use the initial gitignore
                                            match maybe_candidate_gitignore {
                                                Some(gitignore) => gitignore,
                                                None => initial_gitignore.clone(),
                                            }
                                        }
                                    }
                                }
                                Some(gitignore) => gitignore.clone(),
                            }
                        } else {
                            Arc::new(None)
                        }
                    };

                    // Is the entry excluded by the gitignore?
                    if maybe_gitignore
                        .as_ref()
                        .as_ref()
                        .map_or(false, |gitignore| gitignore.is_excluded(&path))
                    {
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

                    // Create a new branch or leaf based on the metadata
                    if entry.file_type().is_dir() {
                        Logger::trace("Creating a new branch");

                        // Create a new branch
                        let new_branch = CodebaseItem::Directory(Arc::new(CodebaseDir {
                            path,
                            parent: Some(Arc::downgrade(&current.root.as_dir().unwrap())),
                        }));
                        let new_tree = Tree::new(new_branch);
                        // Add the new branch to the current branch
                        current.leaves.push(new_tree);
                        // Move the current branch to the new branch
                        current = current.leaves.last_mut().unwrap();
                    } else if entry.file_type().is_file() {
                        Logger::trace("Creating a new leaf");

                        let new_leaf = CodebaseItem::File(Arc::new(CodebaseFile::from_path(
                            path,
                            Arc::downgrade(&current.root.as_dir().unwrap()),
                        )));
                        // Add the new leaf to the current branch
                        current.leaves.push(Tree::new(new_leaf));
                    }
                }
                Err(err) => {
                    Logger::error(format!("Error while reading entry: {:#?}", err).as_str());
                }
            }
        }

        Ok(Codebase { tree })
    }
}

#[derive(Debug)]
pub struct CodebaseDir {
    pub parent: Option<Weak<CodebaseDir>>,
    pub path: PathBuf,
}

impl PartialEq for CodebaseDir {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Display for CodebaseDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // As this will be used to write the tree, we only
        // need to display the directory name (with a trailing slash)
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        write!(f, "{}/", name)
    }
}

#[derive(Debug)]
pub struct FileContent {
    task: Arc<Mutex<Option<Task<Result<String>>>>>,
    content: ArcSwap<Option<String>>,
}

impl FileContent {
    pub fn from_path(path: PathBuf) -> Self {
        let task = nuclei::spawn(async move {
            let fo = File::open(&path)
                .expect(format!("Failed to open file: {}", path.display()).as_str());

            let mut file = nuclei::Handle::<File>::new(fo)
                .expect(format!("Failed to create file handle: {}", path.display()).as_str());
            let mut buffer = String::new();

            file.read_to_string(&mut buffer)
                .await
                .into_diagnostic()
                .wrap_err(format!("Failed to read file 😬: {}", path.display()))?;

            Ok(buffer)
        });
        let content = Arc::new(None);

        Self {
            task: Arc::new(Mutex::new(Some(task))),
            content: ArcSwap::new(content),
        }
    }
    pub async fn content(&self) -> Result<String> {
        let mut task = self.task.lock().await;
        let task = task.take().expect("Task is already taken 🤨, I messed up.");
        let content = task.await?;
        self.content.store(Arc::new(Some(content.clone())));
        Ok(content)
    }
}

#[derive(Debug)]
pub struct CodebaseFile {
    pub parent: Weak<CodebaseDir>,
    pub path: PathBuf,
    pub content: FileContent,
}

impl PartialEq for CodebaseFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl CodebaseFile {
    pub fn from_path(path: PathBuf, parent: Weak<CodebaseDir>) -> Self {
        let content = FileContent::from_path(path.clone());

        Self {
            parent,
            path,
            content,
        }
    }
    pub async fn content(&self) -> Result<String> {
        self.content.content().await
    }
}

impl Display for CodebaseFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // As this will be used to write the tree, we only
        // need to display the file name
        write!(f, "{}", self.path.file_name().unwrap().to_string_lossy())
    }
}

#[derive(Debug)]
pub enum CodebaseItem {
    File(Arc<CodebaseFile>),
    Directory(Arc<CodebaseDir>),
}

impl CodebaseItem {
    pub fn path(&self) -> &Path {
        match self {
            CodebaseItem::File(file) => file.path.as_path(),
            CodebaseItem::Directory(dir) => dir.path.as_path(),
        }
    }
    pub fn parent(&self) -> Option<Arc<CodebaseDir>> {
        match self {
            CodebaseItem::File(file) => file.parent.upgrade(),
            CodebaseItem::Directory(dir) => {
                if let Some(parent) = dir.parent.as_ref() {
                    parent.upgrade()
                } else {
                    None
                }
            }
        }
    }
    pub fn as_dir(&self) -> Option<Arc<CodebaseDir>> {
        match self {
            CodebaseItem::File(_) => None,
            CodebaseItem::Directory(dir) => Some(dir.clone()),
        }
    }
}

impl PartialEq for CodebaseItem {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl Display for CodebaseItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CodebaseItem::File(file) => write!(f, "{}", file),
            CodebaseItem::Directory(dir) => write!(f, "{}", dir),
        }
    }
}

#[derive(Debug)]
pub struct Codebase {
    tree: Tree<CodebaseItem>,
}

impl Codebase {
    pub fn new(tree: Tree<CodebaseItem>) -> Self {
        Self { tree }
    }
    pub fn get_formated_tree(&self) -> String {
        format!("<directory_tree>\n{}\n</directory_tree>", self.tree)
    }
    fn get_formated_leaves_representation(
        tree: &Tree<CodebaseItem>,
        buffer: &mut String,
    ) -> Result<()> {
        let mut leaves = tree.leaves.iter();
        while let Some(leave) = leaves.next() {
            if let CodebaseItem::File(file) = &leave.root {
                let content = nuclei::block_on(async move { file.content().await })?;

                let formated_content = format!(
                    "<file path={}>\n{}\n</file>\n",
                    file.path.display(),
                    content
                );
                buffer.push_str(formated_content.as_str());
            }
            if let CodebaseItem::Directory(_) = &leave.root {
                Self::get_formated_leaves_representation(leave, buffer)?;
            }
        }
        Ok(())
    }
    pub fn get_formated_files_representation(&self) -> Result<String> {
        let mut buffer = String::new();
        Self::get_formated_leaves_representation(&self.tree, &mut buffer)?;
        Ok(buffer)
    }
}
