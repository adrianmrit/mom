#[cfg(test)]
#[path = "mom_files_test.rs"]
mod mom_files_test;

use crate::cli::Version;
use crate::merge_map_values;
use crate::serde_common::CommonFields;
use crate::tasks::Task;
use crate::types::DynErrResult;
use crate::utils::{get_path_relative_to_base, get_task_dependency_graph, to_os_task_name};
use directories::UserDirs;
use indexmap::IndexMap;
use petgraph::algo::toposort;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs};

pub(crate) type MomFileSharedPtr = Arc<Mutex<MomFile>>;

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

// At the moment we don't really take advantage of this, but might be useful in the future.
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
        let mom_file = MomFile::load(path.clone());
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

/// Represents a mom file.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MomFile {
    /// Version of the mom file.
    pub(crate) version: Version,
    /// Path of the file.
    #[serde(skip_deserializing)]
    pub(crate) filepath: PathBuf,

    #[serde(skip_deserializing)]
    pub(crate) directory: PathBuf,

    #[serde(flatten)]
    pub(crate) common: CommonFields,

    /// Tasks inside the mom file.
    #[serde(default)]
    pub(crate) tasks: HashMap<String, Task>,
}

impl MomFile {
    /// Reads the file from the path and constructs a mom file
    fn extract(path: &Path) -> DynErrResult<MomFile> {
        let contents = match fs::read_to_string(path) {
            Ok(file_contents) => file_contents,
            Err(e) => return Err(format!("There was an error reading the file:\n{}", e).into()),
        };
        Ok(serde_yaml::from_str(&contents)?)
    }

    /// Loads a mom file
    ///
    /// # Arguments
    ///
    /// * path - path of the toml file to load
    pub(crate) fn load(path: PathBuf) -> DynErrResult<MomFile> {
        let mut conf: MomFile = MomFile::extract(path.as_path())?;
        conf.filepath = path;
        conf.directory = PathBuf::from(conf.filepath.parent().unwrap());
        conf.common.setup(&conf.directory)?;

        let mut tasks = conf.get_flat_tasks()?;

        let dep_graph = get_task_dependency_graph(&tasks)?;

        // TODO: Return the cycle. Could use petgraph::visit::DfsPostOrder instead of toposort
        let dependencies = toposort(&dep_graph, None);

        let dependencies = match dependencies {
            Ok(dependencies) => dependencies,
            Err(e) => {
                return Err(format!("Found a cyclic dependency for task: {}", e.node_id()).into());
            }
        };

        let dependencies: Vec<String> = dependencies
            .iter()
            .rev()
            .map(|v| String::from(*v))
            .collect();

        for dependency_name in dependencies {
            // temp remove because of rules of references
            let mut task = tasks.remove(&dependency_name).unwrap();

            // We don't need the bases anymore, but we want to keep them in case the user wants to
            // access them from the context in Tera. However we need to remove temporarily because
            // of the rules of references.
            let bases = std::mem::take(&mut task.common.extend);

            // Extend from the bases. Because of the topological sort, the bases should already be
            // loaded.
            for base in bases.iter() {
                let os_task_name = format!("{}.{}", &base, env::consts::OS);
                // The base task must exist, otherwise it would have failed when creating the dependency graph
                let base_task = conf
                    .tasks
                    .get(&os_task_name)
                    .unwrap_or_else(|| conf.tasks.get(base).unwrap());
                task.extend(base_task);
            }

            // Store the dependencies back in the tasks
            task.common.extend = bases;

            // insert modified task back in
            conf.tasks.insert(dependency_name, task);
        }
        Ok(conf)
    }

    pub(crate) fn extend(&mut self, other: &MomFile) {
        self.common.extend(&other.common);
        merge_map_values!(self.tasks, &other.tasks);
    }

    /// If set in the mom file, returns the working directory as an absolute path.
    pub(crate) fn working_directory(&self) -> Option<PathBuf> {
        // Some sort of cache would make it faster, but keeping it
        // simple until it is really needed
        self.common
            .wd
            .as_ref()
            .map(|wd| get_path_relative_to_base(&self.directory, wd))
    }

    /// Returns plain and OS specific tasks with normalized names. This consumes `self.tasks`
    fn get_flat_tasks(&mut self) -> DynErrResult<HashMap<String, Task>> {
        let mut flat_tasks = HashMap::new();
        let tasks = std::mem::take(&mut self.tasks);

        // macro to avoid repeating code
        macro_rules! insert_os_task {
            ($os_task:expr, $parent_name:expr, $os_name:expr) => {
                let os_task = std::mem::replace(&mut $os_task, None);
                let mut os_task = *os_task.unwrap();
                let os_task_name = format!("{}.{}", $parent_name, $os_name);
                if flat_tasks.contains_key(&os_task_name) {
                    return Err(format!("Duplicate task `{}`", os_task_name).into());
                }
                os_task.setup(&os_task_name, &self.directory)?;
                flat_tasks.insert(os_task_name, os_task);
            };
        }

        for (name, mut task) in tasks {
            if task.linux.is_some() {
                insert_os_task!(task.linux, name, "linux");
            }

            if task.windows.is_some() {
                insert_os_task!(task.windows, name, "windows");
            }

            if task.macos.is_some() {
                insert_os_task!(task.macos, name, "macos");
            }
            task.setup(&name, &self.directory)?;
            flat_tasks.insert(name, task);
        }
        Ok(flat_tasks)
    }

    /// Finds and task by name on this mom file and returns a clone if it exists.
    /// It searches fist for the current OS version of the task, if None is found,
    /// it tries with the plain name.
    ///
    /// # Arguments
    ///
    /// * task_name - Name of the task to search for
    pub(crate) fn get_task(&self, task_name: &str) -> Option<Task> {
        self.get_task_ref(task_name).cloned()
    }

    pub(crate) fn get_task_ref(&self, task_name: &str) -> Option<&Task> {
        let os_task_name = to_os_task_name(task_name);

        if let Some(task) = self.tasks.get(&os_task_name) {
            return Some(task);
        } else if let Some(task) = self.tasks.get(task_name) {
            return Some(task);
        }
        None
    }

    /// Finds an public task by name on this mom file and returns it if it exists.
    /// It searches fist for the current OS version of the task, if None is found,
    /// it tries with the plain name.
    ///
    /// # Arguments
    ///
    /// * task_name - Name of the task to search for
    pub(crate) fn get_public_task(&self, task_name: &str) -> Option<Task> {
        let os_task_name = to_os_task_name(task_name);

        let task = self
            .tasks
            .get(&os_task_name)
            .or_else(|| self.tasks.get(task_name));

        if let Some(task) = task {
            if task.is_private() {
                return None;
            }
            Some(task.clone())
        } else {
            None
        }
    }

    /// Returns whether the mom file has a task with the given name. This also
    /// checks for the OS specific version of the task.
    ///
    /// # Arguments
    ///
    /// * `task_name`: Name of the task to check for
    ///
    /// returns: bool
    #[cfg(test)]
    pub(crate) fn has_task(&self, task_name: &str) -> bool {
        let os_task_name = to_os_task_name(task_name);

        self.tasks.contains_key(&os_task_name) || self.tasks.contains_key(task_name)
    }

    /// Returns the list of names of tasks that are not private in this mom file
    pub(crate) fn get_public_task_names(&self) -> Vec<&str> {
        self.tasks
            .values()
            .filter(|t| !t.is_private())
            .map(|t| t.get_name())
            .collect()
    }
}
