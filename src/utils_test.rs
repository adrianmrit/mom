use super::*;
use assert_fs::TempDir;
use std::env;
use std::fs::File;
use std::io::Write;

#[test]
fn test_read_dotenv_not_found() {
    let env_file_path = env::current_dir().unwrap().join("non_existent.env");
    let env_map = read_env_file(&env_file_path).unwrap_err();
    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            let expected_error: &str = "The system cannot find the file specified. (os error 2)";
        } else {
            let expected_error: &str = "No such file or directory (os error 2)";
        }
    }
    assert_eq!(
        env_map.to_string(),
        format!(
            "Failed to read env file at {}: {}",
            env_file_path.display(),
            expected_error
        )
    );
}

#[test]
fn test_read_env_file_invalid() {
    let tmp_dir = TempDir::new().unwrap();
    let env_file_path = tmp_dir.join(".env");
    let mut file = File::create(&env_file_path).unwrap();
    file.write_all(r#"INVALID_ENV_FILE"#.as_bytes()).unwrap();
    let env_map = read_env_file(&env_file_path).unwrap_err();
    let expected_err = format!("Failed to parse env file at {}: ", env_file_path.display());
    assert!(env_map.to_string().contains(&expected_err),);
}

#[test]
fn test_read_env_file() {
    let tmp_dir = TempDir::new().unwrap();
    let env_file_path = tmp_dir.join(".env");
    let mut file = File::create(&env_file_path).unwrap();
    file.write_all(
        r#"
    TEST_VAR=test_value
    "#
        .as_bytes(),
    )
    .unwrap();
    let env_map = read_env_file(&env_file_path).unwrap();
    assert_eq!(env_map.get("TEST_VAR"), Some(&"test_value".to_string()));
}

#[test]
fn test_get_path_relative_to_base() {
    let base = "/home/user";
    let path = "test";
    let path = get_path_relative_to_base(base, path);
    assert_eq!(path, PathBuf::from("/home/user/test"));

    let base = "/home/user";
    let path = "/test";
    let path = get_path_relative_to_base(base, path);
    assert_eq!(path, PathBuf::from("/test"));

    let base = "/home/user";
    let path = ".";
    let path = get_path_relative_to_base(base, path);
    assert_eq!(path, PathBuf::from("/home/user"));

    let base = "/home/user";
    let path = "./hello";
    let path = get_path_relative_to_base(base, path);
    assert_eq!(path, PathBuf::from("/home/user/hello"));
}

#[test]
fn test_working_directory() {
    let base = "/home/user";
    let path = "test";
    let path = get_working_directory(base, path);
    assert_eq!(path, PathBuf::from("/home/user/test"));

    let base = "/home/user";
    let path = "";

    let path = get_working_directory(base, path);
    assert_eq!(path, current_dir().unwrap());

    let base = "/home/user";
    let path = ".";
    let path = get_working_directory(base, path);
    assert_eq!(path, PathBuf::from("/home/user"));
}

#[test]
fn test_split_command() {
    let command = "echo \"Hello World\"";
    let args = split_command(command);
    assert_eq!(args, vec!["echo", "Hello World"]);

    let command = "echo \"Hello World\" \"Hello World\"";
    let args = split_command(command);
    assert_eq!(args, vec!["echo", "Hello World", "Hello World"]);

    let command = "echo Hello\\ World \"Hello \\\"World\"";
    let args = split_command(command);
    assert_eq!(args, vec!["echo", "Hello World", "Hello \"World"]);

    let command = "echo Hello \"World\" \"--param\" \"--param=something\"\n";
    let args = split_command(command);
    assert_eq!(
        args,
        vec!["echo", "Hello", "World", "--param", "--param=something"]
    );
}

#[test]
fn test_join_commands() {
    let commands: Vec<String> = vec!["echo", "Hello World"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let command = join_commands(&commands);
    assert_eq!(command, "echo \"Hello World\"");

    let commands: Vec<String> = vec!["echo", "Hello World", "Hello World"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let command = join_commands(&commands);
    assert_eq!(command, "echo \"Hello World\" \"Hello World\"");

    let commands: Vec<String> = vec!["echo", "Hello World", "Hello \"World"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let command = join_commands(&commands);
    assert_eq!(command, "echo \"Hello World\" \"Hello \\\"World\"");

    let commands: Vec<String> = vec!["echo", "Hello", "World", "--param", "--param=something"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let command = join_commands(&commands);
    assert_eq!(command, "echo Hello World --param --param=something");
}

#[test]
fn test_expand_args() {
    let args = vec![
        "val",
        "${TEST_VAR}",
        "$TEST_VAR",
        "${TEST_VAR}val",
        "$ENV_VAR",
        "${ENV_VAR}",
        "${MOM_NON_EXISTENT_VAR}",
        "${MOM_NON_EXISTENT_VAR}hello",
        "~",
        "~/",
        "~val",
        "~/val",
    ];
    let envs: HashMap<String, String> =
        HashMap::from_iter(vec![("TEST_VAR".to_string(), "test_value".to_string())]);
    // set the env var
    env::set_var("ENV_VAR", "env_value");
    let home_dir = HOME_DIR.as_str();
    let home_dir_slash = format!("{}/", home_dir);
    let home_dir_slash_val = format!("{}/val", home_dir);
    let expected = vec![
        "val",
        "test_value",
        "test_value",
        "test_valueval",
        "env_value",
        "env_value",
        "",
        "hello",
        &home_dir,
        &home_dir_slash,
        "~val",
        &home_dir_slash_val,
    ];
    let expanded_args = expand_args(&args, &envs);
    assert_eq!(expanded_args, expected);
}
