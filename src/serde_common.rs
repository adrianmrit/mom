use std::{
    collections::{BTreeMap, HashMap},
    mem,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    defaults::{default_false, default_true},
    inherit_option_value, merge_map_values,
    types::DynErrResult,
    utils::{get_path_relative_to_base, read_env_file, read_vars_file},
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

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub(crate) struct VarsFileSpec {
    /// Path to the file
    path: String,
    /// Whether the file is optional or not
    #[serde(default = "default_true")]
    required: bool,
    /// Whether the variables from the file should overwrite the existing ones or not
    #[serde(default = "default_false")]
    overwrite: bool,
}

impl From<String> for VarsFileSpec {
    fn from(path: String) -> Self {
        VarsFileSpec {
            path,
            required: false,
            overwrite: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub(crate) enum VarsFileOrString {
    String(String),
    VarsFile(VarsFileSpec),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub(crate) enum VarsFileOption {
    Single(String),
    Multiple(Vec<VarsFileOrString>),
}

impl Default for VarsFileOption {
    fn default() -> Self {
        VarsFileOption::Multiple(Vec::new())
    }
}

impl IntoIterator for VarsFileOption {
    type Item = VarsFileSpec;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            VarsFileOption::Single(s) => vec![s.into()].into_iter(),
            VarsFileOption::Multiple(v) => v
                .into_iter()
                .map(|item| match item {
                    VarsFileOrString::String(s) => s.into(),
                    VarsFileOrString::VarsFile(v) => v,
                })
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}

/// Common fields for tasks and files
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommonFields {
    /// Working directory. Defaults to the folder where the script runs.
    #[serde(default)]
    pub(crate) wd: Option<String>,

    /// Env variables for all the tasks.
    #[serde(default)]
    pub(crate) env: HashMap<String, String>,

    /// Env files to read environment variables from
    #[serde(default)]
    pub(crate) dotenv: VarsFileOption,

    /// Variables to be used around in the mom file
    #[serde(default)]
    pub(crate) vars: HashMap<String, serde_json::Value>,

    /// Variables file to read variables from
    #[serde(default)]
    pub(crate) vars_file: VarsFileOption,

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

        for env_file in envfiles.into_iter() {
            let overwrite = env_file.overwrite;
            let required = env_file.required;
            let env_file = get_path_relative_to_base(base_path, &env_file.path);
            if !required && !env_file.exists() {
                continue;
            }
            let env_variables = read_env_file(env_file.as_path())?;
            if overwrite {
                for (key, val) in env_variables {
                    self.env.insert(key, val);
                }
            } else {
                for (key, val) in env_variables {
                    self.env.entry(key).or_insert(val);
                }
            }
        }

        let varsfiles = mem::take(&mut self.vars_file);

        for vars_file in varsfiles.into_iter() {
            let overwrite = vars_file.overwrite;
            let required = vars_file.required;
            let vars_file = get_path_relative_to_base(base_path, &vars_file.path);
            if !required && !vars_file.exists() {
                continue;
            }
            let vars = read_vars_file(vars_file.as_path())?;
            if overwrite {
                for (key, val) in vars {
                    self.vars.insert(key, val);
                }
            } else {
                for (key, val) in vars {
                    self.vars.entry(key).or_insert_with(|| val);
                }
            }
        }

        Ok(())
    }
}
