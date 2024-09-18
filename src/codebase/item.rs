use std::{
    fmt::Display,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use tokio::{fs, task::JoinHandle};

use crate::error::{CunwError, Result};

#[derive(Debug, Clone)]
pub struct CodebaseItem {
    pub path: PathBuf,
    pub content: Arc<OnceLock<String>>,
}

impl CodebaseItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            content: Arc::new(OnceLock::new()),
        }
    }
    pub fn eventually_load_content(&self) -> JoinHandle<Result<()>> {
        let _content = self.content.clone();
        let _path = self.path.clone();
        tokio::spawn(async move {
            let path = _path;
            if let None = _content.get() {
                let file_content = fs::read_to_string(&path)
                    .await
                    .map_err(|e| CunwError::new(e.into()).with_file(path.clone()))?;
                _content.get_or_init(|| file_content);
            }
            Ok(())
        })
    }
}

impl PartialEq for CodebaseItem {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Display for CodebaseItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the file name (or directory name) instead of the full path.
        write!(f, "{}", self.path.file_name().unwrap().to_str().unwrap())
    }
}
