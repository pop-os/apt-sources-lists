use super::*;
use std::fmt;

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

impl SourceLine {
    /// Parses a single line within an apt source list file.
    pub fn parse_line(line: &str) -> SourceResult<Self> {
        let line = line.trim();
        if line.starts_with('#') {
            Ok(SourceLine::Comment(line.into()))
        } else if line.is_empty() {
            Ok(SourceLine::Empty)
        } else {
            Ok(SourceLine::Entry(SourceEntry::parse_line(line)?))
        }
    }
}
