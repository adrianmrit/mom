use std::fs::File;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::{predicate, PredicateBooleanExt};

#[test]
fn test_list() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let tmp_dir_path = tmp_dir.path();
    File::create(tmp_dir_path.join("mom.private.yml"))?;
    File::create(tmp_dir_path.join("mom.root.yml"))?;
    File::create(tmp_dir_path.join("mom.other.yml"))?;

    let expected_private = format!(
        "{tmp_dir}\n",
        tmp_dir = tmp_dir_path.join("mom.private.yml").to_str().unwrap()
    );
    let expected_root = format!(
        "{tmp_dir}\n",
        tmp_dir = tmp_dir_path.join("mom.root.yml").to_str().unwrap()
    );
    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir_path);
    cmd.arg("--list");
    cmd.assert().success().stdout(
        predicate::str::contains(expected_private).and(predicate::str::ends_with(expected_root)),
    );
    Ok(())
}
