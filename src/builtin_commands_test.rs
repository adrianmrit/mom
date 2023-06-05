use super::*;

#[test]
fn test_echo_command() {
    let args = vec!["Hello", "World"];
    let result = echo_command(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ());
}
