//! Crate for fetching detailed information from all available apt sources.
//!
//! The information retrieved from the provided `SourcesList` and accompanying iterator preserves
//! newlines and comments, so that these files can be modified and overwritten to preserve this data.
//!
//! Active source entries will be parsed into `SourceEntry`'s, which can be handled or serialized
//! back into text. Formatting of these lines are not preserved.

extern crate failure;
#[macro_use]
extern crate failure_derive;

use std::path::{Path, PathBuf};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};

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

/// Stores all apt source information fetched from the system.
pub struct SourcesList {
    /// Entries that were collected from the apt sources list files.
    entries: Vec<SourceLine>,
    /// Stores tickets to the paths in the paths field.
    origins: Vec<u32>,
    /// The files that were scanned when search for repositories.
    paths: Vec<PathBuf>,
}

impl SourcesList {
    /// Scans every file in **/etc/apt/sources.list.d**, including **/etc/apt/sources.list**.
    ///
    /// Note that this will parse every source list into memory before returning.
    pub fn scan() -> SourceResult<Self> {
        let mut paths = vec![PathBuf::from("/etc/apt/sources.list")];

        for entry in fs::read_dir("/etc/apt/sources.list.d/")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "list") {
                paths.push(path);
            }
        }

        let mut entries = Vec::new();
        let mut origins = Vec::new();

        for (id, path) in paths.iter().enumerate() {
            let mut file = File::open(path).map_err(|why| io::Error::new(
                why.kind(),
                format!("failed to open {}: {}", path.display(), why)
            ))?;

            for (no, line) in BufReader::new(file).lines().enumerate() {
                let line = line.map_err(|why| io::Error::new(
                    why.kind(),
                    format!("error reading line {} in {}: {}", no, path.display(), why)
                ))?;

                let entry = SourceLine::parse_line(line.as_str()).map_err(|why| io::Error::new(
                    io::ErrorKind::Other,
                    format!("error parsing line {} in {}: {}", no, path.display(), why)
                ))?;

                entries.push(entry);
                origins.push(id as u32);
            }
        }

        Ok(SourcesList { entries, origins, paths })
    }
}

#[derive(Debug)]
pub enum SourceEvent<'a> {
    NewList(&'a Path),
    Entry(&'a SourceLine)
}

impl<'a> IntoIterator for &'a SourcesList {
    type Item = SourceEvent<'a>;
    type IntoIter = SourcesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SourcesIter::new(self)
    }
}

/// An iterator of active apt source entries.
pub struct SourcesIter<'a> {
    position: u32,
    list: &'a SourcesList,
    last_path: u32,
    started: bool,
}

impl<'a> SourcesIter<'a> {
    pub fn new(list: &'a SourcesList) -> Self {
        Self { position: 0, list, last_path: 0, started: false }
    }
}

impl<'a> Iterator for SourcesIter<'a> {
    type Item = SourceEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.list.entries.len() as u32 > self.position {
            let new_path = self.list.origins[self.position as usize];
            if ! self.started || new_path != self.last_path {
                self.started = true;
                self.last_path = new_path;
                return Some(SourceEvent::NewList(&self.list.paths[new_path as usize]));
            }

            self.position += 1;
            Some(SourceEvent::Entry(&self.list.entries[self.position as usize - 1]))
        } else {
            None
        }
    }
}

/// An apt source entry that is active on the system.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceEntry {
    /// Whether this is a binary or source repo.
    pub source: bool,
    /// Some repos may have special options defined.
    pub options: Option<String>,
    /// The URL of the repo.
    pub url: String,
    /// The suite of the repo would be as `bionic` or `cosmic`.
    pub suite: String,
    /// Components that have been enabled for this repo.
    pub components: Vec<String>,
}

impl SourceEntry {
    pub fn parse_line(line: &str) -> SourceResult<Self> {
        let mut components = Vec::new();
        let mut options = None;
        let url;

        let mut fields = line.split_whitespace();

        let source = match fields.next().ok_or(SourceError::MissingField { field: "source" })? {
            "deb" => false,
            "deb-src" => true,
            other => return Err(SourceError::InvalidValue { field: "source", value: other.to_owned() })
        };

        let field = fields.next().ok_or(SourceError::MissingField { field: "url" })?;
        if field.starts_with('[') {
            let mut leftover: Option<String> = None;
            let mut field: String = field[1..].into();

            if let Some(pos) = field.find(']') {
                if pos == field.len() - 1 {
                    options = Some(field);
                } else {
                    options = Some(field[..pos].into());
                    leftover = Some(field[pos+1..].into());
                }
            } else {
                loop {
                    let next = fields.next().ok_or(SourceError::MissingField { field: "option" })?;
                    if let Some(pos) = next.find(']') {
                        field.push_str(&next[..pos]);
                        if pos != next.len() - 1 {
                            leftover = Some(next[pos+1..].into());
                        }
                        break
                    } else {
                        field.push_str(next);
                    }
                }

                options = Some(field);
            }

            url = match leftover {
                Some(field) => field,
                None => fields.next().ok_or(SourceError::MissingField { field: "url" })?.into()
            };
        } else {
            url = field.into();
        }

        let suite = fields.next().ok_or(SourceError::MissingField { field: "suite" })?.into();
        components.push(fields.next().ok_or(SourceError::MissingField { field: "component" })?.into());
        for field in fields {
            components.push(field.into());
        }

        Ok(SourceEntry {
            source,
            url,
            suite,
            components,
            options
        })
    }
}

impl fmt::Display for SourceEntry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(if self.source { "deb-src " } else { "deb " })?;
        if let Some(ref options) = self.options.as_ref() {
            write!(fmt, "[{}] ", options)?;
        }

        write!(fmt, "{} {} {}", self.url, self.suite, self.components.join(" "))
    }
}

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

#[cfg(test)]
mod tests {
    pub use super::*;

    #[test]
    fn source_entry() {
        assert_eq!(
            SourceLine::parse_line(
                "deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main \
                restricted universe multiverse"
            ).unwrap(),
            SourceLine::Entry(SourceEntry {
                source: true,
                url: "http://us.archive.ubuntu.com/ubuntu/".into(),
                suite: "cosmic".into(),
                options: None,
                components: vec![
                    "main".into(),
                    "restricted".into(),
                    "universe".into(),
                    "multiverse".into(),
                ]
            })
        );

        assert_eq!(
            SourceLine::parse_line(
                "deb http://us.archive.ubuntu.com/ubuntu/ cosmic main \
                restricted universe multiverse"
            ).unwrap(),
            SourceLine::Entry(SourceEntry {
                source: false,
                url: "http://us.archive.ubuntu.com/ubuntu/".into(),
                suite: "cosmic".into(),
                options: None,
                components: vec![
                    "main".into(),
                    "restricted".into(),
                    "universe".into(),
                    "multiverse".into(),
                ]
            })
        );

        let comment = "# deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main \
            restricted universe multiverse";
        assert_eq!(
            SourceLine::parse_line(comment).unwrap(),
            SourceLine::Comment(comment.into())
        );

        assert_eq!(
            SourceLine::parse_line("").unwrap(),
            SourceLine::Empty
        );

        let options = [
            "deb [ arch=amd64 ] http://apt.pop-os.org/proprietary cosmic main",
            "deb [arch=amd64 ] http://apt.pop-os.org/proprietary cosmic main",
            "deb [ arch=amd64] http://apt.pop-os.org/proprietary cosmic main",
            "deb [arch=amd64]http://apt.pop-os.org/proprietary cosmic main",
            "deb [ arch=amd64 ]http://apt.pop-os.org/proprietary cosmic main"
        ];

        for source in &options {
            eprintln!("testing {}", source);
            assert_eq!(
                SourceLine::parse_line(source).unwrap(),
                SourceLine::Entry(SourceEntry {
                    source: false,
                    url: "http://apt.pop-os.org/proprietary".into(),
                    suite: "cosmic".into(),
                    options: Some("arch=amd64".into()),
                    components: vec!["main".into()]
                })
            )
        }
    }
}
