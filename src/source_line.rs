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
            let inner = line[1..].trim();
            let entry = if !inner.is_empty() { line.parse::<SourceEntry>().ok() } else { None };

            Ok(entry.map_or_else(
                || SourceLine::Comment(line.into()),
                |mut entry| {
                    entry.enabled = false;
                    SourceLine::Entry(entry)
                },
            ))
        } else if line.is_empty() {
            Ok(SourceLine::Empty)
        } else {
            Ok(SourceLine::Entry(line.parse::<SourceEntry>()?))
        }
    }
}
