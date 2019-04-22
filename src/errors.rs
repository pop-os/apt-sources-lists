use std::io;
use std::path::PathBuf;

/// An error that may occur when parsing apt sources.
#[derive(Debug, Error)]
pub enum SourceError {
    #[error(display = "I/O error occurred: {}", _0)]
    Io(io::Error),
    #[error(display = "missing field in apt source list: '{}'", field)]
    MissingField { field: &'static str },
    #[error(display = "invalid field in apt source list: '{}' is invalid for '{}'", value, field)]
    InvalidValue { field: &'static str, value: String },
    #[error(display = "entry did not exist in sources")]
    EntryNotFound,
    #[error(display = "failed to write changes to {:?}: {}", path, why)]
    EntryWrite { path: PathBuf, why: io::Error },
    #[error(display = "source file was not found")]
    FileNotFound,
    #[error(display = "failed to parse source list at {:?}: {}", path, why)]
    SourcesList { path: PathBuf, why: Box<SourcesListError> },
    #[error(display = "failed to open / read source list at {:?}: {}", path, why)]
    SourcesListOpen { path: PathBuf, why: io::Error },
}

#[derive(Debug, Error)]
pub enum SourcesListError {
    #[error(display = "parsing error on line {}: {}", line, why)]
    BadLine { line: usize, why: SourceError },
}

impl From<io::Error> for SourceError {
    fn from(why: io::Error) -> Self {
        SourceError::Io(why)
    }
}

/// Equivalent to `Result<T, SourceError>`.
pub type SourceResult<T> = Result<T, SourceError>;
