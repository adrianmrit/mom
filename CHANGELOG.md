# Changelog

## v1.3.0 - 2023-06-14

### Added
- Added a new `password` tera function. Similar to `input`, but does not echo the input to the
terminal.

### Changed
- `input` now supports an `if` parameter, which can be used to skip the prompt if the condition is
not met.

## v1.2.0 - 2023-06-05

### Changed
- Some task parameters support environment variables and tilde expansion
- `get_env` function can now return the environment variables defined in the task or file, which
takes precedence over the system environment variables.
- Make `echo` a builtin command

## v1.1.0 - 2023-06-01

### Added
- Added option to install with homebrew
- Added `condition` option to tasks
- Added `exclude` tera filter
- Added `input` tera function

## v1.0.0 - 2023-05-23

Initial release 🚀
