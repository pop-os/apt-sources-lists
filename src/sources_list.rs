use super::*;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};

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
