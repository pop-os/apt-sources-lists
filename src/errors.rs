use std::io;

/// An error that may occur when parsing apt sources.
#[derive(Debug, Fail)]
pub enum SourceError {
    #[fail(display = "I/O error occurred: {}", why)]
    IO { why: io::Error },
    #[fail(display = "missing field in apt source list: '{}'", field)]
    MissingField { field: &'static str },
    #[fail(display = "invalid field in aopt source list: '{}' is invalid for '{}'", value, field)]
    InvalidValue { field: &'static str, value: String },
}

impl From<io::Error> for SourceError {
    fn from(why: io::Error) -> Self {
        SourceError::IO { why }
    }
}

/// Equivalent to `Result<T, SourceError>`.
pub type SourceResult<T> = Result<T, SourceError>;
