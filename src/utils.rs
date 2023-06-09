#[cfg(test)]
#[path = "utils_test.rs"]
mod utils_test;

use crate::tasks::Task;
use crate::types::DynErrResult;
use dotenv_parser::parse_dotenv;
use lazy_static::lazy_static;
use petgraph::graphmap::DiGraphMap;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::env::current_dir;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// To uniquely identify the temporary folder. Constant so that the scripts are cached.
pub(crate) const TMP_FOLDER_NAMESPACE: &str = "adrianmrit.mom";

#[cfg(test)]
lazy_static! {
    static ref HOME_DIR: String = {
        let tmp_dir = assert_fs::TempDir::new().unwrap();
        tmp_dir.path().to_string_lossy().to_string()
    };
}

#[cfg(not(test))]
lazy_static! {
    static ref HOME_DIR: String = match directories::UserDirs::new() {
        Some(user_dirs) => user_dirs.home_dir().to_string_lossy().to_string(),
        None => "".to_string(),
    };
}

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

    for (task_name, task) in tasks {
        // The dependency graph must contain all nodes, even if they do not have any relations.
        // So that we can use the graph to traverse the tasks in the correct order.
        graph.add_node(task_name);
        for base_name in task.get_dependencies() {
            let os_base_name = to_os_task_name(base_name);
            if let Some((key, _)) = tasks.get_key_value(&os_base_name) {
                graph.add_edge(task_name, key, ());
            } else if let Some((key, _)) = tasks.get_key_value(base_name) {
                graph.add_edge(task_name, key, ());
            } else {
                return Err(format!(
                    "Task {} cannot inherit from non-existing task {}.",
                    task_name, base_name
                )
                .into());
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
    let path = path.strip_prefix("./").unwrap_or(path);
    if path == Path::new(".") {
        return PathBuf::from(base);
    }
    if !path.is_absolute() {
        let base = Path::new(base);
        return base.join(path);
    }
    path.to_path_buf()
}

pub(crate) fn get_working_directory<B: AsRef<OsStr> + ?Sized, P: AsRef<OsStr> + ?Sized>(
    base: &B,
    path: &P,
) -> PathBuf {
    if path.as_ref().to_string_lossy().is_empty() {
        return current_dir().unwrap();
    }
    get_path_relative_to_base(base, path)
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
pub(crate) fn join_commands(commands: &[impl AsRef<str>]) -> String {
    let mut result = String::new();
    for (i, command) in commands.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        if command.as_ref().contains(' ') {
            result.push('"');
            for c in command.as_ref().chars() {
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
            result.push_str(command.as_ref());
        }
    }
    result
}

/// Expands the given string using the given environment variables, falling back to the system
/// environment variables.
/// It also expands the home directory.
/// If the variable or home dir is not found, it will be replaced by an empty string.
///
/// # Arguments
/// * `val`: String to expand
/// * `env`: Environment variables set in the config file
/// returns: Cow<'a, str>
pub(crate) fn expand_arg<'a, S: AsRef<str> + ?Sized>(
    // Accept &str and String
    arg: &'a S,
    env: &HashMap<String, String>,
) -> Cow<'a, str> {
    let get_env = |name: &str| match env.get(name) {
        Some(val) => Some(Cow::Borrowed(val.as_str())),
        None => match env::var(name) {
            Ok(val) => Some(Cow::Owned(val)),
            Err(_) => Some(Cow::Borrowed("")),
        },
    };
    let home_dir = || Some(HOME_DIR.as_str());
    shellexpand::full_with_context_no_errors(arg, home_dir, get_env)
}

/// Expands the given arguments using the given environment variables, falling back to the system
/// environment variables.
/// It also expands the home directory.
/// If the variable or home dir is not found, it will be replaced by an empty string.
///
/// # Arguments
/// * `args`: Arguments to expand
/// * `env`: Environment variables set in the config file
/// returns: Vec<Cow<'a, str>>
pub(crate) fn expand_args<'a>(
    // Accept [&str] and [String]
    args: &'a [impl AsRef<str>],
    env: &HashMap<String, String>,
) -> Vec<Cow<'a, str>> {
    args.iter().map(|arg| expand_arg(arg, env)).collect()
}
