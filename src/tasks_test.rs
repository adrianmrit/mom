use super::*;
use crate::errors::{AwareTaskError, TaskError};
use crate::mom_files::MomFile;
use assert_fs::TempDir;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn get_task(
    name: &str,
    definition: &str,
    base_path: Option<&Path>,
) -> Result<Task, Box<dyn std::error::Error>> {
    let mut task: Task = serde_yaml::from_str(definition)?;
    task.setup(name, base_path.unwrap_or_else(|| Path::new("")))?;
    Ok(task)
}

#[test]
fn test_env_inheritance() {
    let tmp_dir = TempDir::new().unwrap();
    let mom_file_path = tmp_dir.join("mom.root.yml");
    let mut file = File::create(&mom_file_path).unwrap();
    file.write_all(
        r#"
version: 1

tasks:
    hello_base:
        env:
            greeting: hello world

    calc_base:
        env:
            one_plus_one: "2"

    hello:
        extend: ["hello_base", "calc_base"]
        script: "echo $greeting, 1+1=$one_plus_one"

    hello.windows:
        extend: ["hello_base", "calc_base"]
        script: "echo %greeting%, 1+1=%one_plus_one%"
    "#
        .as_bytes(),
    )
    .unwrap();

    let mom_file = MomFile::load(mom_file_path).unwrap();

    let task = mom_file.get_task("hello").unwrap();

    let expected = HashMap::from([
        ("greeting".to_string(), "hello world".to_string()),
        ("one_plus_one".to_string(), "2".to_string()),
    ]);
    assert_eq!(task.common.env, expected);
}

#[test]
fn test_args_inheritance() {
    let tmp_dir = TempDir::new().unwrap();
    let mom_file_path = tmp_dir.join("mom.root.yml");
    let mut file = File::create(&mom_file_path).unwrap();
    file.write_all(
        r#"
    version: 1

    tasks:
        bash:
            program: "bash"

        bash_inline:
            extend: bash
            args_extend: "-c"

        hello:
            extend: ["bash_inline"]
            args_extend: echo hello

        hello_2:
            extend: hello
            args: -c "echo hello"
    "#
        .as_bytes(),
    )
    .unwrap();

    let mom_file = MomFile::load(mom_file_path).unwrap();

    let task = mom_file.get_task("hello").unwrap();
    assert_eq!(task.args.as_ref().unwrap(), "-c echo hello");

    let task = mom_file.get_task("hello_2").unwrap();
    assert_eq!(task.args.as_ref().unwrap(), &"-c \"echo hello\"");
}

#[test]
fn test_get_task_help() {
    let tmp_dir = TempDir::new().unwrap();
    let mom_file_path = tmp_dir.join("mom.root.yml");
    let mut file = File::create(&mom_file_path).unwrap();
    file.write_all(
        r#"
version: 1

tasks:
    base:
        help: >
            New lines
            should be
            trimmed

        program: "bash"

    help_inherited:
        extend: base

    no_help:
        program: "bash"

    help_removed:
        extend: base
        help: ""

    help_overriden:
        extend: base
        help: |
            First line
            Second line
    "#
        .as_bytes(),
    )
    .unwrap();

    let mom_file = MomFile::load(mom_file_path).unwrap();

    let task = mom_file.get_task("base").unwrap();
    assert_eq!(task.get_help(), "New lines should be trimmed");

    let task = mom_file.get_task("help_inherited").unwrap();
    assert_eq!(task.get_help(), "New lines should be trimmed");

    let task = mom_file.get_task("no_help").unwrap();
    assert_eq!(task.get_help(), "");

    let task = mom_file.get_task("help_removed").unwrap();
    assert_eq!(task.get_help(), "");

    let task = mom_file.get_task("help_overriden").unwrap();
    assert_eq!(task.get_help(), "First line\nSecond line");
}

#[test]
fn test_read_env() {
    let tmp_dir = TempDir::new().unwrap();
    let project_config_path = tmp_dir.join("mom.root.yml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
            r#"
version: 1

dotenv: [".env"]

tasks:
    test.windows:
        script: "echo %VAR1% %VAR2% %VAR3%"

    test:
        script: "echo $VAR1 $VAR2 $VAR3"

    test_2.windows:
        script: "echo %VAR1% %VAR2% %VAR3%"
        dotenv: [".env_2"]
        env: 
            VAR1: TASK_VAL1

    test_2:
        script: "echo $VAR1 $VAR2 $VAR3"
        dotenv: ".env_2"
        env:
            VAR1: "TASK_VAL1"
            "#
            .as_bytes(),
        )
        .unwrap();

    let mut env_file = File::create(tmp_dir.join(".env").as_path()).unwrap();
    env_file
        .write_all(
            r#"
    VAR1=VAL1
    VAR2=VAL2
    VAR3=VAL3
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut env_file_2 = File::create(tmp_dir.join(".env_2").as_path()).unwrap();
    env_file_2
        .write_all(
            r#"
    VAR1=OTHER_VAL1
    VAR2=OTHER_VAL2
    "#
            .as_bytes(),
        )
        .unwrap();

    let mom_file = MomFile::load(project_config_path).unwrap();

    let task = mom_file.get_task("test").unwrap();
    let env = task.get_env(&mom_file.common.env);

    let expected = HashMap::from([
        ("VAR1".to_string(), "VAL1".to_string()),
        ("VAR2".to_string(), "VAL2".to_string()),
        ("VAR3".to_string(), "VAL3".to_string()),
    ]);
    assert_eq!(env, expected);

    let task = mom_file.get_task("test_2").unwrap();
    let env = task.get_env(&mom_file.common.env);
    let expected = HashMap::from([
        ("VAR1".to_string(), "TASK_VAL1".to_string()),
        ("VAR2".to_string(), "OTHER_VAL2".to_string()),
        ("VAR3".to_string(), "VAL3".to_string()),
    ]);
    assert_eq!(env, expected);
}

#[test]
fn test_validate() {
    let task = get_task(
        "sample",
        r#"
        script: "hello world"
        program: "some_program"
    "#,
        None,
    );
    let expected_error = AwareTaskError::new(
        "sample",
        TaskError::ConfigError(String::from("Cannot set both `script` and `program`.")),
    );
    assert_eq!(task.unwrap_err().to_string(), expected_error.to_string());

    let task = get_task(
        "sample",
        r#"
        script: "something"
        cmds: ["cmd1", "cmd2"]
    "#,
        None,
    );
    let expected_error = AwareTaskError::new(
        "sample",
        TaskError::ConfigError(String::from("Cannot set both `cmds` and `script`.")),
    );
    assert_eq!(task.unwrap_err().to_string(), expected_error.to_string());

    let task = get_task(
        "sample",
        r#"
        program: "sample script"
        cmds: ["some command"]
    "#,
        None,
    );

    let expected_error = AwareTaskError::new(
        "sample",
        TaskError::ConfigError(String::from("Cannot set both `cmds` and `program`.")),
    );
    assert_eq!(task.unwrap_err().to_string(), expected_error.to_string());
}

#[test]
fn test_create_temp_script() {
    let tmp_dir = TempDir::new().unwrap();
    let project_config_path = tmp_dir.join("mom.root.yml");
    let script = "echo hello world";
    let extension = "sh";
    let task_name = "sample";
    let script_path =
        get_temp_script(script, extension, task_name, project_config_path.as_path()).unwrap();
    assert!(script_path.exists());
    assert_eq!(script_path.extension().unwrap(), extension);
    let script_content = fs::read_to_string(script_path).unwrap();
    assert_eq!(script_content, script);

    let extension = "";
    let task_name = "sample2";
    let script_path =
        get_temp_script(script, extension, task_name, project_config_path.as_path()).unwrap();
    assert!(script_path.exists());
    assert!(script_path.extension().is_none());
    let script_content = fs::read_to_string(script_path).unwrap();
    assert_eq!(script_content, script);

    let extension = ".sh";
    let task_name = "sample3";
    let script_path =
        get_temp_script(script, extension, task_name, project_config_path.as_path()).unwrap();
    assert!(script_path.exists());
    assert_eq!(script_path.extension().unwrap(), "sh");
    let script_content = fs::read_to_string(script_path).unwrap();
    assert_eq!(script_content, script);
}

#[test]
fn test_deserialize_command_invalid() {
    let task = get_task(
        "sample",
        r#"
        cmds: 
            - {}
    "#,
        None,
    );
    assert_eq!(
        task.unwrap_err().to_string(),
        // This is a serde error, so doesn't have the task name
        "cmds[0]: missing field `task_name or task` at line 3 column 15"
    );
}
