use super::*;

#[test]
fn test_from_err_to_task_error() {
    let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
    let task_err: TaskError = err.into();
    let expected = TaskError::RuntimeError(String::from("test"));
    assert_eq!(task_err, expected);

    assert_eq!(task_err.to_string(), "Runtime error:\ntest");
}

#[test]
fn test_from_err_with_inner_to_task_error() {
    let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
    let err = std::io::Error::new(std::io::ErrorKind::Other, err);
    let task_err: TaskError = err.into();
    let expected = TaskError::RuntimeError(String::from("test"));
    assert_eq!(task_err, expected);

    assert_eq!(task_err.to_string(), "Runtime error:\ntest");
}

#[test]
fn test_from_tera_err_wit_cause_to_task_error() {
    // Tera errors have a cause, so we check that the cause is properly formatted
    let err = tera::Error::from(std::io::Error::new(std::io::ErrorKind::Other, "test"));
    let task_err: TaskError = err.into();
    let expected = TaskError::ConfigError(String::from(
        "Io error while writing rendered value to output: Other\nCaused by: test",
    ));
    assert_eq!(task_err, expected);

    assert_eq!(task_err.to_string(), "Improperly configured:\nIo error while writing rendered value to output: Other\nCaused by: test");
}

#[test]
fn test_from_aware_task_error_to_task_error() {
    let err = AwareTaskError::new("test", TaskError::ConfigError(String::from("test")));
    let task_err: TaskError = err.into();
    let expected = TaskError::RuntimeError(String::from(
        "Task `test` failed:\nImproperly configured:\ntest",
    ));
    assert_eq!(task_err, expected);

    assert_eq!(
        task_err.to_string(),
        "Runtime error:\nTask `test` failed:\nImproperly configured:\ntest"
    );
}

#[test]
fn test_not_found_task_err() {
    let err = TaskError::NotFound(String::from("test"));
    assert_eq!(err.to_string(), "Task `test` not found.");
}
