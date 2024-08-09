use miette::{diagnostic, IntoDiagnostic, Result, Severity};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
};

use crate::gitignore::GitIgnore;

const IS_CHILD_GLIPH: &str = "├─ ";
const FIRST_CHILD_GLIPH: &str = "┬─ ";
const LAST_CHILD_GLIPH: &str = "└─ ";
const SKIP_GLIPH: &str = "│  ";
const SKIP_GLIPH_GAP: &str = "   ";

#[derive(Debug, Clone)]
pub struct Tree<T: Clone + PartialEq + Display> {
    current_dir: PathBuf,
    parent: Option<Weak<Tree<T>>>,
    leaves: Arc<Mutex<Vec<T>>>,
    gitignore: OnceLock<GitIgnore>,
    branches: Arc<Mutex<Vec<Tree<T>>>>,
}

impl<T: Clone + PartialEq + Display> Tree<T> {
    pub fn new(current_dir: PathBuf, parent: Option<Weak<Tree<T>>>) -> Self {
        Self {
            current_dir,
            parent,
            leaves: Arc::new(Mutex::new(Vec::new())),
            gitignore: OnceLock::new(),
            branches: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_leaf(&self, leaf: T) {
        let mut leaves = self.leaves.lock().unwrap();
        leaves.push(leaf);
    }

    pub fn add_branch(&self, branch: Tree<T>) {
        let mut branches = self.branches.lock().unwrap();
        branches.push(branch);
    }

    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    pub fn parent(&self) -> Option<Arc<Tree<T>>> {
        self.parent.as_ref().map(|parent| parent.upgrade().unwrap())
    }

    pub fn gitignore(&self) -> Option<GitIgnore> {
        if self.gitignore.get().is_some() {
            return self.gitignore.get().cloned();
        }
        if let Some(parent) = self.parent() {
            return parent.gitignore();
        }
        None
    }

    pub fn backtrack_to_branch(&self, path: &Path) -> Option<Tree<T>> {
        if path == self.current_dir() {
            return Some(self.clone());
        }
        if let Some(parent) = self.parent() {
            return parent.backtrack_to_branch(path);
        }
        None
    }

    pub fn collect_all_leaves(&self) -> Vec<T> {
        let mut local_leaves = {
            let local_leaves_lock = self.leaves.lock().unwrap();
            local_leaves_lock
                .iter()
                .map(|leave| leave.clone())
                .collect::<Vec<_>>()
        };
        let mut branches_leaves = Vec::new();
        for branch in self.branches.lock().unwrap().iter() {
            branches_leaves.extend(branch.collect_all_leaves());
        }
        local_leaves.extend(branches_leaves);
        local_leaves
    }

    pub fn to_string(&self) -> String {
        let mut buffer = String::new();
        self.build_string(&mut buffer, "", true);
        // Remove the last newline character
        buffer.pop();
        buffer
    }

    fn build_string(&self, buffer: &mut String, branch_prefix: &str, is_last_at_level: bool) {
        let branches_len = self.branches.lock().unwrap().len();
        let leaves_len = self.leaves.lock().unwrap().len();
        let dir_name = self.current_dir.file_name().map(|f| f.to_str().unwrap());

        let current_branch_display = format!(
            "{}{}/{}\n",
            branch_prefix,
            if (branches_len > 1 || !is_last_at_level) && dir_name.is_some() {
                IS_CHILD_GLIPH
            } else if dir_name.is_some() {
                LAST_CHILD_GLIPH
            } else {
                ""
            },
            dir_name.unwrap_or_default()
        );

        buffer.push_str(&current_branch_display);

        for (i, branch) in self.branches.lock().unwrap().iter().enumerate() {
            let new_branch_prefix = format!(
                "{}{}",
                branch_prefix,
                if dir_name.is_none() {
                    ""
                } else if i == branches_len - 1 && leaves_len == 0 {
                    SKIP_GLIPH_GAP
                } else {
                    SKIP_GLIPH
                }
            );

            branch.build_string(
                buffer,
                &new_branch_prefix,
                i == branches_len - 1 && leaves_len == 0,
            );
        }

        for (i, leaf) in self.leaves.lock().unwrap().iter().enumerate() {
            let new_leaf_prefix = format!(
                "{}{}",
                branch_prefix,
                if i == leaves_len - 1 {
                    LAST_CHILD_GLIPH
                } else {
                    IS_CHILD_GLIPH
                }
            );
            let new_leaf_display = format!(
                "{}{}{}\n",
                if dir_name.is_none() {
                    ""
                } else if !is_last_at_level {
                    SKIP_GLIPH
                } else {
                    SKIP_GLIPH_GAP
                },
                new_leaf_prefix,
                leaf.to_string()
            );

            buffer.push_str(&new_leaf_display);
        }
    }
}

impl<T: Clone + PartialEq + Display> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        let self_leaves = self.collect_all_leaves();
        let other_leaves = other.collect_all_leaves();
        // Test that all elements in self_leaves are present in other_leaves
        // And they both have the same parent
        self_leaves.iter().all(|x| other_leaves.contains(&x)) && self.parent() == other.parent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, ops::Deref, sync::Arc};
    use tempfile::TempDir;

    #[test]
    fn test_new_tree() {
        let root_path = PathBuf::from("/");
        let tree: Tree<String> = Tree::new(root_path.clone(), None);

        assert_eq!(tree.current_dir(), root_path.as_path());
        assert!(tree.parent().is_none());
        assert_eq!(tree.leaves.lock().unwrap().len(), 0);
        assert_eq!(tree.branches.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_add_leaf() {
        let root_path = PathBuf::from("/");
        let tree: Tree<String> = Tree::new(root_path, None);

        tree.add_leaf(String::from("leaf1"));
        assert_eq!(tree.leaves.lock().unwrap().len(), 1);
        assert_eq!(tree.leaves.lock().unwrap()[0], "leaf1");
    }

    #[test]
    fn test_add_branch() {
        let root_path = PathBuf::from("/");
        let tree: Tree<String> = Tree::new(root_path.clone(), None);

        let branch_path = PathBuf::from("/branch");
        let branch: Tree<String> = Tree::new(
            branch_path.clone(),
            Some(Arc::downgrade(&Arc::new(tree.clone()))),
        );

        tree.add_branch(branch.clone());
        assert_eq!(tree.branches.lock().unwrap().len(), 1);
        assert_eq!(
            tree.branches.lock().unwrap()[0].current_dir(),
            branch_path.as_path()
        );
    }

    #[test]
    fn test_parent() {
        let root_path = PathBuf::from("/");
        let tree: Tree<String> = Tree::new(root_path.clone(), None);

        let root = Arc::new(tree);

        let branch_path = PathBuf::from("/branch");
        let branch: Tree<String> = Tree::new(branch_path.clone(), Some(Arc::downgrade(&root)));

        assert!(branch.parent().is_some());
        assert_eq!(branch.parent().unwrap().current_dir(), root_path.as_path());
    }

    #[test]
    fn test_backtrack_to_branch() {
        let root_path = PathBuf::from("/");
        let root: Arc<Tree<String>> = Arc::new(Tree::new(root_path.clone(), None));

        let branch_path = PathBuf::from("/branch");
        let branch: Arc<Tree<String>> = Arc::new(Tree::new(
            branch_path.clone(),
            Some(Arc::downgrade(&root.clone())),
        ));

        let leaf_path = PathBuf::from("/branch/leaf");
        let leaf: Tree<String> =
            Tree::new(leaf_path.clone(), Some(Arc::downgrade(&branch.clone())));

        assert_eq!(
            leaf.backtrack_to_branch(&branch_path).as_ref(),
            Some(branch.deref())
        );
        assert_eq!(
            leaf.backtrack_to_branch(&root_path).as_ref(),
            Some(root.deref())
        );
        assert_eq!(
            branch.backtrack_to_branch(&root_path).as_ref(),
            Some(root.deref())
        );
        assert!(root.backtrack_to_branch(&leaf_path).is_none());
    }

    #[test]
    fn test_collect_all_leaves() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));
        tree.add_leaf("leaf1".to_string());

        let branch_path = PathBuf::from("/branch");
        let branch = Tree::new(branch_path.clone(), Some(Arc::downgrade(&tree)));
        branch.add_leaf("leaf2".to_string());

        tree.add_branch(branch.clone());

        let leaves = tree.collect_all_leaves();
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&"leaf1".to_string()));
        assert!(leaves.contains(&"leaf2".to_string()));
    }

    #[test]
    fn test_partial_eq() {
        let root_path = PathBuf::from("/");
        let tree1 = Arc::new(Tree::new(root_path.clone(), None));
        tree1.add_leaf("leaf1".to_string());

        let tree2 = Arc::new(Tree::new(root_path, None));
        tree2.add_leaf("leaf1".to_string());

        assert_eq!(tree1, tree2);
    }

    #[test]
    fn test_gitignore() {
        let temp_dir = TempDir::new().expect("Unable to create temp dir");
        let gitignore_path = temp_dir.path().join(".gitignore");

        fs::write(&gitignore_path, "*.rs").expect("Unable to write to .gitignore");

        // Create GitIgnore from the temporary path
        let gitignore = GitIgnore::from(&gitignore_path)
            .expect("Failed to create GitIgnore")
            .expect("GitIgnore is None");

        let tree1: Arc<Tree<String>> = Arc::new(Tree::new(temp_dir.path().to_path_buf(), None));
        tree1.gitignore.set(gitignore.clone()).unwrap();

        let branch_path = temp_dir.path().join("branch");
        let branch = Arc::new(Tree::new(branch_path, Some(Arc::downgrade(&tree1))));

        assert_eq!(branch.gitignore(), Some(gitignore));
    }

    #[test]
    fn test_nested_gitignore() {
        let temp_dir = TempDir::new().expect("Unable to create temp dir");
        let gitignore_path = temp_dir.path().join(".gitignore");

        fs::write(&gitignore_path, "*.rs").expect("Unable to write to .gitignore");

        // Create GitIgnore from the temporary path
        let gitignore = GitIgnore::from(&gitignore_path)
            .expect("Failed to create GitIgnore")
            .expect("GitIgnore is None");

        let tree1: Arc<Tree<String>> = Arc::new(Tree::new(temp_dir.path().to_path_buf(), None));
        tree1.gitignore.set(gitignore.clone()).unwrap();

        let branch_path = temp_dir.path().join("branch");
        let branch = Arc::new(Tree::new(branch_path.clone(), Some(Arc::downgrade(&tree1))));

        let nested_branch_path = branch_path.join("nested");
        let nested_branch = Tree::new(nested_branch_path, Some(Arc::downgrade(&branch)));

        assert_eq!(nested_branch.gitignore(), Some(gitignore));
    }

    #[test]
    fn test_tree_to_string() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));
        tree.add_leaf("leaf1".to_string());

        let branch_path = PathBuf::from("/branch");
        let branch = Tree::new(branch_path.clone(), Some(Arc::downgrade(&tree)));
        branch.add_leaf("leaf2".to_string());

        tree.add_branch(branch.clone());

        let expected = "/\n├─ /branch\n│  └─ leaf2\n└─ leaf1";
        let output = tree.to_string();

        assert_eq!(output, expected);
    }

    #[test]
    fn test_tree_with_multiple_branches_and_leaves() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));

        tree.add_leaf("leaf1".to_string());
        tree.add_leaf("leaf2".to_string());

        let branch1_path = PathBuf::from("/branch1");
        let branch1 = Tree::new(branch1_path.clone(), Some(Arc::downgrade(&tree)));
        branch1.add_leaf("leaf3".to_string());

        let branch2_path = PathBuf::from("/branch2");
        let branch2 = Tree::new(branch2_path.clone(), Some(Arc::downgrade(&tree)));
        branch2.add_leaf("leaf4".to_string());

        tree.add_branch(branch1.clone());
        tree.add_branch(branch2.clone());

        let expected = "/\n├─ /branch1\n│  └─ leaf3\n├─ /branch2\n│  └─ leaf4\n├─ leaf1\n└─ leaf2";
        let output = tree.to_string();

        assert_eq!(output, expected);
    }

    #[test]
    fn test_tree_with_nested_branches() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));
        tree.add_leaf("leaf1".to_string());

        let branch1_path = PathBuf::from("/branch1");
        let branch1 = Tree::new(branch1_path.clone(), Some(Arc::downgrade(&tree)));
        let branch1_arc = Arc::new(branch1.clone());
        branch1.add_leaf("leaf2".to_string());

        let branch2_path = PathBuf::from("/branch1/branch2");
        let branch2 = Tree::new(branch2_path.clone(), Some(Arc::downgrade(&branch1_arc)));
        branch2.add_leaf("leaf3".to_string());

        branch1.add_branch(branch2.clone());
        tree.add_branch(branch1.clone());

        let expected = "/\n├─ /branch1\n│  ├─ /branch2\n│  │  └─ leaf3\n│  └─ leaf2\n└─ leaf1";
        let output = tree.to_string();

        assert_eq!(output, expected);
    }

    #[test]
    fn test_tree_with_deeply_nested_branches() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));

        let branch1_path = PathBuf::from("/branch1");
        let branch1 = Tree::new(branch1_path.clone(), Some(Arc::downgrade(&tree)));
        let branch1_arc = Arc::new(branch1.clone());

        let branch2_path = PathBuf::from("/branch1/branch2");
        let branch2 = Tree::new(branch2_path.clone(), Some(Arc::downgrade(&branch1_arc)));
        let branch2_arc = Arc::new(branch2.clone());

        let branch3_path = PathBuf::from("/branch1/branch2/branch3");
        let branch3 = Tree::new(branch3_path.clone(), Some(Arc::downgrade(&branch2_arc)));
        branch3.add_leaf("leaf4".to_string());

        branch2.add_branch(branch3.clone());
        branch1.add_branch(branch2.clone());
        tree.add_branch(branch1.clone());

        let expected = "/\n└─ /branch1\n   └─ /branch2\n      └─ /branch3\n         └─ leaf4";
        let output = tree.to_string();

        assert_eq!(output, expected);
    }

    #[test]
    fn test_tree_with_mixed_branches_and_leaves() {
        let root_path = PathBuf::from("/");
        let tree = Arc::new(Tree::new(root_path.clone(), None));

        tree.add_leaf("leaf1".to_string());
        tree.add_leaf("leaf2".to_string());

        let branch1_path = PathBuf::from("/branch1");
        let branch1 = Tree::new(branch1_path.clone(), Some(Arc::downgrade(&tree)));
        let branch1_arc = Arc::new(branch1.clone());
        branch1.add_leaf("leaf3".to_string());

        let branch2_path = PathBuf::from("/branch2");
        let branch2 = Tree::new(branch2_path.clone(), Some(Arc::downgrade(&tree)));
        branch2.add_leaf("leaf4".to_string());

        let branch3_path = PathBuf::from("/branch1/branch3");
        let branch3 = Tree::new(branch3_path.clone(), Some(Arc::downgrade(&branch1_arc)));
        branch3.add_leaf("leaf5".to_string());

        branch1.add_branch(branch3.clone());
        tree.add_branch(branch1.clone());
        tree.add_branch(branch2.clone());

        let expected = "/\n├─ /branch1\n│  ├─ /branch3\n│  │  └─ leaf5\n│  └─ leaf3\n├─ /branch2\n│  └─ leaf4\n├─ leaf1\n└─ leaf2";
        let output = tree.to_string();

        assert_eq!(output, expected);
    }
}
