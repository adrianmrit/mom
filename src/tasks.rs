#[cfg(test)]
#[path = "tasks_test.rs"]
mod tasks_test;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fmt, fs, mem};

use crate::args::ArgsContext;
use crate::defaults::default_false;
use crate::errors::{AwareTaskError, TaskError};
use crate::inherit_option_value;
use crate::mom_files::MomFile;
use crate::print_utils::{MomOutput, INFO_COLOR};
use crate::serde_common::CommonFields;
use crate::tera::get_tera_instance;
use colored::Colorize;
use serde::{de, Deserialize, Serialize};

use crate::types::DynErrResult;
use crate::utils::{
    expand_arg, expand_args, get_working_directory, join_commands, split_command,
    TMP_FOLDER_NAMESPACE,
};
use md5::{Digest, Md5};

pub const DRY_RUN_MESSAGE: &str = "Dry run mode, nothing executed.";

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        // Will run the actual script in CMD, but we don't need to specify /C option
        const DEFAULT_SCRIPT_RUNNER: &str = "powershell {{ script_path }}";
        const DEFAULT_SCRIPT_EXTENSION: &str = "cmd";
    } else if #[cfg(target_os = "linux")] {
        const DEFAULT_SCRIPT_RUNNER: &str = "bash {{ script_path }}";
        const DEFAULT_SCRIPT_EXTENSION: &str = "sh";
    } else if #[cfg(target_os = "macos")] {
        const DEFAULT_SCRIPT_RUNNER: &str = "bash {{ script_path }}";
        const DEFAULT_SCRIPT_EXTENSION: &str = "sh";
    }else {
        compile_error!("Unsupported platform.");
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        fn create_script_file<P: AsRef<Path>>(path: P) -> DynErrResult<File> {
            Ok(File::create(&path)?)
        }
    } else {
        use std::os::unix::fs::OpenOptionsExt;
        use std::fs::OpenOptions;
        fn create_script_file<P: AsRef<Path>>(path: P) -> DynErrResult<File> {
            Ok(OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o770)  // Create with appropriate permission
            .open(path)?)
        }
    }
}

/// Creates a temporal script returns the path to it.
/// The OS should take care of cleaning the file.
///
/// # Arguments
///
/// * `content` - Content of the script file
fn get_temp_script(
    content: &str,
    extension: &str,
    task_name: &str,
    mom_file_path: &Path,
) -> DynErrResult<PathBuf> {
    let mut path = temp_dir();
    path.push(TMP_FOLDER_NAMESPACE);
    fs::create_dir_all(&path)?;

    let extension = if extension.is_empty() {
        String::new()
    } else if extension.starts_with('.') {
        String::from(extension)
    } else {
        format!(".{}", extension)
    };

    // get md5 hash of the task_name, mom_file_path and content
    let mut hasher = Md5::new();
    hasher.update(task_name.as_bytes());
    hasher.update(mom_file_path.to_str().unwrap().as_bytes());
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    let file_name = format!("{:X}{}", hash, extension);
    path.push(file_name);

    // Uses the temp file as a cache, so it doesn't have to create it every time
    // we run the same script.
    if path.exists() {
        return Ok(path);
    }
    let mut file = create_script_file(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(path)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskNameOption {
    task: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CmdOption {
    #[serde(flatten)]
    command: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum Cmd {
    #[serde(rename = "task_name")]
    TaskName(String),
    #[serde(rename = "task")]
    Task(Box<Task>),
    #[serde(rename = "cmd")]
    Cmd(String),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum StringOrTask {
    String(String),
    Task(Box<Task>),
}

impl<'de> de::Deserialize<'de> for Cmd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct CmdVisitor;

        impl<'de> de::Visitor<'de> for CmdVisitor {
            type Value = Cmd;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("cmd, task name or task")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Cmd::Cmd(value.to_string()))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                match map.next_key::<String>()? {
                    Some(key) => match key.as_str() {
                        "task" => {
                            let string_or_task: StringOrTask = map.next_value()?;
                            match string_or_task {
                                StringOrTask::String(s) => Ok(Cmd::TaskName(s)),
                                StringOrTask::Task(t) => Ok(Cmd::Task(t)),
                            }
                        }
                        "cmd" => {
                            let cmd: String = map.next_value()?;
                            Ok(Cmd::Cmd(cmd))
                        }
                        _ => Err(de::Error::unknown_field(key.as_str(), &["task", "cmd"])),
                    },
                    None => Err(de::Error::missing_field("task_name or task")),
                }

                // Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
            }
        }

        deserializer.deserialize_any(CmdVisitor {})
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TaskCondition(String);

impl TaskCondition {
    pub(crate) fn holds(
        &self,
        task_name: &str,
        tera: &mut tera::Tera,
        context: &tera::Context,
    ) -> Result<bool, AwareTaskError> {
        let template_name = format!("{}.condition", task_name);
        tera.add_raw_template(&template_name, &self.0)
            .map_err(|e| {
                AwareTaskError::new(
                    task_name,
                    TaskError::ConfigError(format!("Invalid condition: {}", e)),
                )
            })?;
        let result = tera.render(&template_name, context).map_err(|e| {
            AwareTaskError::new(
                task_name,
                TaskError::ConfigError(format!("Invalid condition: {}", e)),
            )
        })?;
        let result = result.trim().to_lowercase();
        Ok(result == "true")
    }
}

/// Represents a Task
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Task {
    /// Name of the task
    #[serde(skip_deserializing)]
    pub(crate) name: String,

    #[serde(flatten)]
    pub(crate) common: CommonFields,

    /// Condition to run the task
    condition: Option<TaskCondition>,

    /// Help of the task
    help: Option<String>,

    /// Script to run
    script: Option<String>,

    /// Interpreter program to use
    script_runner: Option<String>,

    /// Script extension
    #[serde(alias = "script_ext")]
    script_extension: Option<String>,

    /// A program to run
    program: Option<String>,

    /// Args to pass to a command
    args: Option<String>,

    /// Run commands
    cmds: Option<Vec<Cmd>>,

    /// Extends args from bases
    #[serde(alias = "args+")]
    args_extend: Option<String>,

    /// Task to run instead if the OS is linux
    pub(crate) linux: Option<Box<Task>>,

    /// Task to run instead if the OS is windows
    pub(crate) windows: Option<Box<Task>>,

    /// Task to run instead if the OS is macos
    pub(crate) macos: Option<Box<Task>>,

    /// If private, it cannot be called
    #[serde(default = "default_false")]
    private: bool,
}

impl Task {
    /// Returns the dependencies of the task.
    pub(crate) fn get_dependencies(&self) -> Vec<&str> {
        let mut dependencies: Vec<&str> = self.common.extend.iter().collect();

        if let Some(cmds) = &self.cmds {
            for cmd in cmds {
                match cmd {
                    Cmd::TaskName(task_name) => {
                        dependencies.push(task_name);
                    }
                    Cmd::Task(task) => {
                        dependencies.append(&mut task.get_dependencies());
                    }
                    Cmd::Cmd(_) => {}
                }
            }
        }

        dependencies
    }

    /// Does extra setup on the task and does some validation.
    ///
    /// # Arguments
    ///
    /// * `name`: name of the task
    /// * `base_path`: path to use as a reference to resolve relative paths
    ///
    /// returns: Result<(), Box<dyn Error, Global>>
    ///
    pub(crate) fn setup(&mut self, name: &str, base_path: &Path) -> Result<(), AwareTaskError> {
        self.name = String::from(name);
        match self.common.setup(base_path) {
            Ok(_) => {}
            Err(e) => {
                return Err(AwareTaskError::new(
                    name,
                    TaskError::ConfigError(format!("{}", e)),
                ))
            }
        }
        match self.validate() {
            Ok(_) => Ok(()),
            Err(e) => Err(AwareTaskError::new(name, e)),
        }
    }

    #[cfg(test)]
    pub(crate) fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// Helper function for running a task. Accepts the environment variables as a HashMap.
    /// So that we can reuse the environment variables for multiple tasks.
    pub(crate) fn run(
        &self,
        args: &ArgsContext,
        mom_file: &MomFile,
        dry_run: bool,
    ) -> Result<(), AwareTaskError> {
        let env = self.get_env(&mom_file.common.env);
        let vars = self.get_vars(&mom_file.common.vars);

        let mut tera_instance = self
            .get_tera_instance(mom_file, env.clone())
            .map_err(|e| AwareTaskError::new(&self.name, e))?;
        let mut tera_context = self.get_tera_context(args, mom_file, &env, &vars);

        if let Some(condition) = &self.condition {
            if !condition.holds(&self.name, &mut tera_instance, &tera_context)? {
                println!("{}", format!("{} skipped", &self.name).mom_info());
                return Ok(());
            }
        }

        let result = if self.script.is_some() {
            self.run_script(
                mom_file,
                &env,
                &mut tera_instance,
                &mut tera_context,
                dry_run,
            )
        } else if self.program.is_some() {
            self.run_program(
                mom_file,
                &env,
                &mut tera_instance,
                &mut tera_context,
                dry_run,
            )
        } else if self.cmds.is_some() {
            self.run_cmds(
                args,
                mom_file,
                &env,
                &mut tera_instance,
                &mut tera_context,
                dry_run,
            )
        } else {
            Err(TaskError::ConfigError(String::from("Nothing to run.")))
        };

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(AwareTaskError::new(&self.name, e)),
        }
    }

    /// Extends from the given task.
    ///
    /// # Arguments
    ///
    /// * `base_task`: task to extend from
    ///
    /// returns: ()
    ///
    pub(crate) fn extend(&mut self, base_task: &Task) {
        inherit_option_value!(self.help, base_task.help);
        inherit_option_value!(self.script, base_task.script);
        inherit_option_value!(self.script_runner, base_task.script_runner);
        inherit_option_value!(self.script_extension, base_task.script_extension);
        inherit_option_value!(self.program, base_task.program);
        inherit_option_value!(self.args, base_task.args);
        inherit_option_value!(self.cmds, base_task.cmds);
        inherit_option_value!(self.condition, base_task.condition);
        self.common.extend(&base_task.common);

        if self.args_extend.is_some() {
            let new_args = mem::take(&mut self.args_extend).unwrap();
            if self.args.is_none() {
                self.args = mem::replace(&mut self.args, Some(String::new()));
            }
            if let Some(args) = &mut self.args {
                args.push(' ');
                args.push_str(&new_args);
            } else {
                self.args = Some(new_args);
            }
        }
    }

    /// Returns the name of the task
    pub(crate) fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns weather the task is private or not
    pub(crate) fn is_private(&self) -> bool {
        self.private
    }

    /// Returns the help for the task
    pub(crate) fn get_help(&self) -> &str {
        match self.help {
            Some(ref help) => help.trim(),
            None => "",
        }
    }

    /// Returns the environment variables by merging the ones from the mom file with
    /// the ones from the task, where the task takes precedence.
    ///
    /// # Arguments
    ///
    /// * `mom_file`: mom file to load extra environment variables from
    ///
    /// returns: HashMap<String, String, RandomState>
    fn get_env(&self, env: &HashMap<String, String>) -> HashMap<String, String> {
        let mut new_env = self.common.env.clone();
        for (key, val) in env {
            new_env.entry(key.clone()).or_insert_with(|| val.clone());
        }
        new_env
    }

    /// Returns the environment variables by merging the ones from the mom file with
    /// the ones from the task, where the task takes precedence.
    ///
    /// # Arguments
    ///
    /// * `mom_file`: mom file to load extra environment variables from
    ///
    /// returns: HashMap<String, String, RandomState>
    fn get_vars(
        &self,
        env: &HashMap<String, serde_yaml::Value>,
    ) -> HashMap<String, serde_yaml::Value> {
        let mut new_vars: HashMap<String, serde_yaml::Value> = self.common.vars.clone();
        for (key, val) in env {
            new_vars.entry(key.clone()).or_insert_with(|| val.clone());
        }
        new_vars
    }

    /// Same as `get_env` but for the tera templates
    fn get_templates(&self, tera_templates: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut new_templates: BTreeMap<String, String> = self.common.incl.clone();
        for (key, val) in tera_templates {
            new_templates
                .entry(key.clone())
                .or_insert_with(|| val.clone());
        }
        new_templates
    }

    /// Validates the task configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the task
    fn validate(&self) -> Result<(), TaskError> {
        if self.script.is_some() && self.program.is_some() {
            return Err(TaskError::ConfigError(String::from(
                "Cannot set both `script` and `program`.",
            )));
        }

        if self.script.is_some() && self.cmds.is_some() {
            return Err(TaskError::ConfigError(String::from(
                "Cannot set both `cmds` and `script`.",
            )));
        }

        if self.program.is_some() && self.cmds.is_some() {
            return Err(TaskError::ConfigError(String::from(
                "Cannot set both `cmds` and `program`.",
            )));
        }

        Ok(())
    }

    // Returns the Tera instance for the Tera template engine.
    fn get_tera_instance(
        &self,
        mom_file: &MomFile,
        env: HashMap<String, String>,
    ) -> Result<tera::Tera, TaskError> {
        let mut tera = get_tera_instance(env);
        for (name, template) in mom_file.common.incl.iter() {
            tera.add_raw_template(&format!("incl.{name}"), template)?;
        }
        for (name, template) in self.common.incl.iter() {
            tera.add_raw_template(&format!("incl.{name}"), template)?;
        }
        Ok(tera)
    }

    /// Returns the context for the Tera template engine.
    fn get_tera_context(
        &self,
        args: &ArgsContext,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
        vars: &HashMap<String, serde_yaml::Value>,
    ) -> tera::Context {
        let mut context = tera::Context::new();

        context.insert("args", &args.args);
        context.insert("kwargs", &args.kwargs);
        context.insert("pkwargs", &args.pkwargs);
        context.insert("vars", &vars);
        context.insert("env", &env);
        context.insert("TASK", self);
        context.insert("FILE", mom_file);

        context
    }

    /// Sets common parameters for commands, like stdout, stderr, stdin, working directory and
    /// environment variables.
    ///
    /// # Arguments
    ///
    /// * `command` - Command to set the parameters for
    /// * `mom_file` - Configuration file
    fn set_command_basics(
        &self,
        command: &mut Command,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
    ) -> Result<(), TaskError> {
        command.envs(env);
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
        command.stdin(Stdio::inherit());

        let wd = match &self.common.wd {
            None => mom_file.common.wd.as_ref(),
            Some(wd) => Some(wd),
        };

        if let Some(wd) = wd {
            let wd = expand_arg(wd, env);
            let wd = Path::new(wd.as_ref());
            let mom_file_folder = &mom_file.directory;
            // wd may be absolute or relative to the mom file folder
            let wd = get_working_directory(mom_file_folder, wd);
            command.current_dir(wd);
        }

        Ok(())
    }

    /// Spawns a command and waits for its execution.
    ///
    /// # Arguments
    ///
    /// * `command` - Command to spawn
    fn spawn_command(&self, command: &mut Command, dry_run: bool) -> Result<(), TaskError> {
        if dry_run {
            println!("{}", DRY_RUN_MESSAGE.mom_info());
            return Ok(());
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                return Err(TaskError::RuntimeError(format!("{}", e)));
            }
        };

        // let child handle ctrl-c to prevent dropping the parent and leaving the child running
        ctrlc::set_handler(move || {}).unwrap_or(());

        let result = child.wait()?;
        match result.success() {
            true => Ok(()),
            false => match result.code() {
                None => Err(TaskError::RuntimeError(String::from(
                    "Process did not terminate correctly",
                ))),
                Some(code) => Err(TaskError::RuntimeError(format!(
                    "Process terminated with exit code {}",
                    code
                ))),
            },
        }
    }

    /// Runs a program
    fn run_program(
        &self,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
        tera_instance: &mut tera::Tera,
        tera_context: &mut tera::Context,
        dry_mode: bool,
    ) -> Result<(), TaskError> {
        let program = self.program.as_ref().unwrap();

        // In case the program is specified with ~ or $HOME, or something like that
        let program = expand_arg(program, env);

        let mut command = Command::new(program.as_ref());
        self.set_command_basics(&mut command, mom_file, env)?;

        let args_list = match &self.args {
            None => vec![],
            Some(args) => {
                let task_name = &self.name;
                let template_name = format!("tasks.{task_name}.args");
                tera_instance.add_raw_template(&template_name, args)?;
                let rendered_args = tera_instance.render(&template_name, tera_context)?;
                split_command(&rendered_args)
            }
        };
        if args_list.is_empty() {
            println!("{}", format!("{}: {}", self.name, program).mom_info());
        } else {
            let display_args = join_commands(&args_list);
            let args = expand_args(&args_list, env);
            let args = args.iter().map(|s| s.as_ref());
            command.args(args);

            println!(
                "{}",
                format!("{}: {} {}", self.name, program, display_args).mom_info()
            );
        }

        self.spawn_command(&mut command, dry_mode)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_cmds_cmd(
        &self,
        cmd: &str,
        cmd_index: usize,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
        tera_instance: &mut tera::Tera,
        tera_context: &mut tera::Context,
        dry_run: bool,
    ) -> Result<(), TaskError> {
        let task_name = &self.name;
        let task_name = &format!("{task_name}.cmds.{cmd_index}");
        let template_name = &format!("tasks.{task_name}");
        tera_instance.add_raw_template(template_name, cmd)?;

        let cmd = tera_instance.render(template_name, tera_context);
        let cmd = cmd?;
        let cmd_args = split_command(&cmd);
        let cmd_args: Vec<Cow<str>> = expand_args(&cmd_args, env);
        let cmd_args: Vec<&str> = cmd_args.iter().map(|s| s.as_ref()).collect();
        let program = match cmd_args.first() {
            Some(program) => program,
            None => {
                return Err(TaskError::RuntimeError(format!(
                    "Error running task: {}",
                    "No program specified"
                )))
            }
        };
        let program_args = &cmd_args[1..];
        let mut command: Command = Command::new(program);
        self.set_command_basics(&mut command, mom_file, env)?;
        command.args(program_args);

        println!(
            "{}",
            // We print the clean commands, not the rendered ones. For a nicer output.
            format!("{task_name}: {}", join_commands(&cmd_args)).mom_info()
        );
        self.spawn_command(&mut command, dry_run)
    }

    fn run_cmds_task_name(
        &self,
        task_name: &str,
        cmd_index: usize,
        args: &ArgsContext,
        mom_file: &MomFile,
        dry_run: bool,
    ) -> Result<(), TaskError> {
        let display_task_name = format!("{}.cmds.{}.{}", self.name, cmd_index, task_name);
        if let Some(mut task) = mom_file.clone_task(task_name) {
            // The env and vars of the parent take precedence in this case.
            task.common.env = self.get_env(&task.common.env);
            task.common.vars = self.get_vars(&task.common.vars);
            task.common.incl = self.get_templates(&task.common.incl);

            // Should setup first, to load the env_file.
            task.setup(&display_task_name, &mom_file.directory)?;

            if let Err(e) = task.run(args, mom_file, dry_run) {
                Err(TaskError::RuntimeError(format!(
                    "Error running task: {}",
                    e
                )))
            } else {
                Ok(())
            }
        } else {
            Err(TaskError::NotFound(task_name.to_string()))
        }
    }

    fn run_cmds_task(
        &self,
        task: &Task,
        cmd_index: usize,
        args: &ArgsContext,
        mom_file: &MomFile,
        dry_run: bool,
    ) -> Result<(), TaskError> {
        let mut task = task.clone();
        let task_name = format!("{}.cmds.{}", self.name, cmd_index);

        task.setup(&task_name, &mom_file.directory)?;

        // Should setup first, to load the env_file. This way the child task inherits from the parent,
        // but can override specific values

        let extend = &task.common.extend.clone();

        for base_name in extend.iter() {
            // Because the bases have been loaded already, there cannot be any circular dependencies
            // Todo, get reference to base task instead of cloning it
            let base_task = mom_file.clone_task(base_name);
            match base_task {
                Some(base_task) => task.extend(&base_task),
                None => {
                    return Err(TaskError::NotFound(base_name.to_string()));
                }
            }
        }

        // Done after setup and bases, so that the env and vars specified directly in the child take precedence
        task.common.env = task.get_env(&self.common.env);
        task.common.vars = task.get_vars(&self.common.vars);
        task.common.incl = task.get_templates(&self.common.incl);

        // This should load the mom file env and vars
        task.run(args, mom_file, dry_run).map_err(|e| e.into())
    }

    /// Runs the commands specified with the cmds option.
    fn run_cmds(
        &self,
        args: &ArgsContext,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
        tera_instance: &mut tera::Tera,
        tera_context: &mut tera::Context,
        dry_run: bool,
    ) -> Result<(), TaskError> {
        for (i, cmd) in self.cmds.as_ref().unwrap().iter().enumerate() {
            match cmd {
                Cmd::Cmd(cmd) => {
                    self.run_cmds_cmd(cmd, i, mom_file, env, tera_instance, tera_context, dry_run)?;
                }
                Cmd::TaskName(task_name) => {
                    self.run_cmds_task_name(task_name, i, args, mom_file, dry_run)?;
                }
                Cmd::Task(task) => {
                    self.run_cmds_task(task, i, args, mom_file, dry_run)?;
                }
            }
        }
        Ok(())
    }

    /// Runs a script
    fn run_script(
        &self,
        mom_file: &MomFile,
        env: &HashMap<String, String>,
        tera_instance: &mut tera::Tera,
        tera_context: &mut tera::Context,
        dry_run: bool,
    ) -> Result<(), TaskError> {
        let script = self.script.as_ref().unwrap();

        let task_name = &self.name;
        let template_name = format!("tasks.{task_name}.script");
        tera_instance.add_raw_template(&template_name, script)?;
        let script = tera_instance.render(&template_name, tera_context)?;
        let default_script_extension = String::from(DEFAULT_SCRIPT_EXTENSION);
        let script_extension = self
            .script_extension
            .as_ref()
            .unwrap_or(&default_script_extension);

        let script_path = get_temp_script(
            &script,
            script_extension,
            &self.name,
            mom_file.filepath.as_path(),
        );

        let script_path = match script_path {
            Ok(path) => path,
            Err(e) => {
                return Err(TaskError::RuntimeError(format!(
                    "Error creating script file: {}",
                    e
                )))
            }
        };

        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")]
            {
                let script_path = script_path.to_str().unwrap();
                let script_path = script_path.replace('\\', "\\\\");
                tera_context.insert("script_path", &script_path);
            } else {
                tera_context.insert("script_path", &script_path);
            }
        }

        // Interpreter is a list, because sometimes there is need to pass extra arguments to the
        // interpreter, such as the /C option in the batch case
        let script_runner = if let Some(script_runner) = &self.script_runner {
            script_runner
        } else {
            DEFAULT_SCRIPT_RUNNER
        };

        let script_runner_template_name = format!("tasks.{task_name}.script_runner");
        tera_instance.add_raw_template(&script_runner_template_name, script_runner)?;

        let script_runner = tera_instance.render(&script_runner_template_name, tera_context)?;
        let script_runner_values = split_command(&script_runner);
        let script_runner_values = expand_args(&script_runner_values, env);
        let script_runner_values: Vec<&str> =
            script_runner_values.iter().map(|s| s.as_ref()).collect();
        let program = script_runner_values[0];
        let args = &script_runner_values[1..];

        let mut command = Command::new(program);

        // The script runner might not contain the actual script path, but we just leave it as a feature ;)
        command.args(args);

        self.set_command_basics(&mut command, mom_file, env)?;

        println!("{}", format!("{task_name}: {script_runner}").mom_info());
        println!("{}", "Script Begin:".mom_info());
        println!("{}", script.color(INFO_COLOR));
        println!("{}", "Script End.".mom_info());

        self.spawn_command(&mut command, dry_run)
    }
}
