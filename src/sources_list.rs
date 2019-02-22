use super::*;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::mem;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct SourcesFile {
    pub path: PathBuf,
    pub lines: Vec<SourceLine>,
}

impl SourcesFile {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut source_list = Self::default();

        let file = File::open(path).map_err(|why| {
            io::Error::new(why.kind(), format!("failed to open {}: {}", path.display(), why))
        })?;

        for (no, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|why| {
                io::Error::new(
                    why.kind(),
                    format!("error reading line {} in {}: {}", no, path.display(), why),
                )
            })?;

            let entry = SourceLine::parse_line(line.as_str()).map_err(|why| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "error parsing line {} ({}) in {}: {}",
                        line.as_str(),
                        no,
                        path.display(),
                        why
                    ),
                )
            })?;

            source_list.lines.push(entry);
        }

        source_list.path = path.to_path_buf();
        Ok(source_list)
    }

    pub fn is_active(&self) -> bool {
        self.lines.iter().any(|line| if let SourceLine::Entry(_) = line { true } else { false })
    }

    pub fn write_sync(&mut self) -> io::Result<()> {
        fs::write(&self.path, format!("{}", self))
    }

    pub fn reload(&mut self) -> io::Result<()> {
        *self = Self::new(&self.path)?;
        Ok(())
    }
}

impl Display for SourcesFile {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        for line in &self.lines {
            writeln!(fmt, "{}", line)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Stores all apt source information fetched from the system.
pub struct SourcesList(Vec<SourcesFile>);

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

        let lists = paths
            .iter()
            .map(SourcesFile::new)
            .collect::<io::Result<Vec<SourcesFile>>>()
            .map_err(SourceError::Io)?;

        Ok(SourcesList(lists))
    }

    /// Constructs an iterator of source entries from a sources list.
    pub fn dist_paths(&self) -> impl Iterator<Item = &SourceEntry> {
        self.iter().flat_map(|list| list.lines.iter()).filter_map(move |entry| {
            if let SourceLine::Entry(entry) = entry {
                return Some(entry);
            }

            None
        })
    }

    /// Determine if the given entry repo is in a sources list, then return each repo where it was found.
    pub fn lists_which_contain<'a>(
        &'a self,
        entry: &'a str,
    ) -> impl Iterator<Item = (usize, &'a SourcesFile)> {
        self.iter().enumerate().filter(move |(_, list)| {
            list.lines.iter().any(|line| {
                if let SourceLine::Entry(e) = line {
                    entry == e.url
                } else {
                    false
                }
            })
        })
    }

    /// Determine if the given entry repo is in a sources list, then return each repo where it was found.
    pub fn lists_which_contain_mut<'a>(
        &'a mut self,
        entry: &'a str,
    ) -> impl Iterator<Item = (usize, &'a mut SourcesFile)> {
        self.iter_mut().enumerate().filter(move |(_, list)| {
            list.lines.iter().any(|line| {
                if let SourceLine::Entry(e) = line {
                    entry == e.url
                } else {
                    false
                }
            })
        })
    }

    /// Insert new source entries to the list.
    pub fn insert_entry<P: AsRef<Path>>(
        &mut self,
        path: P,
        entry: &SourceEntry,
    ) -> SourceResult<()> {
        let path = path.as_ref();

        for list in self.iter_mut() {
            if list.path == path {
                list.lines.push(SourceLine::Entry(entry.clone()));
                return list
                    .write_sync()
                    .map_err(|why| SourceError::EntryWrite { path: list.path.clone(), why });
            }
        }

        Err(SourceError::FileNotFound)
    }

    /// Remove source entry from the lists
    pub fn remove_entry(&mut self, repo: &str) -> SourceResult<()> {
        for (line, entry) in self.lists_which_contain_mut(repo) {
            entry.lines.remove(line);
            entry
                .write_sync()
                .map_err(|why| SourceError::EntryWrite { path: entry.path.clone(), why })?;
        }

        Ok(())
    }

    /// Instead of removing it, comment it.
    pub fn comment_entry(&mut self, repo: &str) -> SourceResult<()> {
        for (line, entry) in self.lists_which_contain_mut(repo) {
            let mut v = SourceLine::Empty;
            mem::swap(&mut v, &mut entry.lines[line]);

            if let SourceLine::Entry(e) = v {
                entry.lines[line] = SourceLine::Comment(format!("# {}", e));
            }

            entry
                .write_sync()
                .map_err(|why| SourceError::EntryWrite { path: entry.path.clone(), why })?;
        }

        Ok(())
    }

    /// Upgrade entries so that they point to a new release.
    ///
    /// Files are copied to "$path.save" before being overwritten. On failure, these backup files
    /// will be used to restore the original list.
    pub fn dist_upgrade(&mut self, from_suite: &str, to_suite: &str) -> io::Result<()> {
        fn newfile(modified: &mut Vec<PathBuf>, path: &Path) -> io::Result<File> {
            let backup_path = path
                .file_name()
                .map(|str| {
                    let mut string = str.to_os_string();
                    string.push(".save");

                    let mut backup = path.to_path_buf();
                    backup.set_file_name(&string);
                    backup
                })
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("filename not found for apt source at '{}'", path.display()),
                    )
                })?;

            fs::copy(path, &backup_path)?;
            modified.push(backup_path);
            fs::OpenOptions::new().truncate(true).write(true).open(path)
        }

        fn apply(
            sources: &mut SourcesList,
            modified: &mut Vec<PathBuf>,
            from_suite: &str,
            to_suite: &str,
        ) -> io::Result<()> {
            for list in sources.iter_mut() {
                let mut current_file = newfile(modified, &list.path)?;

                for line in list.lines.iter_mut() {
                    if let SourceLine::Entry(entry) = line {
                        if entry.url.starts_with("http") && entry.suite.starts_with(from_suite) {
                            entry.suite = entry.suite.replace(from_suite, to_suite);;
                        }
                    }

                    writeln!(&mut current_file, "{}", line)?
                }

                current_file.flush()?;
            }

            Ok(())
        }

        let mut modified = Vec::new();
        apply(self, &mut modified, from_suite, to_suite).map_err(|why| {
            // TODO: Revert the ipathsn-memory changes that were made when being applied.
            // revert(self, &modified);

            for (original, backup) in self.iter().zip(modified.iter()) {
                if let Err(why) = fs::copy(backup, &original.path) {
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
        to_suite: &'a str,
    ) -> impl Iterator<Item = String> + 'a {
        self.dist_paths().filter_map(move |entry| {
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

    pub fn into_iter(self) -> impl Iterator<Item = SourcesFile> {
        self.0.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SourcesFile> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut SourcesFile> {
        self.0.iter_mut()
    }
}
