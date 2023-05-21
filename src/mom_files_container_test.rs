use std::{fs::File, io::Write};

use assert_fs::TempDir;

use crate::mom_files_container::MomFilesContainer;

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

    let task = config_file.clone_task("t1").unwrap();
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
