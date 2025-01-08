use self::source_deb822::SourceListDeb822;

use super::*;
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct SourcesList {
    pub path: PathBuf,
    pub entries: SourceListType,
}

#[derive(PartialEq, Clone, Debug)]
pub enum SourceListType {
    SourceLine(SourceListLineStyle),
    Deb822(SourceListDeb822),
}

#[derive(PartialEq, Clone, Debug)]
pub struct SourceListLineStyle(pub Vec<SourceLine>);

impl FromStr for SourceListLineStyle {
    type Err = SourcesListError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut entries = vec![];
        for (line_num, line) in s.lines().enumerate() {
            let entry = line
                .parse::<SourceLine>()
                .map_err(|why| SourcesListError::BadLine {
                    line: line_num,
                    why,
                })?;

            entries.push(entry);
        }

        Ok(SourceListLineStyle(entries))
    }
}

impl SourcesList {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, SourcesListError> {
        let path = path.as_ref();
        let data = fs::read_to_string(path).map_err(|why| SourcesListError::SourcesListOpen {
            path: path.to_path_buf(),
            why,
        })?;

        let mut sources_file = match path.extension() {
            Some(x) if x == "sources" => SourcesList {
                path: path.to_path_buf(),
                entries: SourceListType::Deb822(SourceListDeb822::from_str(&data).map_err(
                    |e| SourcesListError::Deb822 {
                        path: path.to_path_buf(),
                        why: e,
                    },
                )?),
            },
            Some(x) if x == "list" => get_line_style_sources_list(&data)?,
            _ => {
                return Err(SourcesListError::UnknownFile {
                    path: path.to_path_buf(),
                })
            }
        };

        sources_file.path = path.to_path_buf();

        Ok(sources_file)
    }

    pub fn contains_entry(&self, entry: &str) -> Option<usize> {
        let elem = &self.entries;
        match elem {
            SourceListType::SourceLine(lines) => lines.0.iter().position(|e| {
                if let SourceLine::Entry(e) = e {
                    e.url == entry
                } else {
                    false
                }
            }),
            SourceListType::Deb822(e) => e.entries.iter().position(|x| x.url == entry),
        }
    }

    pub fn get_entries_mut<'a>(
        &'a mut self,
        entry: &'a str,
    ) -> Box<dyn Iterator<Item = &'a mut SourceEntry> + 'a> {
        match self.entries {
            SourceListType::SourceLine(ref mut line) => {
                Box::new(line.0.iter_mut().filter_map(move |line| {
                    if let SourceLine::Entry(ref mut e) = line {
                        if entry == e.url {
                            return Some(e);
                        }
                    }

                    None
                }))
            }
            SourceListType::Deb822(ref mut e) => {
                Box::new(e.entries.iter_mut().filter_map(move |e| {
                    if entry == e.url {
                        return Some(e);
                    }

                    None
                }))
            }
        }
    }

    pub fn is_active(&self) -> bool {
        match &self.entries {
            SourceListType::SourceLine(line) => line
                .0
                .iter()
                .any(|line| matches!(line, SourceLine::Entry(_))),
            SourceListType::Deb822(e) => !e.entries.is_empty(),
        }
    }

    pub fn write_sync(&mut self) -> io::Result<()> {
        fs::OpenOptions::new()
            .truncate(true)
            .write(true)
            .open(&self.path)
            .and_then(|mut file| writeln!(&mut file, "{}", self))
    }

    pub fn reload(&mut self) -> Result<(), SourcesListError> {
        *self = Self::new(&self.path)?;
        Ok(())
    }
}

fn get_line_style_sources_list(data: &str) -> Result<SourcesList, SourcesListError> {
    let mut entries = vec![];
    for (line_num, line) in data.lines().enumerate() {
        let entry = line
            .parse::<SourceLine>()
            .map_err(|why| SourcesListError::BadLine {
                line: line_num,
                why,
            })?;

        entries.push(entry);
    }

    Ok(SourcesList {
        path: PathBuf::from(""),
        entries: SourceListType::SourceLine(SourceListLineStyle(entries)),
    })
}

impl Display for SourcesList {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match &self.entries {
            SourceListType::SourceLine(lines) => {
                for line in &lines.0 {
                    writeln!(fmt, "{}", line)?;
                }
            }
            SourceListType::Deb822(e) => {
                for entry in &e.entries {
                    writeln!(fmt, "{}", entry)?;
                }
            }
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
    pub fn scan() -> Result<Self, SourcesListError> {
        scan_inner("/")
    }

    /// Scans every file in **/etc/apt/sources.list.d**, including **/etc/apt/sources.list**. (from root argument)
    ///
    /// Note that this will parse every source list into memory before returning.
    pub fn scan_from_root<P: AsRef<Path>>(root: P) -> Result<Self, SourcesListError> {
        scan_inner(root)
    }

    /// When given a list of paths to source lists, this will attempt to parse them.
    pub fn new_from_paths<P: AsRef<Path>, I: Iterator<Item = P>>(
        paths: I,
    ) -> Result<Self, SourcesListError> {
        let files = paths
            .map(SourcesList::new)
            .collect::<Result<Vec<SourcesList>, SourcesListError>>()?;

        Ok(SourcesLists {
            modified: Vec::with_capacity(files.len()),
            files,
        })
    }

    /// Specify to enable or disable a repo. `true` is returned if the repo was found.
    pub fn repo_modify(&mut self, repo: &str, enabled: bool) -> bool {
        let &mut Self {
            ref mut modified,
            ref mut files,
        } = self;

        let iterator = files
            .iter_mut()
            .enumerate()
            .flat_map(|(pos, list)| list.get_entries_mut(repo).map(move |e| (pos, e)));

        let mut found = false;
        for (pos, entry) in iterator {
            add_modified(modified, pos as u16);
            entry.enabled = enabled;
            found = true;
        }

        found
    }

    /// Constructs an iterator of enabled source entries from a sources list.
    pub fn entries(&self) -> impl Iterator<Item = &SourceEntry> {
        self.iter()
            .flat_map(|list| -> Box<dyn Iterator<Item = &SourceEntry>> {
                match &list.entries {
                    SourceListType::SourceLine(lines) => Box::new(lines.0.iter().filter_map(|x| {
                        if let SourceLine::Entry(entry) = x {
                            Some(entry)
                        } else {
                            None
                        }
                    })),
                    SourceListType::Deb822(e) => Box::new(e.entries.iter()),
                }
            })
    }

    /// A callback-based iterator that tracks which files have been modified.
    pub fn entries_mut<F: FnMut(&mut SourceEntry) -> bool>(&mut self, mut func: F) {
        let &mut Self {
            ref mut files,
            ref mut modified,
        } = self;
        for (pos, list) in files.iter_mut().enumerate() {
            match list.entries {
                SourceListType::SourceLine(ref mut lines) => {
                    for entry in &mut lines.0 {
                        if let SourceLine::Entry(entry) = entry {
                            if func(entry) {
                                add_modified(modified, pos as u16)
                            }
                        }
                    }
                }
                SourceListType::Deb822(ref mut e) => {
                    for entry in &mut e.entries {
                        if func(entry) {
                            add_modified(modified, pos as u16)
                        }
                    }
                }
            }
        }
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
        let &mut Self {
            ref mut modified,
            ref mut files,
        } = self;

        for (id, list) in files.iter_mut().enumerate() {
            if list.path == path {
                match list.contains_entry(&entry.url) {
                    Some(pos) => match list.entries {
                        SourceListType::SourceLine(ref mut lines) => {
                            lines.0[pos] = SourceLine::Entry(entry)
                        }
                        SourceListType::Deb822(ref mut e) => {
                            e.entries[pos] = entry;
                        }
                    },
                    None => match list.entries {
                        SourceListType::SourceLine(ref mut lines) => {
                            lines.0.push(SourceLine::Entry(entry))
                        }
                        SourceListType::Deb822(ref mut e) => {
                            e.entries.push(entry);
                        }
                    },
                }

                add_modified(modified, id as u16);
                return Ok(());
            }
        }

        files.push(SourcesList {
            path: path.to_path_buf(),
            entries: SourceListType::SourceLine(SourceListLineStyle(vec![SourceLine::Entry(
                entry,
            )])),
        });

        Ok(())
    }

    /// Remove the source entry from each file in the sources lists.
    pub fn remove_entry(&mut self, repo: &str) {
        let &mut Self {
            ref mut modified,
            ref mut files,
        } = self;
        for (id, list) in files.iter_mut().enumerate() {
            if let Some(line) = list.contains_entry(repo) {
                match list.entries {
                    SourceListType::SourceLine(ref mut lines) => {
                        lines.0.remove(line);
                    }
                    SourceListType::Deb822(ref mut e) => {
                        e.entries.remove(line);
                    }
                }
                add_modified(modified, id as u16);
            }
        }
    }

    /// Modify all sources with the `from_suite` to point to the `to_suite`.
    ///
    /// Changes are only applied in-memory. Use `SourcesLists::wirte_sync` to write
    /// all changes to the disk.
    pub fn dist_replace(&mut self, from_suite: &str, to_suite: &str) {
        let &mut Self {
            ref mut modified,
            ref mut files,
        } = self;
        for (id, file) in files.iter_mut().enumerate() {
            let mut changed = false;
            match file.entries {
                SourceListType::SourceLine(ref mut lines) => {
                    for line in &mut lines.0 {
                        if let SourceLine::Entry(ref mut entry) = line {
                            if entry.suite.starts_with(from_suite) {
                                entry.suite = entry.suite.replace(from_suite, to_suite);
                                changed = true;
                            }
                        }
                    }
                }
                SourceListType::Deb822(ref mut e) => {
                    for entry in &mut e.entries {
                        if entry.suite.starts_with(from_suite) {
                            entry.suite = entry.suite.replace(from_suite, to_suite);
                            changed = true;
                        }
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
    pub fn dist_upgrade(
        &mut self,
        retain: &HashSet<Box<str>>,
        from_suite: &str,
        to_suite: &str,
    ) -> io::Result<()> {
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
            retain: &HashSet<Box<str>>,
            from_suite: &str,
            to_suite: &str,
        ) -> io::Result<()> {
            for list in sources.iter_mut() {
                let mut current_file = newfile(modified, &list.path)?;

                match list.entries {
                    SourceListType::SourceLine(ref mut lines) => {
                        for line in &mut lines.0 {
                            if let SourceLine::Entry(entry) = line {
                                if !retain.contains(entry.url.as_str())
                                    && entry.url.starts_with("http")
                                    && entry.suite.starts_with(from_suite)
                                {
                                    entry.suite = entry.suite.replace(from_suite, to_suite);
                                }
                            }

                            writeln!(&mut current_file, "{}", line)?
                        }
                    }
                    SourceListType::Deb822(ref mut e) => writeln!(&mut current_file, "{}", e)?,
                }

                current_file.flush()?;
            }

            Ok(())
        }

        let mut modified = Vec::new();
        apply(self, &mut modified, retain, from_suite, to_suite).inspect_err(|_| {
            // TODO: Revert the ipathsn-memory changes that were made when being applied.
            // revert(self, &modified);

            for (original, backup) in self.iter().zip(modified.iter()) {
                if let Err(why) = fs::copy(backup, &original.path) {
                    eprintln!("failed to restore backup of {:?}: {}", backup, why);
                }
            }
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
        let &mut Self {
            ref mut modified,
            ref mut files,
        } = self;
        modified
            .drain(..)
            .try_for_each(|id| files[id as usize].write_sync())
    }
}

fn scan_inner<P: AsRef<Path>>(dir: P) -> Result<SourcesLists, SourcesListError> {
    let paths = sources_list(dir)?;

    SourcesLists::new_from_paths(paths.iter())
}

pub(crate) fn sources_list<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, SourcesListError> {
    let dir = dir.as_ref();
    let mut paths = vec![];
    let default = dir.join("etc/apt/sources.list");

    if default.exists() {
        paths.push(default);
    }

    if dir.join("etc/apt/sources.list.d/").exists() {
        for entry in fs::read_dir(dir.join("etc/apt/sources.list.d/"))? {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .map_or(false, |e| e == "list" || e == "sources")
            {
                paths.push(path);
            }
        }
    }

    Ok(paths)
}

fn add_modified(modified: &mut Vec<u16>, list: u16) {
    if !modified.iter().any(|&v| v == list) {
        modified.push(list);
    }
}
