use super::*;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::mem;
use std::ops::{Deref, DerefMut};
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

            // Prevent duplicate entries.
            if !source_list.lines.contains(&entry) {
                source_list.lines.push(entry);
            }
        }

        source_list.path = path.to_path_buf();
        Ok(source_list)
    }

    pub fn contains_entry(&self, entry: &str) -> Option<usize> {
        self.lines.iter().position(|line| {
            if let SourceLine::Entry(e) = line {
                entry == e.url
            } else {
                false
            }
        })
    }

    pub fn is_active(&self) -> bool {
        self.lines.iter().any(|line| if let SourceLine::Entry(_) = line { true } else { false })
    }

    pub fn write_sync(&mut self) -> io::Result<()> {
        fs::OpenOptions::new()
            .truncate(true)
            .write(true)
            .open(&self.path)
            .and_then(|mut file| writeln!(&mut file, "{}", self))
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
pub struct SourcesList {
    files: Vec<SourcesFile>,
    modified: Vec<u16>,
}

impl Deref for SourcesList {
    type Target = Vec<SourcesFile>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for SourcesList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
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

        let files = paths
            .iter()
            .map(SourcesFile::new)
            .collect::<io::Result<Vec<SourcesFile>>>()
            .map_err(SourceError::Io)?;

        Ok(SourcesList { modified: Vec::with_capacity(files.len()), files })
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
        self.iter().filter_map(move |list| list.contains_entry(entry).map(|p| (p, list)))
    }

    /// Determine if the given entry repo is in a sources list, then return each repo where it was found.
    pub fn lists_which_contain_mut<'a>(
        &'a mut self,
        entry: &'a str,
    ) -> impl Iterator<Item = (usize, &'a mut SourcesFile)> {
        self.iter_mut().filter_map(move |list| list.contains_entry(entry).map(|p| (p, list)))
    }

    /// Insert new source entries to the list.
    pub fn insert_entry<P: AsRef<Path>>(
        &mut self,
        path: P,
        entry: SourceEntry,
    ) -> SourceResult<()> {
        let path = path.as_ref();
        let &mut Self { ref mut modified, ref mut files } = self;

        for (id, list) in files.iter_mut().enumerate() {
            if list.path == path {
                match list.contains_entry(&entry.url) {
                    Some(pos) => list.lines[pos] = SourceLine::Entry(entry),
                    None => list.lines.push(SourceLine::Entry(entry)),
                }

                add_modified(modified, id as u16);
                return Ok(());
            }
        }

        Err(SourceError::FileNotFound)
    }

    /// Remove the source entry from each file in the sources lists.
    pub fn remove_entry(&mut self, repo: &str) -> SourceResult<()> {
        let &mut Self { ref mut modified, ref mut files } = self;
        for (id, list) in files.iter_mut().enumerate() {
            if let Some(line) = list.contains_entry(repo) {
                list.lines.remove(line);
                add_modified(modified, id as u16);
            }
        }

        Ok(())
    }

    /// Instead of removing it, comment it.
    pub fn comment_entry(&mut self, repo: &str) -> SourceResult<()> {
        let &mut Self { ref mut modified, ref mut files } = self;
        for (id, list) in files.iter_mut().enumerate() {
            if let Some(line) = list.contains_entry(repo) {
                let mut v = SourceLine::Empty;
                add_modified(modified, id as u16);
                mem::swap(&mut v, &mut list.lines[line]);

                if let SourceLine::Entry(e) = v {
                    list.lines[line] = SourceLine::Comment(format!("# {}", e));
                }
            }
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

    /// Overwrite all files which were modified.
    pub fn write_sync(&mut self) -> io::Result<()> {
        let &mut Self { ref mut modified, ref mut files } = self;
        modified.drain(..).map(|id| files[id as usize].write_sync()).collect()
    }
}

fn add_modified(modified: &mut Vec<u16>, list: u16) {
    if !modified.iter().any(|&v| v == list) {
        modified.push(list);
    }
}
