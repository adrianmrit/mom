use super::*;
use assert_fs::TempDir;
use std::fs::File;
use std::io::Write;

#[test]
fn test_discovery() {
    let tmp_dir = TempDir::new().unwrap();
    let project_config_path = tmp_dir.path().join("mom.root.yml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
            r#"
    version: 1

    tasks:
        hello_project:
            script: "echo hello project"
    "#
            .as_bytes(),
        )
        .unwrap();

    let config_path = tmp_dir.path().join("mom.yaml");
    let mut mom_file = File::create(config_path.as_path()).unwrap();
    mom_file
        .write_all(
            r#"
    version: 1

    tasks:
        hello:
            script: echo hello
    "#
            .as_bytes(),
        )
        .unwrap();

    let local_config_path = tmp_dir.path().join("mom.private.yaml");
    let mut local_file = File::create(local_config_path.as_path()).unwrap();
    local_file
        .write_all(
            r#"
    version: 1

    tasks:
        hello_local:
            script: echo hello local
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut mom_files = MomFilesContainer::new();
    let mut paths: Box<MomFilePaths> = MomFilePaths::new(&tmp_dir.path());
    let local_path = paths.next().unwrap();
    let regular_path = paths.next().unwrap();
    let project_path = paths.next().unwrap();

    assert!(paths.next().is_none());

    mom_files.read_mom_file(local_path).unwrap();
    mom_files.read_mom_file(regular_path).unwrap();
    mom_files.read_mom_file(project_path).unwrap();

    assert!(!mom_files.has_task("non_existent"));
    assert!(mom_files.has_task("hello_project"));
    assert!(mom_files.has_task("hello"));
    assert!(mom_files.has_task("hello_local"));
}

#[test]
fn test_extend() {
    let tmp_dir = TempDir::new().unwrap();
    let source_mom_file = tmp_dir.path().join("mom.source.yml");
    let mut source_mom_file = File::create(source_mom_file.as_path()).unwrap();
    source_mom_file
        .write_all(
            r#"
    version: 1

    env:
        EVAR1: ROOT_EVAL1
        EVAR2: ROOT_EVAL2
    
    vars:
        VAR1: ROOT_VAL1
        VAR2: ROOT_VAL2

    tasks:
        t1:
            script: "echo hello t1"
        t2:
            script: "echo hello t2"
    "#
            .as_bytes(),
        )
        .unwrap();

    let target_mom_file_path = tmp_dir.path().join("mom.target.yml");
    let mut target_mom_file = File::create(target_mom_file_path.as_path()).unwrap();
    target_mom_file
        .write_all(
            r#"
    version: 1

    extend: mom.source.yml

    env:
        EVAR1: ROOT_EVAL1.1
        EVAR3: ROOT_EVAL3
    
    vars:
        VAR1: ROOT_VAL1.1
        VAR3: ROOT_VAL3
    
    tasks:
        t1:
            script: "echo hello t1.1"
        t3:
            script: "echo hello t3"
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut mom_files = MomFilesContainer::new();

    let config_file = mom_files.read_mom_file(target_mom_file_path).unwrap();
    let config_file = config_file.lock().unwrap();
    let task_names = config_file.get_public_task_names();
    assert_eq!(task_names.len(), 3);
    assert!(task_names.contains(&"t1"));
    assert!(task_names.contains(&"t2"));
    assert!(task_names.contains(&"t3"));

    let task = config_file.get_task("t1").unwrap();
    assert_eq!(task.script().unwrap(), "echo hello t1.1");

    let env = &config_file.common.env;
    assert_eq!(env.get("EVAR1").unwrap(), "ROOT_EVAL1.1");
    assert_eq!(env.get("EVAR2").unwrap(), "ROOT_EVAL2");
    assert_eq!(env.get("EVAR3").unwrap(), "ROOT_EVAL3");

    let vars = &config_file.common.vars;
    assert_eq!(vars.get("VAR1").unwrap(), "ROOT_VAL1.1");
    assert_eq!(vars.get("VAR2").unwrap(), "ROOT_VAL2");
    assert_eq!(vars.get("VAR3").unwrap(), "ROOT_VAL3");
}

#[test]
fn test_extend_cyclic_dependency() {
    let tmp_dir = TempDir::new().unwrap();
    let source_mom_file = tmp_dir.path().join("mom.source.yml");
    let mut source_mom_file = File::create(source_mom_file.as_path()).unwrap();
    source_mom_file
        .write_all(
            r#"
    version: 1

    extend: mom.target.yml

    tasks:
        t1:
            script: "echo hello t1"
    "#
            .as_bytes(),
        )
        .unwrap();

    let target_mom_file_path = tmp_dir.path().join("mom.target.yml");
    let mut target_mom_file = File::create(target_mom_file_path.as_path()).unwrap();
    target_mom_file
        .write_all(
            r#"
    version: 1

    extend: mom.source.yml

    tasks:
        t1:
            script: "echo hello t1.1"
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut mom_files = MomFilesContainer::new();

    let config_file = mom_files.read_mom_file(target_mom_file_path);
    assert!(config_file.is_err());

    let err = config_file.err().unwrap();
    assert!(err
        .to_string()
        .starts_with("Found a cyclic dependency for mom file: "));
}

#[test]
fn test_discovery_given_file() {
    let tmp_dir = TempDir::new().unwrap();
    let sample_mom_file_path = tmp_dir.path().join("sample.mom.yml");
    let mut sample_mom_file = File::create(sample_mom_file_path.as_path()).unwrap();
    sample_mom_file
        .write_all(
            r#"
version: 1

tasks:
    hello_project:
        script: echo hello project
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut mom_files = MomFilesContainer::new();
    let mut paths = SingleMomFilePath::new(&sample_mom_file_path);
    let sample_path = paths.next().unwrap();
    assert!(paths.next().is_none());

    mom_files.read_mom_file(sample_path).unwrap();

    assert!(mom_files.has_task("hello_project"));
}

#[test]
fn test_mom_file_invalid_path() {
    let cnfg = MomFile::extract(Path::new("non_existent"));
    assert!(cnfg.is_err());

    let cnfg = MomFile::extract(Path::new("non_existent.ext"));
    assert!(cnfg.is_err());

    let cnfg = MomFile::extract(Path::new("non_existent.yml"));
    assert!(cnfg.is_err());
}

#[test]
fn test_container_read_config_error() {
    let tmp_dir = TempDir::new().unwrap();
    let project_config_path = tmp_dir.path().join("mom.root.yml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
            r#"
    some invalid condig
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut mom_files = MomFilesContainer::default();
    let result = mom_files.read_mom_file(project_config_path);

    assert!(result.is_err());
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
    let mom_file = MomFile::load(project_config_path).unwrap();
    assert!(mom_file.has_task("hello_local"));
    assert_eq!(
        mom_file.common.env.get("VALUE_OVERRIDE").unwrap(),
        "NEW_VALUE"
    );
    assert_eq!(mom_file.common.env.get("OTHER_VALUE").unwrap(), "HELLO");
}

#[test]
fn test_mom_file_flatten_task() {
    let tmp_dir = TempDir::new().unwrap();

    let project_config_path = tmp_dir.path().join("mom.root.yaml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
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
"#
            .as_bytes(),
        )
        .unwrap();
    let mom_file = MomFile::load(project_config_path).unwrap();

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
    let tmp_dir = TempDir::new().unwrap();

    let project_config_path = tmp_dir.path().join("mom.root.yaml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
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

        "#
            .as_bytes(),
        )
        .unwrap();
    let mom_file = MomFile::load(project_config_path).unwrap();

    let task_nam = mom_file.get_task("task_1");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_1");

    let task_nam = mom_file.get_task("task_2");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_2");

    let task_nam = mom_file.get_task("task_3");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_3");
}

#[test]
fn test_mom_file_get_non_private_task() {
    let tmp_dir = TempDir::new().unwrap();

    let project_config_path = tmp_dir.path().join("mom.root.yaml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
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

        "#
            .as_bytes(),
        )
        .unwrap();
    let mom_file = MomFile::load(project_config_path).unwrap();

    let task_nam = mom_file.get_public_task("task_1");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_1");

    let task_nam = mom_file.get_public_task("task_2");
    assert!(task_nam.is_some());
    assert_eq!(task_nam.unwrap().get_name(), "task_2");

    let task_nam = mom_file.get_public_task("task_3");
    assert!(task_nam.is_none());
}

#[test]
fn test_circular_dependencies_return_error() {
    let tmp_dir = TempDir::new().unwrap();

    let project_config_path = tmp_dir.path().join("mom.root.yaml");
    let mut project_mom_file = File::create(project_config_path.as_path()).unwrap();
    project_mom_file
        .write_all(
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
        "#
            .as_bytes(),
        )
        .unwrap();

    let mom_file = MomFile::load(project_config_path);
    assert!(mom_file.is_err());

    let err = mom_file.err().unwrap();

    // Can be either task_1 or task_2
    assert!(err
        .to_string()
        .starts_with("Found a cyclic dependency for task: task_"));
}
