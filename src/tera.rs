#[cfg(test)]
#[path = "tera_test.rs"]
mod tera_test;

use std::{collections::HashMap, io::Write};

use crate::print_utils::MomOutput;
use tera::{Error, Function, Value};

#[cfg(test)]
const USER_INPUT: &str = "something";

fn exclude(val: &Value, params: &HashMap<String, Value>) -> Result<Value, Error> {
    let value_to_exclude = match params.get("val") {
        Some(value) => value,
        None => return Err(Error::msg("val parameter is required")),
    };

    match val {
        Value::Array(array) => {
            let mut result = Vec::new();
            for item in array {
                if item != value_to_exclude {
                    result.push(item.clone());
                }
            }
            Ok(Value::Array(result))
        }
        Value::Object(object) => {
            let mut result = tera::Map::new();
            for (key, value) in object {
                if key != value_to_exclude {
                    result.insert(key.clone(), value.clone());
                }
            }
            Ok(Value::Object(result))
        }
        _ => Err(Error::msg(
            "exclude filter can only be used on arrays and objects",
        )),
    }
}

#[cfg(test)]
fn get_user_input(buffer: &mut String) -> Result<(), Error> {
    buffer.push_str(USER_INPUT);
    Ok(())
}

#[cfg(not(test))]
fn get_user_input(buffer: &mut String) -> Result<(), Error> {
    std::io::stdin().read_line(buffer)?;
    Ok(())
}

/// Prompts the user for input and returns the value as a string.
fn generic_input(args: &HashMap<String, Value>, secret: bool) -> Result<Value, Error> {
    let label = match args.get("label") {
        Some(value) => value,
        None => return Err(Error::msg("label parameter is required")),
    };

    let default = args.get("default");
    if let Some(default) = default {
        if !default.is_string() {
            return Err(Error::msg("default parameter must be a string"));
        }
    }

    let trim = match args.get("trim") {
        Some(Value::Bool(value)) => value,
        Some(_) => return Err(Error::msg("trim parameter must be a boolean")),
        None => &true,
    };

    let condition = match args.get("if") {
        Some(Value::Bool(value)) => value,
        Some(_) => return Err(Error::msg("if parameter must be a boolean")),
        None => &true,
    };

    if !condition {
        match default {
            Some(default) => return Ok(default.clone()),
            None => {
                return Err(Error::msg(
                    "A default value is required with `if` parameter",
                ))
            }
        }
    }

    match label {
        Value::String(label) => {
            let mut input = String::new();

            while input.is_empty() {
                match default {
                    Some(default) => {
                        print!("{}", format!("{} [{}]: ", label, default).mom_just_prefix());
                    }
                    None => print!("{}: ", label),
                }
                // flush stdout so the prompt is shown
                std::io::stdout().flush().unwrap();

                if secret {
                    input = rpassword::read_password().map_err(|e| Error::msg(e.to_string()))?
                } else {
                    get_user_input(&mut input)?;
                }

                if *trim {
                    input = input.trim().to_string();
                }

                if input.is_empty() {
                    if let Some(default) = default {
                        return Ok(default.clone());
                    }
                    println!("Please enter a value");
                }
            }
            Ok(Value::String(input))
        }
        _ => Err(Error::msg("label parameter must be a string")),
    }
}

fn input(args: &HashMap<String, Value>) -> Result<Value, Error> {
    generic_input(args, false)
}

fn password(args: &HashMap<String, Value>) -> Result<Value, Error> {
    generic_input(args, true)
}

/// Returns a function that can be used to get environment variables
/// from the task, or system environment variables if the task does
/// not have the variable.
///
/// # Arguments
///
/// * `env`: HashMap of environment variables
///
/// returns: Function
fn make_get_env(env: HashMap<String, String>) -> impl Function {
    Box::new(
        move |args: &HashMap<String, Value>| -> tera::Result<Value> {
            let name_arg = match args.get("name") {
                Some(Value::String(value)) => value,
                Some(_) => return Err(Error::msg("name parameter must be a string")),
                None => return Err(Error::msg("name parameter is required")),
            };

            let default = args.get("default");

            match env.get(name_arg) {
                Some(value) => Ok(Value::String(value.clone())),
                None => match std::env::var(name_arg) {
                    Ok(value) => Ok(Value::String(value)),
                    Err(_) => {
                        if let Some(default) = default {
                            Ok(default.clone())
                        } else {
                            Err(Error::msg(format!(
                                "Environment variable `{}` not found",
                                name_arg
                            )))
                        }
                    }
                },
            }
        },
    )
}

/// Returns a Tera instance with all the filters registered
/// and ready to be used.
pub(crate) fn get_tera_instance(env: HashMap<String, String>) -> tera::Tera {
    let mut tera = tera::Tera::default();
    tera.register_filter("exclude", exclude);
    tera.register_function("input", input);
    tera.register_function("password", password);
    tera.register_function("get_env", make_get_env(env));
    tera
}
