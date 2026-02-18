use std::fmt;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayablePathBuf(pub PathBuf);

impl fmt::Display for DisplayablePathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl From<PathBuf> for DisplayablePathBuf {
    fn from(path: PathBuf) -> Self {
        DisplayablePathBuf(path)
    }
}

impl std::ops::Deref for DisplayablePathBuf {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for DisplayablePathBuf {
    fn from(s: String) -> Self {
        DisplayablePathBuf(PathBuf::from(s))
    }
}

impl From<&str> for DisplayablePathBuf {
    fn from(s: &str) -> Self {
        DisplayablePathBuf(PathBuf::from(s))
    }
}
