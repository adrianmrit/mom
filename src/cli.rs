#[cfg(test)]
#[path = "cli_test.rs"]
mod cli_test;

use clap::ArgAction;
use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{env, fmt};

use crate::args::ArgsContext;
use crate::mom_file_paths::{GlobalMomFilePath, MomFilePaths, PathIterator, SingleMomFilePath};
use crate::mom_files::MomFile;
use crate::mom_files_container::MomFilesContainer;
use crate::print_utils::MomOutput;
use crate::types::DynErrResult;

const HELP: &str = "For documentation check https://github.com/adrianmrit/mom.";

/// Holds the data for running the given task.
struct TaskSubcommand {
    /// Task to run, if given
    pub(crate) task: String,
    /// Args to run the command with
    pub(crate) args_context: ArgsContext,
}

/// Enum of available mom file versions
#[derive(Deserialize, Serialize)]
pub(crate) enum Version {
    #[serde(rename = "1")]
    V1,
}

/// Argument errors
#[derive(Debug, PartialEq, Eq)]
enum ArgsError {
    /// Raised when no task to run is given
    MissingTaskArg,
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArgsError::MissingTaskArg => write!(f, "No task was given."),
        }
    }
}

impl Error for ArgsError {
    fn description(&self) -> &str {
        match *self {
            ArgsError::MissingTaskArg => "no task given",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

/// Sets the color when printing the task name
fn colorize_task_name(val: &str) -> ColoredString {
    val.bright_cyan()
}

/// Sets the color when printing the mom file path
fn colorize_mom_file_path(val: &str) -> ColoredString {
    val.bright_blue()
}

struct Mom {
    mom_files: MomFilesContainer,
}

impl Mom {
    /// Creates a new instance of `Mom`
    fn new() -> Self {
        Self {
            mom_files: MomFilesContainer::new(),
        }
    }

    fn get_mom_file_lock(&mut self, path: PathBuf) -> DynErrResult<Arc<Mutex<MomFile>>> {
        let mom_file_ptr = match self.mom_files.read_mom_file(path.clone()) {
            Ok(val) => val,
            Err(e) => {
                let e = format!("{}:\n{}", &path.to_string_lossy().red(), e);
                return Err(e.into());
            }
        };
        Ok(mom_file_ptr)
    }

    /// prints mom file paths and their tasks
    fn print_tasks_list(&mut self, paths: PathIterator) -> DynErrResult<()> {
        let mut found = false;
        for path in paths {
            found = true;
            let mom_file_ptr = self.get_mom_file_lock(path.clone())?;
            let mom_file_lock = mom_file_ptr.lock().unwrap();

            println!("{}:", colorize_mom_file_path(&path.to_string_lossy()));

            let mut task_names = mom_file_lock.get_public_task_names();
            task_names.sort();
            if task_names.is_empty() {
                println!("  {}", "No tasks found.".red());
            } else {
                for task in task_names {
                    println!(" - {}", colorize_task_name(task));
                }
            }
        }
        if !found {
            println!("No mom files found.");
        }
        Ok(())
    }

    /// Prints help for the given task
    fn print_task_info(&mut self, paths: PathIterator, task: &str) -> DynErrResult<()> {
        for path in paths {
            let mom_file_ptr = self.get_mom_file_lock(path.clone())?;
            let mom_file_lock = mom_file_ptr.lock().unwrap();

            let task = mom_file_lock.clone_task(task);

            match task {
                Some(task) => {
                    println!("{}:", colorize_mom_file_path(&path.to_string_lossy()));
                    print!(" - {}", colorize_task_name(task.get_name()));
                    if task.is_private() {
                        print!(" {}", "(private)".red());
                    }
                    println!();
                    let prefix = "     ";
                    match task.get_help().trim() {
                        "" => println!("{}{}", prefix, "No help to display".yellow()),
                        help => {
                            //                 " -   "  Two spaces after the dash
                            let help_lines: Vec<&str> = help.lines().collect();
                            println!(
                                "{}{}",
                                prefix,
                                help_lines.join(&format!("\n{}", prefix)).green()
                            )
                        }
                    }
                    return Ok(());
                }
                None => continue,
            }
        }
        Err(format!("Task {} not found", task).into())
    }

    /// Runs the given task
    fn run_task(
        &mut self,
        paths: PathIterator,
        task: &str,
        args: &ArgsContext,
        dry_run: bool,
    ) -> DynErrResult<()> {
        for path in paths {
            let mom_file_ptr = self.get_mom_file_lock(path.clone())?;
            let mom_file_lock = mom_file_ptr.lock().unwrap();

            let task = mom_file_lock.clone_public_task(task);

            match task {
                Some(task) => {
                    println!("{}", &path.to_string_lossy().mom_info());
                    return match task.run(args, &mom_file_lock, dry_run) {
                        Ok(val) => Ok(val),
                        Err(e) => {
                            let e = format!("{}:\n{}", &path.to_string_lossy().red(), e);
                            Err(e.into())
                        }
                    };
                }
                None => continue,
            }
        }
        Err(format!("Task {} not found", task).into())
    }
}

// TODO: Handle
impl TaskSubcommand {
    /// Returns a new TaskSubcommand
    pub(crate) fn new(args: &clap::ArgMatches) -> Result<TaskSubcommand, ArgsError> {
        let (task_name, task_args) = match args.subcommand() {
            None => return Err(ArgsError::MissingTaskArg),
            Some(command) => command,
        };

        Ok(TaskSubcommand {
            task: String::from(task_name),
            args_context: ArgsContext::from(task_args.clone()),
        })
    }
}

/// Executes the program. If errors are encountered during the execution these
/// are returned immediately. The wrapping method needs to take care of formatting
/// and displaying these errors appropriately.
pub fn exec() -> DynErrResult<()> {
    let app = clap::Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .after_help(HELP)
        .allow_external_subcommands(true)
        .arg(
            clap::Arg::new("list")
                .short('l')
                .long("list")
                .help("Lists configuration files that can be reached from the current directory")
                .action(ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("list-tasks")
                .short('t')
                .long("list-tasks")
                .help("Lists tasks")
                .conflicts_with_all(["task-info"])
                .action(ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("task-info")
                .short('i')
                .long("task-info")
                .action(ArgAction::Set)
                .help("Displays information about the given task")
                .value_name("TASK"),
        )
        .arg(
            clap::Arg::new("dry")
                .long("dry")
                .action(ArgAction::SetTrue)
                .help("Runs the task in dry mode, i.e. without executing any commands"),
        )
        .arg(
            clap::Arg::new("file")
                .short('f')
                .long("file")
                .action(ArgAction::Set)
                .help("Search for tasks in the given file")
                .value_name("FILE"),
        )
        .arg(
            clap::Arg::new("global")
                .short('g')
                .long("global")
                .help("Search for tasks in ~/mom/mom.global.{yml,yaml}")
                .conflicts_with_all(["file"])
                .action(ArgAction::SetTrue),
        );
    let matches = app.get_matches();

    let current_dir = env::current_dir()?;
    let mut mom = Mom::new();

    let mom_file_paths: PathIterator = match matches.get_one::<String>("file") {
        None => match matches.get_one::<bool>("global").cloned().unwrap_or(false) {
            true => GlobalMomFilePath::new(),
            false => MomFilePaths::new(&current_dir),
        },
        Some(file_path) => SingleMomFilePath::new(file_path),
    };

    let dry_run = matches.get_one::<bool>("dry").cloned().unwrap_or(false);

    if matches
        .get_one::<bool>("list-tasks")
        .cloned()
        .unwrap_or(false)
    {
        mom.print_tasks_list(mom_file_paths)?;
        return Ok(());
    };

    if let Some(task_name) = matches.get_one::<String>("task-info") {
        mom.print_task_info(mom_file_paths, task_name)?;
        return Ok(());
    };

    if matches.get_one::<bool>("list").cloned().unwrap_or(false) {
        for path in mom_file_paths {
            println!("{}", colorize_mom_file_path(&path.to_string_lossy()));
        }
        return Ok(());
    }

    let task_command = TaskSubcommand::new(&matches)?;

    mom.run_task(
        mom_file_paths,
        &task_command.task,
        &task_command.args_context,
        dry_run,
    )
}
