use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Cunw error: {source}")]
#[diagnostic(code(cunw::error))]
pub struct CunwError {
    #[source]
    #[diagnostic_source]
    pub source: CunwErrorKind,
    pub related_to_file: Option<PathBuf>,
}

impl CunwError {
    pub fn new(source: CunwErrorKind) -> Self {
        Self {
            source,
            related_to_file: None,
        }
    }
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.related_to_file = Some(file);
        self
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum CunwErrorKind {
    #[error("IO error: {0}")]
    #[diagnostic(code(cunw::io_error))]
    Io(#[from] std::io::Error),

    #[error("Failed to build codebase: {0}")]
    #[diagnostic(code(cunw::codebase_build_error))]
    CodebaseBuild(String),

    #[error("Failed to build gitignore: {0}")]
    #[diagnostic(code(cunw::gitignore_build_error))]
    GitignoreBuild(#[from] ignore::Error),
}

pub type Result<T> = std::result::Result<T, CunwError>;
