use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self};
use std::path::{Path, PathBuf};

use log::debug;

#[derive(Debug)]
pub struct FileMap {
    entries: HashMap<PathBuf, PathBuf>,
    resolver_base: Option<PathBuf>
}

impl FileMap {
    pub fn with_resolver_base(base: impl Into<PathBuf>) -> Self {
        Self {
            entries: HashMap::new(),
            resolver_base: Some(base.into())
        }
    }

    pub fn add_directory(
        &mut self,
        dir: &Path,
        prefix: Option<&Path>
    ) -> Result<(), Box<dyn Error>> {
        // let dir = dir.canonicalize()?;
        // self.walk(&dir, &dir, prefix)
        self.walk(dir, dir, prefix)
    }

    fn walk(
        &mut self,
        dir: &Path,
        base: &Path,
        prefix: Option<&Path>
    ) -> Result<(), Box<dyn Error>> {
        debug!(
            "Walking dir: {:?}, with base {:?} and prefix {:?}",
            dir, base, prefix
        );
        fs::read_dir(dir)?.try_for_each(|i| {
            let entry = i?;
            let real = entry.path();
            let relative = real.strip_prefix(base)?;
            let logical = match prefix {
                Some(prefix) => prefix.join(relative),
                None => relative.to_path_buf()
            };
            if real.is_dir() {
                self.walk(&real, base, Some(&logical))?;
            } else {
                let stored_real = match &self.resolver_base {
                    Some(base) => real
                        .strip_prefix(base)
                        .map(|p| PathBuf::from("./").join(p))
                        .unwrap_or_else(|_| real.clone()),
                    None => real.clone()
                };
                self.entries.insert(logical, stored_real);
            }
            Ok::<(), Box<dyn Error>>(())
        })?;
        Ok(())
    }

    pub fn get_real(&self, logical: &Path) -> Option<&PathBuf> {
        self.entries.get(logical)
    }

    pub fn subdirs_at(&self, prefix: &Path) -> HashSet<PathBuf> {
        self.entries
            .keys()
            .filter_map(move |k| {
                k.strip_prefix(prefix).ok().and_then(|rel| {
                    let mut components = rel.components();
                    let first = components.next()?;
                    if components.next().is_some() {
                        Some(prefix.join(first))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    pub fn contains(&self, logical: &Path) -> bool {
        self.entries.contains_key(logical)
    }

    pub fn typ_files_at(&self, prefix: &Path) -> impl Iterator<Item = &PathBuf> {
        self.entries.keys().filter(move |k| {
            k.extension().and_then(|s| s.to_str()) == Some("typ")
                && k.strip_prefix(prefix)
                    .map(|r| r.components().count() == 1)
                    .unwrap_or(false)
        })
    }
}
