#[cfg(test)]
#[path = "builtin_commands_test.rs"]
mod builtin_commands_test;

use crate::types::DynErrResult;

/// Represents a built-in command.
type BuiltInCommand = fn(args: &[&str]) -> DynErrResult<()>;

/// Creates an echo built-in command.
fn echo_command(args: &[&str]) -> DynErrResult<()> {
    println!("{}", args.join(" "));
    Ok(())
}

/// Returns a built-in command.
pub(crate) fn get_builtin_command(name: &str) -> Option<BuiltInCommand> {
    match name {
        "echo" => Some(echo_command),
        _ => None,
    }
}
