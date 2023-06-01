use std::collections::HashMap;

use super::USER_INPUT;

use super::get_tera_instance;

#[test]
fn test_exclude_filter() {
    let mut tera = get_tera_instance();

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
    let mut tera = get_tera_instance();

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
