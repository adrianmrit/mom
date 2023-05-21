#[cfg(test)]
#[path = "mom_files_test.rs"]
mod mom_files_test;

use crate::cli::Version;
use crate::merge_map_values;
use crate::serde_common::CommonFields;
use crate::tasks::Task;
use crate::types::DynErrResult;
use crate::utils::{get_path_relative_to_base, get_task_dependency_graph, to_os_task_name};
use petgraph::algo::toposort;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

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
    fn deserialize_from_path(path: &Path) -> DynErrResult<MomFile> {
        let contents = match fs::read_to_string(path) {
            Ok(file_contents) => file_contents,
            Err(e) => return Err(format!("There was an error reading the file:\n{}", e).into()),
        };
        Ok(serde_yaml::from_str(&contents)?)
    }

    /// Reads the file from the string and constructs a mom file
    #[cfg(test)]
    fn deserialize_from_str(contents: &str) -> DynErrResult<MomFile> {
        Ok(serde_yaml::from_str(contents)?)
    }

    pub(crate) fn from_path(path: PathBuf) -> DynErrResult<MomFile> {
        let mut mom_file = MomFile::deserialize_from_path(path.as_path())?;
        mom_file.filepath = path;
        mom_file.directory = PathBuf::from(mom_file.filepath.parent().unwrap());
        mom_file.setup()?;
        Ok(mom_file)
    }

    #[cfg(test)]
    pub(crate) fn from_str(contents: &str) -> DynErrResult<MomFile> {
        let mut mom_file = MomFile::deserialize_from_str(contents)?;
        mom_file.setup()?;
        Ok(mom_file)
    }

    /// Loads a mom file
    ///
    /// # Arguments
    ///
    /// * path - path of the toml file to load
    pub(crate) fn setup(&mut self) -> DynErrResult<()> {
        self.common.setup(&self.directory)?;

        let mut tasks = self.get_flat_tasks()?;

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
                let base_task = self
                    .tasks
                    .get(&os_task_name)
                    .unwrap_or_else(|| self.tasks.get(base).unwrap());
                task.extend(base_task);
            }

            // Store the dependencies back in the tasks
            task.common.extend = bases;

            // insert modified task back in
            self.tasks.insert(dependency_name, task);
        }
        Ok(())
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
    pub(crate) fn clone_task(&self, task_name: &str) -> Option<Task> {
        self.get_task(task_name).cloned()
    }

    pub(crate) fn get_task(&self, task_name: &str) -> Option<&Task> {
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
    pub(crate) fn clone_public_task(&self, task_name: &str) -> Option<Task> {
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
