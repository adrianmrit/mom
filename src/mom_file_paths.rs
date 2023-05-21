#[cfg(test)]
#[path = "mom_file_paths_test.rs"]
mod mom_file_paths_test;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use directories::UserDirs;

/// Mom file names by order of priority. The program should discover mom files
/// by looping on the parent folders and current directory until reaching the root path
/// or a the project config (last one on the list) is found.
const MOM_FILES_PRIO: &[&str] = &[
    "mom.private.yml",
    "mom.private.yaml",
    "mom.yml",
    "mom.yaml",
    "mom.root.yml",
    "mom.root.yaml",
];

/// Global mom file names by order of priority.
const GLOBAL_MOM_FILES_PRIO: &[&str] = &["mom/mom.global.yml", "mom/mom.global.yaml"];

pub(crate) type PathIteratorItem = PathBuf;
pub(crate) type PathIterator = Box<dyn Iterator<Item = PathIteratorItem>>;

/// Iterates over existing mom file paths, in order of priority.
pub(crate) struct MomFilePaths {
    /// Index of value to use from `MOM_FILES_PRIO`
    index: usize,
    /// Whether the iterator finished or not
    ended: bool,
    /// Current directory
    current_dir: PathBuf,
}

impl Iterator for MomFilePaths {
    // Returning &Path would be more optimal but complicates more the code. There is no need
    // to optimize that much since it should not find that many mom files.
    type Item = PathIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }

        while !self.ended {
            // Loops until a project mom file is found or the root path is reached
            let mom_file_name = MOM_FILES_PRIO[self.index];
            let mom_file_path = self.current_dir.join(mom_file_name);

            let mom_file_path = if mom_file_path.is_file() {
                if self.is_root_mom_file(&mom_file_path) {
                    self.ended = true;
                }
                Some(mom_file_path)
            } else {
                None
            };

            self.index = (self.index + 1) % MOM_FILES_PRIO.len();

            // If we checked all the mom files, we need to check in the parent directory
            if self.index == 0 {
                let new_current = self.current_dir.parent();
                match new_current {
                    None => {
                        self.ended = true;
                    }
                    Some(new_current) => {
                        self.current_dir = new_current.to_path_buf();
                    }
                }
            }
            if let Some(mom_file_path) = mom_file_path {
                return Some(mom_file_path);
            }
        }
        None
    }
}

impl MomFilePaths {
    /// Initializes MomFilePaths to start at the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: Path to start searching for mom files.
    ///
    /// returns: MomFilePaths
    pub(crate) fn new<S: AsRef<OsStr> + ?Sized>(path: &S) -> Box<Self> {
        let current = PathBuf::from(path);
        Box::new(MomFilePaths {
            index: 0,
            ended: false,
            current_dir: current,
        })
    }

    fn is_root_mom_file(&self, path: &Path) -> bool {
        path.file_name()
            .map(|s| s.to_string_lossy().starts_with("mom.root."))
            .unwrap_or(false)
    }
}

/// Single mom file path iterator. This iterator will only return the given path
/// if it exists and is a file, otherwise it will return None.

pub(crate) struct SingleMomFilePath {
    path: PathBuf,
    ended: bool,
}

impl SingleMomFilePath {
    /// Initializes SingleMomFilePath to start at the given path.
    /// If the path does not exist or is not a file, the iterator will return None.
    /// # Arguments
    /// * `path`: Path to start searching for mom files.
    /// returns: SingleMomFilePath

    pub(crate) fn new<S: AsRef<OsStr> + ?Sized>(path: &S) -> Box<Self> {
        Box::new(SingleMomFilePath {
            path: PathBuf::from(path),
            ended: false,
        })
    }
}

impl Iterator for SingleMomFilePath {
    type Item = PathIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        self.ended = true;

        if self.path.is_file() {
            Some(self.path.clone())
        } else {
            None
        }
    }
}

/// Iterator that returns the first existing global mom file path.
pub(crate) struct GlobalMomFilePath {
    ended: bool,
}

impl GlobalMomFilePath {
    /// Initializes GlobalMomFilePath.

    pub(crate) fn new() -> Box<Self> {
        Box::new(GlobalMomFilePath { ended: false })
    }
}

impl Iterator for GlobalMomFilePath {
    type Item = PathIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        self.ended = true;
        if let Some(user_dirs) = UserDirs::new() {
            let home_dir = user_dirs.home_dir();
            for &path in GLOBAL_MOM_FILES_PRIO {
                let path = home_dir.join(path);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }
}
