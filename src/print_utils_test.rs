use super::*;

#[test]
fn test_mom_prefix() {
    let info_prefix = PREFIX.color(INFO_COLOR);
    let warn_prefix = PREFIX.color(WARN_COLOR);
    let error_prefix = PREFIX.color(ERROR_COLOR);

    let output = "\nThis is a test\n\nThis is another test\n\n".to_string();
    let prefix_output = output.mom_just_prefix();
    let expected_output = format!(
        "{PREFIX} \n{PREFIX} This is a test\n{PREFIX} \n{PREFIX} This is another test\n{PREFIX} \n"
    );
    assert_eq!(prefix_output, expected_output);

    let output = "\nThis is a test\n\nThis is another test\n\n".to_string();
    let colored_output = output.mom_prefix_error();
    let expected_output = format!(
        "{error_prefix} \n{error_prefix} This is a test\n{error_prefix} \n{error_prefix} This is another test\n{error_prefix} \n"
    );
    assert_eq!(colored_output, expected_output);

    let output = "This is a test\nThis is another test";
    let colored_output = output.mom_prefix_error();
    let expected_output =
        format!("{error_prefix} This is a test\n{error_prefix} This is another test");
    assert_eq!(colored_output, expected_output);

    let output = "This is a test\nThis is another test";
    let colored_output = output.mom_error();
    let expected_output = format!("{PREFIX} This is a test\n{PREFIX} This is another test")
        .color(ERROR_COLOR)
        .to_string();
    assert_eq!(colored_output, expected_output);

    let colored_text = "This is a test".color(Color::Blue);
    let output = format!("{colored_text}\nThis is another test");
    let colored_output = output.mom_prefix_warn();
    let expected_output =
        format!("{warn_prefix} {colored_text}\n{warn_prefix} This is another test");
    assert_eq!(colored_output, expected_output);

    let output = "This is a test\n";
    let colored_output = output.mom_prefix_info();
    let expected_output = format!("{info_prefix} This is a test\n");
    assert_eq!(colored_output, expected_output);

    let output = "This is a test";
    let colored_output = output.mom_info();
    let expected_output = format!("{PREFIX} This is a test")
        .color(INFO_COLOR)
        .to_string();
    assert_eq!(colored_output, expected_output);

    let output = "\n\n";
    let colored_output = output.mom_prefix_info();
    let expected_output = format!("{info_prefix} \n{info_prefix} \n");
    assert_eq!(colored_output, expected_output);

    let output = "";
    let colored_output = output.mom_prefix(Color::Red);
    let expected_output = "";
    assert_eq!(colored_output, expected_output);
}
