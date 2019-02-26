use super::*;
use std::fmt;
use std::str::FromStr;

/// A line from an apt source list.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceLine {
    Comment(String),
    Empty,
    Entry(SourceEntry),
}

impl fmt::Display for SourceLine {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SourceLine::Comment(ref comment) => write!(fmt, "{}", comment),
            SourceLine::Empty => Ok(()),
            SourceLine::Entry(ref entry) => write!(fmt, "{}", entry),
        }
    }
}

impl FromStr for SourceLine {
    type Err = SourceError;
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let line = line.trim();
        if line.starts_with('#') {
            Ok(SourceLine::Comment(line.into()))
        } else if line.is_empty() {
            Ok(SourceLine::Empty)
        } else {
            Ok(SourceLine::Entry(line.parse::<SourceEntry>()?))
        }
    }
}
