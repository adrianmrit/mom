use std::{
    collections::{BTreeMap, HashMap},
    mem,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{
    inherit_option_value, merge_map_values,
    types::DynErrResult,
    utils::{get_path_relative_to_base, read_env_file},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum StringOrVecString {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for StringOrVecString {
    fn default() -> Self {
        StringOrVecString::Multiple(Vec::new())
    }
}

pub(crate) struct StringOrVecStringIter<'a> {
    task_extend: &'a StringOrVecString,
    index: usize,
}

impl StringOrVecString {
    pub(crate) fn iter(&self) -> StringOrVecStringIter {
        StringOrVecStringIter {
            task_extend: self,
            index: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            StringOrVecString::Single(_) => false,
            StringOrVecString::Multiple(v) => v.is_empty(),
        }
    }
}

impl<'a> Iterator for StringOrVecStringIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self.task_extend {
            StringOrVecString::Single(s) => {
                if self.index == 0 {
                    self.index += 1;
                    Some(s)
                } else {
                    None
                }
            }
            StringOrVecString::Multiple(v) => {
                if self.index < v.len() {
                    let item = &v[self.index];
                    self.index += 1;
                    Some(item)
                } else {
                    None
                }
            }
        }
    }
}

/// Common fields for tasks and files
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommonFields {
    /// Working directory. Defaults to the folder where the script runs.
    #[serde(default)]
    pub(crate) wd: Option<PathBuf>,

    /// Env variables for all the tasks.
    #[serde(default)]
    pub(crate) env: HashMap<String, String>,

    /// Env files to read environment variables from
    #[serde(default)]
    pub(crate) dotenv: StringOrVecString,

    /// Variables to be used around in the mom file
    #[serde(default)]
    pub(crate) vars: HashMap<String, Value>,

    /// Adds the given text to Tera, so that they can be included in templates
    #[serde(default)]
    pub(crate) incl: BTreeMap<String, String>, // Order matters, so we use a BTreeMap

    /// Files to extend from
    #[serde(default)]
    pub(crate) extend: StringOrVecString,
}

impl CommonFields {
    pub(crate) fn extend(&mut self, other: &CommonFields) {
        inherit_option_value!(self.wd, other.wd);
        // env_file should have been loaded into env
        // inherit_option_value!(self.env_file, other.env_file);
        merge_map_values!(self.env, &other.env);
        merge_map_values!(self.vars, &other.vars);
        merge_map_values!(self.incl, &other.incl);
    }

    /// Loads the environment file into the environment variables
    ///
    /// # Arguments
    ///
    /// * `base_path`: path to use as a reference to resolve relative paths
    ///
    /// returns: Result<(), Box<dyn Error, Global>>
    pub(crate) fn setup(&mut self, base_path: &Path) -> DynErrResult<()> {
        // removes the env_file as we won't need it again
        let envfiles = mem::take(&mut self.dotenv);
        for env_file in envfiles.iter() {
            let env_file = get_path_relative_to_base(base_path, &env_file);
            let env_variables = read_env_file(env_file.as_path())?;
            for (key, val) in env_variables {
                self.env.entry(key).or_insert(val);
            }
        }

        self.wd = match &self.wd {
            None => None,
            Some(wd) => {
                if wd == &PathBuf::from(".") {
                    Some(base_path.to_path_buf())
                } else {
                    Some(get_path_relative_to_base(base_path, wd))
                }
            }
        };

        Ok(())
    }
}
