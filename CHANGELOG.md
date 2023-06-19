# Changelog

## Unreleased

### Added
- Added `vars_file` option to load json, yaml, toml or .env files as variables.
- Added `escape` (replacing default `escape` filter), `shell_escape`, `shell_escape_unix`, `shell_escape_windows`
and `escape_html` tera filters.

### Changed
- The dotenv option can take a list of file paths or objects, where the object must have a path
and optionally the `required`,`overwrite` or `into` properties.
- Disabled auto-escaping in tera templates.
- Arguments in `cmds` and `args` are now parsed following unix shell rules.

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
