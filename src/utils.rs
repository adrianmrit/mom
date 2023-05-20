#[cfg(test)]
#[path = "utils_test.rs"]
mod utils_test;

use crate::tasks::Task;
use crate::types::DynErrResult;
use dotenv_parser::parse_dotenv;
use petgraph::graphmap::DiGraphMap;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// To uniquely identify the temporary folder. Constant so that the scripts are cached.
pub(crate) const TMP_FOLDER_NAMESPACE: &str = "adrianmrit.mom";

/// Shortcut to inherit values from the task
#[macro_export]
macro_rules! inherit_option_value {
    ($into:expr, $from:expr) => {
        if $into.is_none() && $from.is_some() {
            $into = $from.clone();
        }
    };
}

#[macro_export]
macro_rules! merge_map_values {
    ($into:expr, $from:expr) => {
        for (key, value) in $from {
            if !$into.contains_key(key) {
                $into.insert(key.clone(), value.clone());
            }
        }
    };
}

#[macro_export]
macro_rules! merge_option_map_values {
    ($into:expr, $from:expr) => {
        if $into.is_none() && $from.is_some() {
            $into = $from.clone();
        } else if $into.is_some() && $from.is_some() {
            let mut $into = $into.unwrap();
            let $from = $from.unwrap();
            merge_map_values!($into, $from);
        }
    };
}

/// Returns the task name as per the current OS.
///
/// # Arguments
///
/// * `task_name`: Plain name of the task
///
/// returns: ()
///
/// # Examples
///
/// ```ignore
/// // Assuming it is a linux system
/// assert_eq!(to_os_task_name("sample"), "sample.linux");
/// ```
pub(crate) fn to_os_task_name(task_name: &str) -> String {
    format!("{}.{}", task_name, env::consts::OS)
}

/// Returns a directed graph containing dependency relations dependency for the given tasks, where
/// the nodes are the names of the tasks. The graph does not include tasks that do not depend, or
/// are not dependencies of other tasks. It is also possible that the graph contains multiple
/// connected components, that is, subgraphs that are not part of larger connected subgraphs.
///
/// # Arguments
///
/// * `tasks`: Hashmap of name to task
///
/// returns: Result<GraphMap<&str, (), Directed>, Box<dyn Error, Global>>
pub(crate) fn get_task_dependency_graph<'a>(
    tasks: &'a HashMap<String, Task>,
) -> DynErrResult<DiGraphMap<&'a str, ()>> {
    let mut graph: DiGraphMap<&'a str, ()> = DiGraphMap::new();

    let mut bases_stack: Vec<&str> = vec![];
    for (task_name, task) in tasks {
        let mut current_task = task;
        let mut current_task_name: &str = task_name;

        if current_task.common.extend.is_empty() {
            continue;
        }

        loop {
            for base_name in current_task.common.extend.iter() {
                let os_base_name = to_os_task_name(base_name);
                let base_name = if tasks.contains_key(&os_base_name) {
                    // os_base_name needs to be a reference to the string in the HashMap
                    let (os_base_name, _) = tasks.get_key_value(&os_base_name).unwrap();
                    os_base_name
                } else {
                    base_name
                };
                if !graph.contains_node(base_name) {
                    bases_stack.push(base_name);
                }
                graph.add_edge(current_task_name, base_name, ());
            }
            while let Some(base) = bases_stack.pop() {
                match tasks.get(base) {
                    None => {
                        return Err(format!(
                            "Task {} cannot inherit from non-existing task {}.",
                            current_task_name, base
                        )
                        .into())
                    }
                    Some(new_current_task) => {
                        current_task = new_current_task;
                        current_task_name = base;
                    }
                }
            }
            if bases_stack.is_empty() {
                break;
            }
        }
    }

    Ok(graph)
}

/// Returns the path relative to the base. If path is already absolute, it will be returned instead.
///
/// # Arguments
///
/// * `base`: Base path
/// * `path`: Path to make relative to the base
///
/// returns: PathBuf
pub(crate) fn get_path_relative_to_base<B: AsRef<OsStr> + ?Sized, P: AsRef<OsStr> + ?Sized>(
    base: &B,
    path: &P,
) -> PathBuf {
    let path = Path::new(path);
    if !path.is_absolute() {
        let base = Path::new(base);
        return base.join(path);
    }
    path.to_path_buf()
}

/// Reads the content of an environment file from the given path and returns a BTreeMap.
///
/// # Arguments
/// * `path`: Path of the environment file
///
/// returns: DynErrResult<BTreeMap<String, String>>
pub(crate) fn read_env_file<S: AsRef<OsStr> + ?Sized>(
    path: &S,
) -> DynErrResult<BTreeMap<String, String>> {
    let path = Path::new(path);
    let result = match fs::read_to_string(path) {
        Ok(content) => parse_dotenv(&content),
        Err(err) => {
            return Err(format!("Failed to read env file at {}: {}", path.display(), err).into())
        }
    };

    match result {
        Ok(envs) => Ok(envs),
        Err(err) => Err(format!("Failed to parse env file at {}: {}", path.display(), err).into()),
    }
}

/// Split a command into its arguments. This is a very simple implementation
/// but it should be enough for most cases.
pub(crate) fn split_command(val: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    for c in val.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        let is_space_like = c == ' ' || c == '\t' || c == '\n' || c == '\r';
        match c {
            '\\' => escaped = true,
            '"' => in_quotes = !in_quotes,
            _ if !in_quotes && is_space_like => {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }

    // TODO: Return error if in_quotes is true

    if !current.is_empty() {
        result.push(current);
    }
    result
}

// Joins the commands, quoting those with spaces and escaping quotes and backslashes.
pub(crate) fn join_commands(commands: &[String]) -> String {
    let mut result = String::new();
    for (i, command) in commands.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        if command.contains(' ') {
            result.push('"');
            for c in command.chars() {
                match c {
                    '"' | '\\' => {
                        result.push('\\');
                        result.push(c);
                    }
                    _ => result.push(c),
                }
            }
            result.push('"');
        } else {
            result.push_str(command);
        }
    }
    result
}
