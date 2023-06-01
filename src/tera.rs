#[cfg(test)]
#[path = "tera_test.rs"]
mod tera_test;

use std::{collections::HashMap, io::Write};

use crate::print_utils::MomOutput;
use tera::{Error, Value};

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
fn input(args: &HashMap<String, Value>) -> Result<Value, Error> {
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

    match label {
        Value::String(label) => {
            let mut input = String::new();

            while input.is_empty() {
                match default {
                    Some(Value::String(default)) => {
                        print!("{}", format!("{} [{}]: ", label, default).mom_just_prefix());
                    }
                    Some(_) => unreachable!("Should have validated that default is a string"),
                    None => print!("{}: ", label),
                }
                // flush stdout so the prompt is shown
                std::io::stdout().flush().unwrap();
                get_user_input(&mut input)?;
                input = input.trim().to_string();
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

/// Returns a Tera instance with all the filters registered
/// and ready to be used.
pub(crate) fn get_tera_instance() -> tera::Tera {
    let mut tera = tera::Tera::default();
    tera.register_filter("exclude", exclude);
    tera.register_function("input", input);
    tera
}
