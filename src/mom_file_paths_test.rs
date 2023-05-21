use std::{fs::File, io::Write};

use assert_fs::TempDir;

use crate::{
    mom_file_paths::{MomFilePaths, SingleMomFilePath},
    mom_files_container::MomFilesContainer,
};

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

    let inner_dir = tmp_dir.path().join("inner");
    std::fs::create_dir(&inner_dir).unwrap();
    let mut inner_mom_file = File::create(inner_dir.join("mom.yml").as_path()).unwrap();
    inner_mom_file
        .write_all(
            r#"
version: 1

tasks:
    hello_inner:
        script: echo hello inner
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
    let mut paths: Box<MomFilePaths> = MomFilePaths::new(&inner_dir);
    let inner_path = paths.next().unwrap();
    let local_path = paths.next().unwrap();
    let regular_path = paths.next().unwrap();
    let project_path = paths.next().unwrap();

    assert!(paths.next().is_none());

    mom_files.read_mom_file(inner_path).unwrap();
    mom_files.read_mom_file(local_path).unwrap();
    mom_files.read_mom_file(regular_path).unwrap();
    mom_files.read_mom_file(project_path).unwrap();

    assert!(!mom_files.has_task("non_existent"));
    assert!(mom_files.has_task("hello_inner"));
    assert!(mom_files.has_task("hello_project"));
    assert!(mom_files.has_task("hello"));
    assert!(mom_files.has_task("hello_local"));
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
