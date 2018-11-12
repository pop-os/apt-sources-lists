use super::*;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};

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

    /// Constructs an iterator of source entries from a sources list.
    pub fn dist_paths(&self) -> impl Iterator<Item = &SourceEntry> {
        self.into_iter()
            .filter_map(move |entry| {
                if let SourceEvent::Entry(SourceLine::Entry(entry)) = entry {
                    return Some(entry);
                }

                None
            })
    }

    /// Upgrade entries so that they point to a new release.
    ///
    /// Files are copied to "$path.save" before being overwritten. On failure, these backup files
    /// will be used to restore the original list.
    pub fn dist_upgrade(&mut self, from_suite: &str, to_suite: &str) -> io::Result<()> {
        fn newfile(modified: &mut Vec<PathBuf>, path: &Path) -> io::Result<File> {
            let backup_path = path.file_name()
                .map(|str| {
                    let mut string = str.to_os_string();
                    string.push(".save");

                    let mut backup = path.to_path_buf();
                    backup.set_file_name(&string);
                    backup
                })
                .ok_or_else(|| io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("filename not found for apt source at '{}'", path.display())
                ))?;

            fs::copy(path, &backup_path)?;
            modified.push(backup_path);
            fs::OpenOptions::new().truncate(true).write(true).open(path)
        }

        fn apply(
            sources: &mut SourcesList,
            modified: &mut Vec<PathBuf>,
            from_suite: &str,
            to_suite: &str
        ) -> io::Result<()> {
            let mut iterator = sources.iter_mut();
            let mut current_file = match iterator.next() {
                Some(SourceEventMut::NewList(path)) => newfile(modified, &path)?,
                Some(_) => unreachable!("expected first input to be a path"),
                None => return Ok(())
            };

            for entry in iterator {
                match entry {
                    SourceEventMut::NewList(path) => {
                        current_file.flush()?;
                        current_file = newfile(modified, &path)?;
                    },
                    SourceEventMut::Entry(line) => {
                        if let SourceLine::Entry(entry) = line {
                            if entry.url.starts_with("http") && entry.suite.starts_with(from_suite) {
                                entry.suite = entry.suite.replace(from_suite, to_suite);;
                            }
                        }

                        writeln!(&mut current_file, "{}", line)?
                    }
                }
            }

            Ok(())
        }

        let mut modified = Vec::new();
        apply(self, &mut modified, from_suite, to_suite)
            .map_err(|why| {
                // TODO: Revert the in-memory changes that were made when being applied.
                // revert(self, &modified);
    
                for (original, backup) in self.paths.iter().zip(modified.iter()) {
                    if let Err(why) = fs::copy(backup, original) {
                        eprintln!("failed to restore backup of {:?}: {}", backup, why);
                    }
                }

                why
            })
    }

    /// Retrieve an iterator of upgradeable paths.
    ///
    /// All source entries that have the `from_suite` will have new URLs constructed with the
    /// `to_suite`.
    pub fn dist_upgrade_paths<'a>(
        &'a self,
        from_suite: &'a str,
        to_suite: &'a str
    ) -> impl Iterator<Item = String> + 'a {
        self.dist_paths()
            .filter_map(move |entry| {
                if entry.url.starts_with("http") && entry.suite.starts_with(from_suite) {
                    let entry = {
                        let mut entry = entry.clone();
                        entry.suite = entry.suite.replace(from_suite, to_suite);
                        entry
                    };

                    let dist_path = entry.dist_path();
                    Some(dist_path)
                } else {
                    None
                }
            })
    }

    /// Returns an iterator over immutable entries.
    pub fn iter(&self) -> SourcesIter<'_> {
        self.into_iter()
    }

    /// Returns an iterator over mutable entries.
    pub fn iter_mut(&mut self) -> SourcesIterMut<'_> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a SourcesList {
    type Item = SourceEvent<'a>;
    type IntoIter = SourcesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SourcesIter::new(self)
    }
}

#[derive(Debug)]
pub enum SourceEvent<'a> {
    NewList(&'a Path),
    Entry(&'a SourceLine)
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

impl<'a> IntoIterator for &'a mut SourcesList {
    type Item = SourceEventMut<'a>;
    type IntoIter = SourcesIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SourcesIterMut::new(self)
    }
}

#[derive(Debug)]
pub enum SourceEventMut<'a> {
    NewList(&'a mut PathBuf),
    Entry(&'a mut SourceLine)
}

/// An iterator of active apt source entries.
pub struct SourcesIterMut<'a> {
    position: u32,
    list: &'a mut SourcesList,
    last_path: u32,
    started: bool,
}

impl<'a> SourcesIterMut<'a> {
    pub fn new(list: &'a mut SourcesList) -> Self {
        Self { position: 0, list, last_path: 0, started: false }
    }
}

impl<'a> Iterator for SourcesIterMut<'a> {
    type Item = SourceEventMut<'a>;

    fn next(&mut self) -> Option<SourceEventMut<'a>> {
        // TODO: avoid unsafe usage here.
        let (paths, entries) = match (self.list.paths.get_mut(0), self.list.entries.get_mut(0)) {
            (Some(path), Some(entry)) => (path as *mut PathBuf, entry as *mut SourceLine),
            _ => return None
        };

        if self.list.entries.len() as u32 > self.position {
            let new_path = self.list.origins[self.position as usize];
            if ! self.started || new_path != self.last_path {
                self.started = true;
                self.last_path = new_path;
                let path: &mut PathBuf = unsafe {
                    &mut *paths.offset(new_path as isize)
                };
                Some(SourceEventMut::NewList(path))
            } else {
                self.position += 1;
                let entry: &mut SourceLine = unsafe {
                    &mut *entries.offset(self.position as isize - 1)
                };
                Some(SourceEventMut::Entry(entry))
            }
        } else {
            None
        }
    }
}
