use std::collections::HashMap;

use super::USER_INPUT;

use super::get_tera_instance;

#[test]
fn test_exclude_filter() {
    let mut tera = get_tera_instance(HashMap::new());

    let result = tera
        .render_str(
            r#"{{ [1, 2, 3, 4, 5] | exclude(val=3) }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, "[1, 2, 4, 5]");

    let map: HashMap<String, tera::Value> = HashMap::from_iter(vec![
        ("a".to_string(), 1.into()),
        ("b".to_string(), 2.into()),
        ("c".to_string(), 3.into()),
    ]);

    let mut context = tera::Context::new();
    context.insert("map", &map);

    // Test with an object
    let result = tera
        .render_str(r#"{{ map | exclude(val="b") | json_encode() }}"#, &context)
        .unwrap();
    assert_eq!(result, "{\"a\":1,\"c\":3}");

    // Test with bad input
    let result = tera.render_str(r#"{{ 1 | exclude(val="b") }}"#, &context);
    assert!(result.is_err());

    // Test with missing parameter
    let result = tera.render_str(r#"{{ map | exclude() }}"#, &context);
    assert!(result.is_err());
}

#[test]
fn test_input_function() {
    let mut tera = get_tera_instance(HashMap::new());

    let result = tera
        .render_str(
            r#"{{ input(label="Enter a value", default="something") }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, USER_INPUT);

    // Test with missing parameter
    let result = tera.render_str(r#"{{ input() }}"#, &tera::Context::new());
    assert!(result.is_err());

    // Test with non string default
    let result = tera.render_str(
        r#"{{ input(label="Enter a value", default=1) }}"#,
        &tera::Context::new(),
    );
    assert!(result.is_err());
}

#[test]
fn test_get_env() {
    let env: HashMap<String, String> =
        HashMap::from_iter(vec![("TEST_VAR".to_string(), "test_value".to_string())]);

    let mut tera = get_tera_instance(env);

    let result = tera
        .render_str(r#"{{ get_env(name="TEST_VAR") }}"#, &tera::Context::new())
        .unwrap();

    assert_eq!(result, "test_value");

    // Test missing no default
    let result = tera.render_str(
        r#"{{ get_env(name="MOM_NON_EXISTENT") }}"#,
        &tera::Context::new(),
    );
    assert!(result.is_err());

    // Test with default
    let result = tera.render_str(
        r#"{{ get_env(name="MOM_NON_EXISTENT", default="other") }}"#,
        &tera::Context::new(),
    );
    assert_eq!(result.unwrap(), "other");

    // Test with missing parameter
    let result = tera.render_str(r#"{{ get_env() }}"#, &tera::Context::new());
    assert!(result.is_err());

    // Test with non string default
    let result = tera.render_str(
        r#"{{ get_env(name="MOM_NON_EXISTENT", default=1) }}"#,
        &tera::Context::new(),
    );
    assert_eq!(result.unwrap(), "1");

    // Test with non string name
    let result = tera.render_str(
        r#"{{ get_env(name=1, default="other") }}"#,
        &tera::Context::new(),
    );
    assert!(result.is_err());

    // Test system env
    let env_var_value = "test_value";
    std::env::set_var("MOM_SYSTEM_TEST_VAR", env_var_value);
    let result = tera
        .render_str(
            r#"{{ get_env(name="MOM_SYSTEM_TEST_VAR") }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, env_var_value);
}

#[test]
#[cfg(windows)]
fn test_shell_escape_filter() {
    let mut tera = get_tera_instance(HashMap::new());

    let result = tera
        .render_str(r#"{{ "test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "test");

    let result = tera
        .render_str(r#"{{ "test test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "\"test test\"");

    let result = tera
        .render_str(r#"{{ "test'test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "test'test");

    let result = tera
        .render_str(r#"{{ 'test"test' | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "\"test\\\"test\"");

    let result = tera
        .render_str(
            r#"{{ ["test", "test test", "test'test", 'test"test'] | shell_escape }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, "test \"test test\" test'test \"test\\\"test\"");
}

#[test]
#[cfg(unix)]
fn test_shell_escape_filter() {
    let mut tera = get_tera_instance(HashMap::new());

    let result = tera
        .render_str(r#"{{ "test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "test");

    let result = tera
        .render_str(r#"{{ "test test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "'test test'");

    let result = tera
        .render_str(r#"{{ "test'test" | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "'test'\\''test'");

    let result = tera
        .render_str(r#"{{ 'test"test' | shell_escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "'test\"test'");

    let result = tera
        .render_str(
            r#"{{ ["test", "test test", "test'test", 'test"test'] | shell_escape }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, "test 'test test' 'test'\\''test' 'test\"test'");
}

#[test]
fn test_escape_filter() {
    let mut tera = get_tera_instance(HashMap::new());

    let result = tera
        .render_str(r#"{{ "test" | escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "test");

    let result = tera
        .render_str(r#"{{ "test test" | escape }}"#, &tera::Context::new())
        .unwrap();
    assert_eq!(result, "'test test'");

    let result = tera
        .render_str(
            r#"{{ ["test", "test test"] | shell_escape_unix }}"#,
            &tera::Context::new(),
        )
        .unwrap();
    assert_eq!(result, "test 'test test'");
}
