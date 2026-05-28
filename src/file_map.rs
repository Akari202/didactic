use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use log::debug;

use crate::error::DidacticError;

#[derive(Debug, Clone, Default)]
pub struct FileMap {
    entries: HashMap<LogicalPath, RealPath>
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RealPath(pub PathBuf);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct LogicalPath(pub PathBuf);

impl RealPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn strip_base(&self, base: &Path) -> Result<Self, std::path::StripPrefixError> {
        self.0
            .strip_prefix(base)
            .map(|p| Self(PathBuf::from("./").join(p)))
    }
}

impl LogicalPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn to_url_string(&self) -> String {
        format!("/{}", self.0.to_string_lossy().replace('\\', "/"))
    }

    pub fn with_extension(&self, extension: &str) -> Self {
        Self(self.0.with_extension(extension))
    }

    pub fn make_parent(&self) -> Result<(), DidacticError> {
        if let Some(parent) = self.parent()
            && !parent.is_dir()
        {
            debug!("Creating parent directory: {}", parent.display());
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn with_output_path(&self, output_path: &RealPath) -> Self {
        Self(output_path.join(self.0.clone()))
    }
}

impl FileMap {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }

    pub fn add_directory(
        &mut self,
        dir: impl Into<RealPath>,
        logical_prefix: Option<&LogicalPath>
    ) -> Result<(), DidacticError> {
        let dir = dir.into();
        self.walk(&dir, &dir, logical_prefix)
    }

    fn walk(
        &mut self,
        current_dir: &RealPath,
        scan_root: &RealPath,
        logical_prefix: Option<&LogicalPath>
    ) -> Result<(), DidacticError> {
        debug!(
            "Walking dir: {:?}, with root {:?} and prefix {:?}",
            current_dir, scan_root, logical_prefix
        );

        fs::read_dir(current_dir)?.try_for_each(|i| {
            let real = RealPath(i?.path());

            let relative = real.strip_prefix(scan_root)?;
            let logical = LogicalPath(match logical_prefix {
                Some(p) => p.join(relative),
                None => relative.to_path_buf()
            });

            if real.is_dir() {
                self.walk(&real, scan_root, logical_prefix)?;
            } else {
                self.entries.insert(logical, real);
            }
            Ok::<(), DidacticError>(())
        })?;

        Ok(())
    }

    pub fn get_real(&self, logical: &LogicalPath) -> Option<&RealPath> {
        self.entries.get(logical)
    }

    pub fn subdirs_at(&self, prefix: &Path) -> Result<HashSet<LogicalPath>, DidacticError> {
        self.entries.keys().try_fold(HashSet::new(), |mut acc, k| {
            if let Ok(rel) = k.strip_prefix(prefix) {
                let mut components = rel.components();
                if let Some(first) = components.next()
                    && components.next().is_some()
                {
                    acc.insert(LogicalPath(prefix.join(first)));
                }
            }
            Ok(acc)
        })
    }

    pub fn contains(&self, logical: &LogicalPath) -> bool {
        self.entries.contains_key(logical)
    }

    pub fn filter_entries<'a, F>(
        &'a self,
        mut predicate: F
    ) -> impl Iterator<Item = (&'a LogicalPath, &'a RealPath)>
    where
        F: FnMut(&LogicalPath, &RealPath) -> bool + 'a
    {
        self.entries
            .iter()
            .filter(move |(logical, real)| predicate(logical, real))
    }
}

impl fmt::Display for FileMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FileMap:")?;
        writeln!(f, "  Mapped Entries ({}):", self.entries.len())?;
        if self.entries.is_empty() {
            writeln!(f, "    [Empty]")?;
        } else {
            let mut sorted_keys: Vec<&LogicalPath> = self.entries.keys().collect();
            sorted_keys.sort();

            for logical in sorted_keys {
                if let Some(real) = self.entries.get(logical) {
                    writeln!(f, "    {} -> {}", logical.display(), real.display())?;
                }
            }
        }
        Ok(())
    }
}

impl Deref for RealPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for LogicalPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<Path> for LogicalPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for RealPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl From<PathBuf> for RealPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<RealPath> for PathBuf {
    fn from(real_path: RealPath) -> Self {
        real_path.0
    }
}

impl From<&Path> for RealPath {
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

impl From<&PathBuf> for RealPath {
    fn from(path: &PathBuf) -> Self {
        Self(path.clone())
    }
}

impl From<&str> for RealPath {
    fn from(path: &str) -> Self {
        Self(PathBuf::from(path))
    }
}

impl From<String> for RealPath {
    fn from(path: String) -> Self {
        Self(PathBuf::from(path))
    }
}
