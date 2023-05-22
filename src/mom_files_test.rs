use super::*;
use assert_fs::TempDir;
use std::fs::File;
use std::io::Write;

#[test]
fn test_mom_file_invalid_path() {
    let cnfg = MomFile::deserialize_from_path(Path::new("non_existent"));
    assert!(cnfg.is_err());

    let cnfg = MomFile::deserialize_from_path(Path::new("non_existent.ext"));
    assert!(cnfg.is_err());

    let cnfg = MomFile::deserialize_from_path(Path::new("non_existent.yml"));
    assert!(cnfg.is_err());
}

#[test]
fn test_mom_file_read() {
    let tmp_dir = TempDir::new().unwrap();

    let dot_env_path = tmp_dir.path().join(".env");
    let mut dot_env_file = File::create(dot_env_path.as_path()).unwrap();
    dot_env_file
        .write_all(
            r#"VALUE_OVERRIDE=OLD_VALUE
OTHER_VALUE=HELLO
"#
            .as_bytes(),
        )
        .unwrap();

    let project_config_path = tmp_dir.path().join("mom.root.yaml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
            r#"
version: 1

dotenv: ".env"
env:
  VALUE_OVERRIDE: NEW_VALUE
tasks:
  hello_local:
    script: echo hello local
        "#
            .as_bytes(),
        )
        .unwrap();
    let mom_file = MomFile::from_path(project_config_path).unwrap();
    assert!(mom_file.has_task("hello_local"));
    assert_eq!(
        mom_file.common.env.get("VALUE_OVERRIDE").unwrap(),
        "NEW_VALUE"
    );
    assert_eq!(mom_file.common.env.get("OTHER_VALUE").unwrap(), "HELLO");
}

#[test]
fn test_mom_file_flatten_task() {
    let mom_file = MomFile::from_str(
        r#"
    version: 1
    
    tasks:
        test:
            script: echo hello
            windows:
                script: echo hello windows
            macos:
                script: echo hello macos
            linux:
                script: echo hello linux
    "#,
    )
    .unwrap();

    let task = mom_file.tasks.get("test");
    assert!(task.is_some());
    assert_eq!(task.unwrap().script().unwrap(), "echo hello");

    let task = mom_file.tasks.get("test.windows");
    assert!(task.is_some());
    assert_eq!(task.unwrap().script().unwrap(), "echo hello windows");

    let task = mom_file.tasks.get("test.macos");
    assert!(task.is_some());
    assert_eq!(task.unwrap().script().unwrap(), "echo hello macos");
}

#[test]
fn test_mom_file_get_task() {
    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    task_1:
        script: echo hello

    task_2:
        script: echo hello again

    task_3:
        script: echo hello again
        private: true

"#,
    )
    .unwrap();

    let task_nam = mom_file.clone_task("task_1");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_1");

    let task_nam = mom_file.clone_task("task_2");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_2");

    let task_nam = mom_file.clone_task("task_3");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_3");
}

#[test]
fn test_mom_file_get_non_private_task() {
    let mom_file = MomFile::from_str(
        r#"
version: 1
    
tasks:
    task_1:
        script: echo hello
    
    task_2:
        script: echo hello again

    task_3:
        script: echo hello again
        private: true

"#,
    )
    .unwrap();

    let task_nam = mom_file.clone_public_task("task_1");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_1");

    let task_nam = mom_file.clone_public_task("task_2");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_2");

    let task_nam = mom_file.clone_public_task("task_3");
    assert!(task_nam.is_none());
}

#[test]
fn test_task_circular_dependencies_return_error() {
    let mom_file = MomFile::from_str(
        r#"
version: 1
        
tasks:
    task_1:
        script: echo hello
        extend:
            - task_2
        
    task_2:
        script: echo hello again
        extend:
            - task_1
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();

    // Can be either task_1 or task_2
    assert!(err
        .to_string()
        .starts_with("Found a cyclic dependency for task: task_"));

    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    task_1:
        cmds:
            - task: task_2

    task_2:
        cmds:
            - task:
                extend: task_1
                cmds:
                    - some command
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();

    // Can be either task_1 or task_2
    assert!(err
        .to_string()
        .starts_with("Found a cyclic dependency for task: task_"));
}

#[test]
fn test_inherit_non_existing_task_return_err() {
    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    task_1:
        script: echo hello
        extend: task_2
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();
    assert!(err
        .to_string()
        .contains("Task task_1 cannot inherit from non-existing task task_2"));
}

#[test]
fn test_valid_task_name() {
    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    "-invalid_task_name":
        script: echo hello
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();
    assert!(err
        .to_string()
        .contains("Invalid task name `-invalid_task_name`"));

    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    "invalid:task_name":
        script: echo hello
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();
    assert!(err
        .to_string()
        .contains("Invalid task name `invalid:task_name`"));

    let mom_file = MomFile::from_str(
        r#"
version: 1

tasks:
    "":
        script: echo hello
"#,
    );
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();
    assert!(err.to_string().contains("Invalid task name ``"));
}
