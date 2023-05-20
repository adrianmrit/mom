use std::error::Error;
use std::fmt;

/// Represents an error that can occur in a task
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskError {
    /// Raised when there is an error running a task
    RuntimeError(String),
    /// Raised when the task is improperly configured
    ConfigError(String),
    NotFound(String),
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TaskError::RuntimeError(ref reason) => {
                write!(f, "Runtime error:\n{}", reason)
            }
            TaskError::ConfigError(ref reason) => {
                write!(f, "Improperly configured:\n{}", reason)
            }
            TaskError::NotFound(ref name) => {
                write!(f, "Task `{}` not found.", name)
            }
        }
    }
}

impl Error for TaskError {}

impl From<tera::Error> for TaskError {
    fn from(err: tera::Error) -> TaskError {
        let mut full_error = err.to_string();
        let mut source = err.source();
        while let Some(inner) = source {
            full_error.push_str(&format!("\nCaused by: {}", inner));
            source = inner.source();
        }
        TaskError::ConfigError(full_error)
    }
}

impl From<std::io::Error> for TaskError {
    fn from(err: std::io::Error) -> TaskError {
        TaskError::RuntimeError(err.to_string())
    }
}

// We convert back to TaskError in case a subtask fails
// TODO: Use source() instead
impl From<AwareTaskError> for TaskError {
    fn from(err: AwareTaskError) -> TaskError {
        TaskError::RuntimeError(err.to_string())
    }
}

/// Task error aware of the task name
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AwareTaskError {
    /// Name of the task that failed
    pub(crate) task_name: String,
    /// The error that caused the task to fail
    pub(crate) error: TaskError,
}

impl fmt::Display for AwareTaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Task `{}` failed:\n{}", self.task_name, self.error)
    }
}

impl Error for AwareTaskError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

impl AwareTaskError {
    /// Creates a new AwareTaskError
    /// # Arguments
    /// * `task_name` - Name of the task that failed
    /// * `error` - The error that caused the task to fail
    pub(crate) fn new(task_name: &str, error: TaskError) -> AwareTaskError {
        AwareTaskError {
            task_name: task_name.to_string(),
            error,
        }
    }
}
