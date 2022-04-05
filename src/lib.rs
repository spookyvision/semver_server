use std::{
    collections::HashMap,
    convert::{Infallible, TryFrom},
    fmt::Display,
    fs::File,
    hash::Hash,
    num::ParseIntError,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
pub mod api;

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct SemVer {
    major: u16,
    minor: u16,
    patch: u16,
}

impl SemVer {
    pub fn new(major: u16, minor: u16, patch: u16) -> SemVer {
        SemVer {
            major,
            minor,
            patch,
        }
    }

    fn new_short(major: u16) -> SemVer {
        Self::new(major, 0, 0)
    }
}

impl Default for SemVer {
    fn default() -> Self {
        Self::new_short(1)
    }
}

impl Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// auto-implements Into<SemVer> for &str
impl From<&str> for SemVer {
    fn from(s: &str) -> Self {
        let vs: Vec<u16> = s.split(".").filter_map(|item| item.parse().ok()).collect();
        assert!(vs.len() == 3);
        SemVer {
            major: vs[0],
            minor: vs[1],
            patch: vs[2],
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("wrong number of parts, {0} (expected: 3)")]
    WrongNumberOfParts(usize),
    #[error("could not parse integer")]
    ParseInt(#[from] ParseIntError),
}

// impl std::fmt::Display for ParseError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str("parse error")
//     }
// }

// impl std::error::Error for ParseError {}

// impl From<ParseIntError> for ParseError {
//     fn from(e: ParseIntError) -> Self {
//         ParseError::ParseInt(e)
//     }
// }

impl FromStr for SemVer {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(".").collect();
        let num_parts = parts.len();
        if num_parts != 3 {
            return Err(ParseError::WrongNumberOfParts(num_parts));
        }
        let res = SemVer {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
        };

        Ok(res)
    }
}

impl TryFrom<String> for SemVer {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<[u16; 3]> for SemVer {
    fn from(value: [u16; 3]) -> Self {
        SemVer {
            major: value[0],
            minor: value[1],
            patch: value[2],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FileURL(String);

#[derive(Debug)]
enum FileURLError {
    InvalidScheme,
    EmptyPath,
}

impl TryFrom<String> for FileURL {
    type Error = FileURLError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !value.starts_with("file://") {
            return Err(FileURLError::InvalidScheme);
        }
        if value.len() < 8 {
            return Err(FileURLError::EmptyPath);
        }
        Ok(Self(value))
    }
}

impl AsRef<Path> for FileURL {
    fn as_ref(&self) -> &Path {
        let inner_string = &self.0;
        let path_part = &inner_string[7..];
        Path::new(path_part)
    }
}

#[derive(Debug, Hash, Clone, Serialize, Deserialize)]
pub struct Metadata {
    name: String,
    author: String,
    kind: CrateKind,
    // repo: FileURL,
}

impl Metadata {
    pub fn new(name: impl AsRef<str>, author: impl AsRef<str>, kind: CrateKind) -> Self {
        Self {
            name: name.as_ref().to_string(),
            author: author.as_ref().to_string(),
            kind,
        }
    }

    /// Get a reference to the metadata's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get a reference to the metadata's author.
    #[must_use]
    pub fn author(&self) -> &str {
        self.author.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crate {
    metadata: Metadata,
    release_history: Vec<SemVer>,
}

impl Crate {
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            release_history: vec![],
        }
    }

    pub fn add_release(&mut self, release: SemVer) -> Result<(), RepoError> {
        let is_newer_hence_valid = self
            .release_history
            .last()
            .map(|v| &release > v)
            .unwrap_or(true);

        if is_newer_hence_valid {
            self.release_history.push(release);
            Ok(())
        } else {
            Err(RepoError::InvalidVersion)
        }
    }
}

impl Hash for Crate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.metadata.name.hash(state);
    }
}
impl PartialEq for Crate {
    fn eq(&self, other: &Self) -> bool {
        self.metadata.name == other.metadata.name
    }
}

impl Eq for Crate {}

#[derive(Debug, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum CrateKind {
    Binary,
    Library,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repository {
    crates: HashMap<String, Crate>,
    store: PathBuf,
}

#[derive(thiserror::Error, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("invalid version")]
    InvalidVersion,
    #[error("already exists")]
    AlreadyExists,
}

impl Repository {
    pub fn new(store: impl AsRef<Path>) -> Self {
        let maybe_contents: Option<Self> = match File::open(&store) {
            Ok(f) => serde_json::from_reader(f).ok(),
            Err(_) => None,
        };

        maybe_contents.unwrap_or(Self {
            crates: HashMap::new(),
            store: store.as_ref().into(),
        })
    }

    /// exact search
    pub fn find_exact(&self, name: impl AsRef<str>) -> Option<&Crate> {
        self.crates.get(name.as_ref())
    }

    /// case insensitive substring search, may return multiple results
    pub fn find_containing(&self, name_part: impl AsRef<str>) -> Vec<&Crate> {
        let name_part_lower = name_part.as_ref().to_lowercase();
        let mut res = vec![];

        for (k, v) in self.crates.iter() {
            if k.to_lowercase().contains(&name_part_lower) {
                res.push(v);
            }
        }
        // the same iteration as
        // for kv in self.crates.iter() {
        //     let (k, v) = kv;
        // }
        res
    }

    pub fn add_crate(&mut self, metadata: Metadata, version: SemVer) -> Result<(), RepoError> {
        if self.crates.contains_key(&metadata.name) {
            Err(RepoError::AlreadyExists)
        } else {
            let mut crt = Crate::new(metadata);
            crt.release_history.push(version);
            self.crates.insert(crt.metadata.name.clone(), crt);
            Ok(())
        }
    }

    pub fn add_release(&mut self, name: impl AsRef<str>, version: SemVer) -> Result<(), RepoError> {
        let crt = self
            .crates
            .get_mut(name.as_ref())
            .ok_or(RepoError::NotFound)?;

        crt.add_release(version)
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        // here we make use of the fact serde_json errors can be converted into std::io::Error
        // (I learned this today while chasing down the dyn problem…)
        let res: Result<_, std::io::Error> = File::create(&self.store)
            .and_then(|f| serde_json::to_writer(f, self).map_err(|e| e.into()));

        if let Err(e) = res {
            eprintln!("could not save repository: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tempfile::NamedTempFile;

    use super::*;

    fn create_repo() -> (NamedTempFile, Repository) {
        let store = NamedTempFile::new().unwrap();
        let repo = Repository::new(&store);

        (store, repo)
    }

    fn create_crate() -> Crate {
        Crate::new(Metadata::new(
            "linux.exe".to_string(),
            "Linus Torvalds".to_string(),
            CrateKind::Binary,
        ))
    }

    fn create_shouty_crate() -> Crate {
        Crate::new(Metadata::new(
            "LINUX.EXE!!".to_string(),
            "LINUS TORVALDS!!!!!".to_string(),
            CrateKind::Binary,
        ))
    }

    #[test]
    fn hash_and_eq() {
        let crt = create_crate();
        let mut crt2 = crt.clone();
        crt2.add_release(SemVer::new(1, 2, 3)).unwrap();

        let mut s1 = HashSet::new();
        let mut s2 = HashSet::new();
        s1.insert(crt);
        s2.insert(crt2);
        assert_eq!(s1, s2);
    }

    #[test]
    fn add_crate() -> Result<(), RepoError> {
        let (_store, mut repo) = create_repo();
        let crt = create_crate();
        let ver = SemVer::new(1, 0, 0);
        repo.add_crate(crt.metadata.clone(), ver)?;

        assert_eq!(
            Err(RepoError::AlreadyExists),
            repo.add_crate(crt.metadata, ver)
        );
        Ok(())
    }

    #[test]
    fn add_release() -> Result<(), RepoError> {
        let (_store, mut repo) = create_repo();
        let crt = create_crate();
        let ver = SemVer::new(1, 0, 0);
        let metadata = crt.metadata;

        repo.add_crate(metadata.clone(), ver)?;

        repo.add_release(&metadata.name, SemVer::new(1, 0, 1))?;

        assert_eq!(
            Err(RepoError::InvalidVersion),
            repo.add_release(&metadata.name, SemVer::new(1, 0, 1))
        );
        repo.add_release(&metadata.name, SemVer::new(2, 0, 0))?;

        Ok(())
    }

    #[test]
    fn find() -> Result<(), RepoError> {
        let store = NamedTempFile::new().unwrap();
        let mut repo = Repository::new(&store);

        assert_eq!(None, repo.find_exact("linux.exe"));
        repo.add_crate(create_crate().metadata, SemVer::new(1, 2, 3))?;

        assert_eq!(Some(&create_crate()), repo.find_exact("linux.exe"));
        Ok(())
    }

    #[test]
    fn find_all() -> Result<(), RepoError> {
        let (_store, mut repo) = create_repo();

        assert_eq!(None, repo.find_exact("linux.exe"));
        repo.add_crate(create_crate().metadata, SemVer::new(1, 0, 0))?;
        repo.add_crate(create_shouty_crate().metadata, SemVer::new(2, 0, 5))?;

        // HashMap does not guarantee order, hence we're going to compare sets instead
        let mut cmp = HashSet::new();

        let c1 = create_crate();
        let c2 = create_shouty_crate();
        cmp.extend(vec![&c1, &c2].into_iter());
        // the same as:
        //cmp.insert(&c1); cmp.insert(&c2);
        let search_result = repo.find_containing("NuX").into_iter().collect();
        assert_eq!(cmp, search_result);
        Ok(())
    }
}
