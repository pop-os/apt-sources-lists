use super::*;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Debug, Default)]
pub struct SourcesList {
    pub path: PathBuf,
    pub lines: Vec<SourceLine>,
}

impl FromStr for SourcesList {
    type Err = SourcesListError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut source_list = Self::default();
        for (no, line) in input.lines().enumerate() {
            let entry = line
                .parse::<SourceLine>()
                .map_err(|why| SourcesListError::BadLine { line: no, why })?;

            // Prevent duplicate entries.
            if !source_list.lines.contains(&entry) {
                source_list.lines.push(entry);
            }
        }

        Ok(source_list)
    }
}

impl SourcesList {
    pub fn new<P: AsRef<Path>>(path: P) -> SourceResult<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .map_err(|why| SourceError::SourcesListOpen { path: path.to_path_buf(), why })?;
        let mut sources_file = data.parse::<SourcesList>().map_err(|why| {
            SourceError::SourcesList { path: path.to_path_buf(), why: Box::new(why) }
        })?;

        sources_file.path = path.to_path_buf();
        Ok(sources_file)
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

    pub fn get_entry_mut(&mut self, entry: &str) -> Option<&mut SourceEntry> {
        self.lines
            .iter_mut()
            .filter_map(|line| {
                if let SourceLine::Entry(ref mut e) = line {
                    if entry == e.url {
                        return Some(e);
                    }
                }

                None
            })
            .next()
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

    pub fn reload(&mut self) -> SourceResult<()> {
        *self = Self::new(&self.path)?;
        Ok(())
    }
}

impl Display for SourcesList {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        for line in &self.lines {
            writeln!(fmt, "{}", line)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Stores all apt source information fetched from the system.
pub struct SourcesLists {
    pub(crate) files: Vec<SourcesList>,
    pub(crate) modified: Vec<u16>,
}

impl Deref for SourcesLists {
    type Target = Vec<SourcesList>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for SourcesLists {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
}

impl SourcesLists {
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

        Self::new_from_paths(paths.iter())
    }

    /// When given a list of paths to source lists, this will attempt to parse them.
    pub fn new_from_paths<P: AsRef<Path>, I: Iterator<Item = P>>(paths: I) -> SourceResult<Self> {
        let files = paths.map(SourcesList::new).collect::<SourceResult<Vec<SourcesList>>>()?;

        Ok(SourcesLists { modified: Vec::with_capacity(files.len()), files })
    }

    /// Specify to enable or disable a repo. `true` is returned if the repo was found.
    pub fn repo_modify(&mut self, repo: &str, enabled: bool) -> bool {
        let &mut Self { ref mut modified, ref mut files } = self;

        files
            .iter_mut()
            .enumerate()
            .filter_map(|(pos, list)| list.get_entry_mut(repo).map(|e| (pos, e)))
            .next()
            .map_or(false, |(pos, entry)| {
                add_modified(modified, pos as u16);
                entry.enabled = enabled;
                true
            })
    }

    /// Constructs an iterator of enabled source entries from a sources list.
    pub fn entries(&self) -> impl Iterator<Item = &SourceEntry> {
        self.iter().flat_map(|list| list.lines.iter()).filter_map(move |entry| {
            if let SourceLine::Entry(entry) = entry {
                return Some(entry);
            }

            None
        })
    }

    /// Insert a source entry to the lists.
    ///
    /// If the entry already exists, it will be modified.
    /// Otherwise, the entry will be added to the preferred list.
    /// If the preferred list does not exist, it will be created.
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

        files.push(SourcesList { path: path.to_path_buf(), lines: vec![SourceLine::Entry(entry)] });

        Ok(())
    }

    /// Remove the source entry from each file in the sources lists.
    pub fn remove_entry(&mut self, repo: &str) {
        let &mut Self { ref mut modified, ref mut files } = self;
        for (id, list) in files.iter_mut().enumerate() {
            if let Some(line) = list.contains_entry(repo) {
                list.lines.remove(line);
                add_modified(modified, id as u16);
            }
        }
    }

    /// Modify all sources with the `from_suite` to point to the `to_suite`.
    ///
    /// Changes are only applied in-memory. Use `SourcesLists::wirte_sync` to write
    /// all changes to the disk.
    pub fn dist_replace(&mut self, from_suite: &str, to_suite: &str) {
        let &mut Self { ref mut modified, ref mut files } = self;
        for (id, file) in files.iter_mut().enumerate() {
            let mut changed = false;
            for line in &mut file.lines {
                if let SourceLine::Entry(ref mut entry) = line {
                    if entry.suite == from_suite {
                        entry.suite = to_suite.to_owned();
                        changed = true;
                    }
                }
            }

            if changed {
                add_modified(modified, id as u16);
            }
        }
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
            sources: &mut SourcesLists,
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
        self.entries().filter_map(move |entry| {
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
