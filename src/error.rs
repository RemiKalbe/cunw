use log::Level as LogLevel;
use std::backtrace::Backtrace;

#[derive(Debug, Clone)]
pub enum Crate {
    Clap,
    Crossterm,
    IgnoreFiles,
    Indicatif,
    Tokio,
    Wax,
}

impl std::fmt::Display for Crate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Crate::Clap => write!(f, "clap"),
            Crate::Crossterm => write!(f, "crossterm"),
            Crate::IgnoreFiles => write!(f, "ignore_files"),
            Crate::Indicatif => write!(f, "indicatif"),
            Crate::Tokio => write!(f, "tokio"),
            Crate::Wax => write!(f, "wax"),
        }
    }
}

#[derive(Debug)]
pub struct ErrorContext {
    pub message_title: Option<String>,
    pub message: Option<String>,
    pub helper: Option<String>,
    pub from_crate: Option<Crate>,
    pub level: LogLevel,
}

#[derive(Debug)]
pub struct ErrorContextStringParts {
    pub message_title: String,
    pub message: String,
    pub helper: String,
    pub from_crate: String,
    pub level: String,
}

impl ErrorContext {
    pub fn new(level: LogLevel) -> Self {
        Self {
            message_title: None,
            message: None,
            helper: None,
            from_crate: None,
            level,
        }
    }
    pub fn with_message_title(mut self, message_title: String) -> Self {
        self.message_title = Some(message_title);
        self
    }
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
    pub fn with_helper(mut self, helper: String) -> Self {
        self.helper = Some(helper);
        self
    }
    pub fn with_from_crate(mut self, from_crate: Crate) -> Self {
        self.from_crate = Some(from_crate);
        self
    }
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ErrorContext {{ message_title: {:?}, message: {:?}, helper: {:?}, from_crate: {:?}, level: {:?} }}",
            self.message_title, self.message, self.helper, self.from_crate, self.level
        )
    }
}

#[derive(Debug)]
pub enum Error {
    ArgError {
        inner: clap::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    ArgPatternError {
        inner: globset::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    IgnoreFilePatternError {
        inner: ignore_files::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    WalkError {
        inner: walkdir::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    IOError {
        inner: std::io::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    RuntimeError {
        inner: std::io::Error,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
    UnknownError {
        inner: String,
        backtrace: Backtrace,
        context: Option<ErrorContext>,
    },
}

impl Error {
    pub fn with_context(mut self, c: ErrorContext) -> Self {
        match self {
            Self::ArgError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::IOError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::RuntimeError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::ArgPatternError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::IgnoreFilePatternError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::UnknownError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
            Self::WalkError {
                ref mut context, ..
            } => {
                context.replace(c);
            }
        }
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

// Convertions

impl From<clap::Error> for Error {
    fn from(error: clap::Error) -> Self {
        Self::ArgError {
            inner: error,
            backtrace: Backtrace::capture(),
            context: None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IOError {
            inner: error,
            backtrace: Backtrace::capture(),
            context: None,
        }
    }
}

impl From<ignore_files::Error> for Error {
    fn from(e: ignore_files::Error) -> Self {
        match e {
            ignore_files::Error::Read { err, .. } => Self::IOError {
                inner: err,
                backtrace: Backtrace::capture(),
                context: None,
            },
            ignore_files::Error::Glob { .. } => Self::IgnoreFilePatternError {
                inner: e,
                backtrace: Backtrace::capture(),
                context: None,
            },
            ignore_files::Error::Multi(_) => Self::UnknownError {
                inner: e.to_string(),
                backtrace: Backtrace::capture(),
                context: None,
            },
            ignore_files::Error::Canonicalize { err, .. } => Self::IOError {
                inner: err,
                backtrace: Backtrace::capture(),
                context: None,
            },
            _ => Self::UnknownError {
                inner: e.to_string(),
                backtrace: Backtrace::capture(),
                context: None,
            },
        }
    }
}

impl From<globset::Error> for Error {
    fn from(e: globset::Error) -> Self {
        Self::ArgPatternError {
            inner: e,
            backtrace: Backtrace::capture(),
            context: None,
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(e: walkdir::Error) -> Self {
        Self::WalkError {
            inner: e,
            backtrace: Backtrace::capture(),
            context: None,
        }
    }
}
