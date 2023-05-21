#[cfg(test)]
#[path = "mom_files_container_test.rs"]
mod mom_files_container_test;

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use indexmap::IndexMap;

use crate::{mom_files::MomFile, types::DynErrResult, utils::get_path_relative_to_base};

pub(crate) type MomFileSharedPtr = Arc<Mutex<MomFile>>;

/// Caches mom files to avoid reading them multiple times.
pub(crate) struct MomFilesContainer {
    /// Cached mom files
    cached: IndexMap<PathBuf, MomFileSharedPtr>,
    loading: HashSet<PathBuf>,
}

impl MomFilesContainer {
    /// Initializes MomFilesContainer.
    pub(crate) fn new() -> Self {
        MomFilesContainer {
            cached: IndexMap::new(),
            loading: HashSet::new(),
        }
    }

    /// Just loads the mom file without extending it.
    pub(crate) fn load_mom_file(&mut self, path: PathBuf) -> DynErrResult<MomFileSharedPtr> {
        if self.loading.contains(&path) {
            return Err(format!(
                "Found a cyclic dependency for mom file: {}",
                &path.display()
            )
            .into());
        }
        if let Some(mom_file) = self.cached.get(&path) {
            return Ok(Arc::clone(mom_file));
        }
        let mom_file = MomFile::from_path(path.clone());
        match mom_file {
            Ok(mom_file) => {
                let arc_mom_file = Arc::new(Mutex::new(mom_file));
                let result = Ok(Arc::clone(&arc_mom_file));
                self.cached.insert(path, arc_mom_file);
                result
            }
            Err(e) => Err(e),
        }
    }

    /// Reads the mom file from the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: Path to read the mom file from
    ///
    /// returns: Result<Arc<Mutex<MomFile>>, Box<dyn Error, Global>>
    pub(crate) fn read_mom_file(&mut self, path: PathBuf) -> DynErrResult<MomFileSharedPtr> {
        let mom_file = self.load_mom_file(path)?;

        let mut mom_file_lock = mom_file.lock().unwrap();
        let mom_file_lock = &mut *mom_file_lock;

        if mom_file_lock.common.extend.is_empty() {
            return Ok(Arc::clone(&mom_file));
        }

        self.loading.insert(mom_file_lock.filepath.clone());

        let bases = std::mem::take(&mut mom_file_lock.common.extend);
        for base in bases.iter() {
            let full_path = get_path_relative_to_base(&mom_file_lock.directory, &base);
            let base_mom_file = self.read_mom_file(full_path)?;
            mom_file_lock.extend(&base_mom_file.lock().unwrap());
        }

        self.loading.remove(&mom_file_lock.filepath);

        Ok(Arc::clone(&mom_file))
    }

    #[cfg(test)] // Used in tests only for now, but still leaving it here just in case
    /// Returns whether the given task exists in the mom files.
    pub(crate) fn has_task<S: AsRef<str>>(&mut self, name: S) -> bool {
        for mom_file in self.cached.values() {
            let mom_file_ptr = mom_file.as_ref();
            let handle = mom_file_ptr.lock().unwrap();
            if handle.has_task(name.as_ref()) {
                return true;
            }
        }
        false
    }
}

impl Default for MomFilesContainer {
    fn default() -> Self {
        Self::new()
    }
}
