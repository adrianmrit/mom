# mom
![build](https://github.com/adrianmrit/mom/actions/workflows/test.yml/badge.svg)
[![codecov](https://codecov.io/gh/adrianmrit/mom/branch/main/graph/badge.svg?token=3BBJFNNJPT)](https://codecov.io/gh/adrianmrit/mom)
![License: MIT](https://img.shields.io/github/license/adrianmrit/mom)

> Task runner for teams and individuals. Written in [Rust](https://www.rust-lang.org/).

## Index
* [Inspiration](#inspiration)
* [Installation](#installation)
  * [Binary releases](#binary-releases)
* [Quick start](#quick-start)
* [Usage](#usage)
  * [Command line options](#command-line-options)
  * [Task files](#task-files)
  * [Common Properties](#common-properties)
    * [wd](#wd)
    * [env](#env)
    * [dotenv](#dotenv)
    * [incl](#incl)
  * [Tasks File Properties](#tasks-file-properties)
    * [version](#version)
    * [tasks](#tasks)
    * [extend](#file_extend)
  * [Task Properties](#task-properties)
    * [help](#help)
    * [script](#script)
    * [script_runner](#script_runner)
    * [script_extension](#script_extension)
    * [cmds](#cmds)
    * [program](#program)
    * [args](#args)
    * [args_extend](#args_extend)
    * [args+](#args_extend)
    * [linux](#os-specific-tasks)
    * [windows](#os-specific-tasks)
    * [mac](#os-specific-tasks)
    * [private](#private)
    * [extend](#task_extend)
  * [OS specific tasks](#os-specific-tasks)
  * [Passing arguments](#passing-arguments)
* [Contributing](#contributing)


<a name="inspiration"></a>
## Inspiration

Inspired by different tools like [cargo-make](https://github.com/sagiegurari/cargo-make),
[go-task](https://taskfile.dev/)
[doskey](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/doskey),
[bash](https://www.gnu.org/savannah-checkouts/gnu/bash/manual/bash.html)
and
[docker-compose](https://docs.docker.com/compose/).


<a name="installation"></a>
## Installation

If you have [Rust](https://www.rust-lang.org/) and [Cargo](https://doc.rust-lang.org/cargo/) installed ([rust installation instructions](https://www.rust-lang.org/tools/install)). Then run:
```bash
cargo install --force mom
```

Pro-tip: make sure `~/.cargo/bin` directory is in your `PATH` environment variable.


<a name="binary-releases"></a>
### Binary releases:

Binaries are also available for Windows, Linux and macOS under
[releases](https://github.com/adrianmrit/mom/releases/). To install, download the zip for your system, extract,
and copy the binary to the desired location. You will need to ensure the folder that contains the binary is available
in the `PATH`.


<a name="quick-start"></a>
## Quick start

Create a file named `mom.root.yml` in the root of your project.

Here is a very basic example of a task file:
```yaml
# mom.root.yml
version: 1

vars:
  greeting: Hello World

tasks:
  hi:
    cmds:
      - echo {{ vars.greeting }}
  
  hi.windows:
    script: echo {{ vars.greeting }} from Windows
  
  sum:
    cmds:
      - echo "{{ args.0 }} + {{ args.1 }} = {{ args.0 | int + args.1 | int }}"
```

After having a mom file, you can run a task by calling `mom`, the name of the task, and any arguments, i.e.
`mom hi`. Arguments can be passed right after the task name, either by name or position, i.e. `mom sum 1 2`.


<a name="usage"></a>
## Usage

<a name="command-line-options"></a>
### Command line options
You can see some help about the command line options by running `mom -h` or `mom --help`. Essentially, the
usage would be like this:

```
Usage: mom [OPTIONS] [COMMAND]

Options:
  -l, --list              Lists configuration files that can be reached from the current directory
  -t, --list-tasks        Lists tasks
  -i, --task-info <TASK>  Displays information about the given task
      --dry               Runs the task in dry mode, i.e. without executing any commands
  -f, --file <FILE>       Search for tasks in the given file
  -g, --global            Search for tasks in ~/mom/mom.global.{yml,yaml}
  -h, --help              Print help
  -V, --version           Print version
```


<a name="task-files"></a>
### Task files

The tasks are defined using the YAML format.

When invoking a task, starting in the working directory and continuing to the root directory, the program will
look configuration files in a certain order until either a task is found, a `mom.root.{yml,yaml}` file is found,
or there are no more parent folders (reached root directory). The name of these files is case-sensitive in case-sensitive
systems, i.e. `mom.root.yml` will not work in linux.

The priority order is as follows:
- `mom.private.yml`: Should hold private tasks and should not be committed to the repository.
- `mom.private.yaml`: Same as above but for yaml format.
- `mom.yml`: Should be used in sub-folders of a project for tasks specific to that folder and sub-folders.
- `mom.yaml`: Same as above but for yaml format.
- `mom.root.yml`: Should hold tasks for the entire project.
- `mom.root.yaml`: Same as above but for yaml format.

An especial task file can be defined at `~/mom/mom.global.yml` or `~/mom/mom.global.yaml` for global tasks.
To run a global task, you need to pass the `--global` or `-g` flag, i.e. `mom -g say_hi`. This is useful for
personal tasks that are not related to a specific project.

Tasks can also be defined in a different file by passing the `--file` or `-f` flag, i.e. `mom -f my_tasks.yml say_hi`.

While you can add any of the two formats, i.e. `mom.root.yml` and `mom.root.yaml`, it is recommended to use
only one format for consistency and to avoid confusion.


<a name="common-properties"></a>
### Common Properties
The following properties can be defined in the task file or in the task itself. The value defined in the task takes
precedence over the value defined in the file.

- [wd](#wd): The default working directory.
- [env](#env): Environment variables.
- [dotenv](#dotenv): File or list of files containing environment variables.
- [vars](#vars): Variables.
- [incl](#incl): Templates that can be included/imported in the Tera template engine.


<a name="wd"></a>
##### wd

The `wd` property is used to define the default working directory for the tasks in the file. The value of the
property is a string containing the path to the working directory. The path can be absolute or relative to the
location of the file.

If not defined in the file or task, it defaults to the directory where the command was
executed. To set the working directory relative to the location of the file, use `wd: ""`. Note that
`wd: "/"` will not work, as it will be interpreted as an absolute path.

The value defined in the executed task takes precedence over the value defined in the file.


<a name="env"></a>
##### env

The `env` property is used to define environment variables that will be available to all tasks in the file.
The value of the property is a map of key-value pairs, where the key is the name of the environment variable,
and the value is the value of the environment variable.

The value defined in the executed task takes precedence over the value defined in the file.

See also:
- [dotenv](#dotenv)
- [env and vars inheritance](#env-and-vars-inheritance)


<a name="dotenv"></a>
##### dotenv

The `dotenv` property is used to define environment variables that will be available to all tasks in the file.
The value of the property is a string, or list of strings containing the path to the files containing the environment
variables. The path can be absolute or relative to the location of the file.

The value defined in the `env` property take precedence over the value defined using the `dotenv` property.


<a name="vars"></a>
##### vars

The `vars` property is used to define variables that will be available to all tasks in the file.
This behaves like the [env](#env) property, but the variables are not exported to the environment,
and can be more complex than strings.

For example, you can define a variable like this:

```yaml
vars:
  user:
    age: 20
    name: John
```

And then use it in a task like this:

```yaml
tasks:
  say_hi:
    cmd: echo "Hi, {{ vars.user.name }}!"
```


<a name="incl"></a>
##### incl

The `incl` property is used to define Tera includes/templates that will be available to all tasks in the file.
The value of the property is a map of key-value pairs, where the key is the name of the template,
and the value is the template itself. The template can then be accessed in a task with the name `incl.<name>`.

Templates can include other templates, but the order in which they are defined matters.

For example, you can define a template like this:

```yaml
incl:
  say_hi: "Hi from {{ TASK.name }}!"
  say_bye: "Bye from {% include "incl.say_hi" %}!"
```

However, the following will not work:

```yaml
incl:
  say_bye: "Bye from {% include "incl.say_hi" %}!"
  say_hi: "Hi from {{ TASK.name }}!"
```

Templates can also be defined in the task, and they will take precedence over the templates defined in the file.

Templates can be also used to define [macros](https://tera.netlify.app/docs/#macros).
See the also the [include](https://tera.netlify.app/docs/#include) documentation for Tera.


<a name="tasks-file-properties"></a>
### Tasks File Properties

Besides the [common properties](#common-properties), the following properties can be defined in the task file:
- [tasks](#tasks): The tasks defined in the file.
- version: The version of the file. Although not used at the moment, it is required for future compatibility. The version
  can be a number or string. At the moment backward compatibility with version 1 was not implemented. Therefore, at the
  moment of writing this, the version should be `2`.


<a name="tasks"></a>
##### tasks
The `tasks` property is used to define the tasks in the file. The value of the property is a map of key-value
pairs, where the key is the name of the task, and the value is the task definition.

The name of the task can be any string, but it is recommended to use only alphanumeric characters and dashes.
Private tasks should start with an underscore, i.e. `_private-task`.

<a name="file_extend"></a>
##### File extend

In the file, the `extend` property is used to define the mom files to inherit from. It might be a path or a list of paths
relative to the location of the file, or an absolute path. For example:
  
```yaml
version: 1

# Extend from a file in the same directory
extend: mom.base.yml

# Extend from a file in a subdirectory
extend: base/mom.base.yml

# Extend from multiple files
extend:
  - mom.base.yml
  - base/mom.base.yml
```

The inherited values are:
- [wd](#wd)

Values merged (with the file values taking precedence) are:
- [env](#env)
- [vars](#vars)
- [incl](#incl)
- [tasks](#tasks)

[dotenv](#dotenv) is loaded and merged with the [env](#env) in the same file before extending from a file or merging into the parent file.
Which means it is treated as part of the [env](#env)


<a name="task-properties"></a>
### Task Properties

Besides the common properties, the task can have the following properties:
- [help](#help): The help message.
- [script](#script): The script to execute.
- [script_runner](#script_runner): A template to parse the script program and arguments.
- [script_extension](#script_extension): The extension of the script file.
- [script_ext](#script_extension): Alias for `script_extension`.
- [cmds](#cmds): The commands to execute.
- [program](#program): The program to execute.
- [args](#args): The arguments to pass to the program.
- [args_extend](#args_extend): The arguments to pass to the program, appended to the arguments from the base task, if any.
- [args+](#args_extend): Alias for `args_extend`.
- [linux](#os-specific-tasks): A version of the task to execute in linux.
- [windows](#os-specific-tasks): A version of the task to execute in windows.
- [mac](#os-specific-tasks): A version of the task to execute in mac.
- [private](#private): Whether the task is private or not.
- [extend](#extend): Tasks to inherit from.


<a name="help"></a>
##### help

The `help` property is used to define the help message for the task. The value of the property is a string
containing the help message.

Unlike comments, help will be printed when running `mom -i <TASK>`.


<a name="script"></a>
#### Script

**⚠️Warning:**
DO NOT PASS SENSITIVE INFORMATION AS PARAMETERS IN SCRIPTS. Scripts are stored in a file in the temporal
directory of the system and is the job of the OS to delete it, however it is not guaranteed that when or if that would
be the case. So any sensitive argument passed could be persisted indefinitely.

The `script` value inside a task will be executed in the command line (defaults to cmd in Windows
and bash in Unix). Scripts can spawn multiple lines, and contain shell built-ins and programs.

The generated scripts are stored in the temporal directory, and the filename will be a hash so that if the
script was previously called with the same parameters, we can reuse the previous file, essentially working
as a cache.


<a name="script_runner"></a>
##### script_runner

The `script_runner` property is used to define the template to parse the script program and arguments. Must contain
a program and a `{{ script_path }}` template, i.e. `python {{ script_path }}`. Arguments are separated in the same way
as [args](#args).


<a name="script_extension"></a>
##### script_extension

The `script_extension` property is used to define the extension of the script file. I.e. `py` or `.py` for python scripts.

<a name="program"></a>
#### Program

The `program` value inside a task will be executed as a separate process, with the arguments passed
on `args`, if any.

<a name="args"></a>
#### Args

The `args` values inside a task will be passed as arguments to the program, if any. The value is a string
containing the arguments separated by spaces. Values with spaces can be quoted to be treated as one, i.e.
`"hello world"`. Quotes can be escaped with a backslash, i.e. `\"`.

<a name="args_extend"></a>
#### Args Extend
The `args_extend` values will be appended to `args` (with a space in between), if any. The value is a string
in the same form as `args`.

<a name="cmds"></a>
#### Cmds

The `cmds` value is a list of commands to execute. Each command can be either a string, or a map with a `task` key.

If the command is a string, it will be executed as a program, with the first value being the program, and the
rest being the arguments. Arguments are separated in the same way as [args](#args).

If the command is a map, the value of `task` can be either the name of a task to execute, or the definition of a
task to execute.

Example:
```yaml
tasks:
  say_hi:
    script: echo "hi"

  say_bye:
    script: echo "bye"
  
  greet:
    cmds:
      - python -c "print('hello')"
      - task: say_hi
      - task:
          extend: say_bye
```

<a name="private"></a>
#### Private
The `private` value is a boolean that indicates if the task is private or not. Private tasks cannot be executed
directly, but can be inherited from.


<a name="task_extend"></a>
##### Task extend

In the task, the `extend` property is used to define the tasks to inherit from. It might be a string or a list of strings. For example:
  
```yaml
tasks:
  base_task:
    program: echo
    args: "Hello, world!"
  task:
    extend: base_task

  other_task:
    extend:
      - base_task
      - task
```

The tasks are merged, with the parent task taking precedence over the base task.

The inherited values are:
- [wd](#wd)
- [help](#help)
- [script](#script)
- [script_runner](#script_runner)
- [script_extension](#script_extension)
- [script_ext](#script_extension) (alias for `script_extension`)
- [program](#program)
- [args](#args)
- [cmds](#cmds)

Values merged (with the parent values taking precedence) are:
- [env](#env)
- [vars](#vars)
- [incl](#incl)

Just like in the file, [dotenv](#dotenv) is loaded and merged with the [env](#env) in the same task before extending from a task or merging into the parent task. Which means it is treated as part of the [env](#env) in the task.

Values not inherited are:
- [args_extend](#args_extend) (appended to the inherited [args](#args))
- [args+][#args_extend] (alias for `args_extend`)
- [private](#private)
- [windows](#os-specific-tasks)
- [linux](#os-specific-tasks)
- [macos](#os-specific-tasks)


<a name="os-specific-tasks"></a>
### OS specific tasks

You can have a different OS version for each task. If a task for the current OS is not found, it will
fall back to the non os-specific task if it exists. I.e.
```yaml
tasks:
  ls:
    script: "ls {{ args.0 }}"

  ls.windows:
    script: "dir {{ args.0 }}"
```

Os tasks can also be specified in a single key, i.e. the following is equivalent to the example above.

```yaml
tasks:
  ls: 
    script: "ls {{ args.0 }}"

  ls.windows:
    script: "dir {{ args.0 }}"
```

Note that os-specific tasks do not inherit from the non-os specific task implicitly, if you want to do so, you will have
to define [extend](#extend) explicitly, i.e.

```yaml
tasks:
  ls:
    env:
      DIR: "."
    script: "ls {{ env.DIR }}"

  ls.windows:
    extend: ls
    script: "dir {{ env.DIR }}"
```


<a name="passing-arguments"></a>
### Passing arguments

Arguments for tasks can be either passed as a key-value pair, i.e. `--name "John Doe"`, or as a positional argument, i.e.
`"John Doe"`.

Named arguments must start with one or two dashes, followed by an ascii alpha character or underscore, followed by any number
of letters, digits, `-` or `_`. The value will be either the next argument or the value after the equals sign, i.e.
`--name "John Doe"`, `--name-person1="John Doe"`, `-name_person1 John` are all valid. Note that `"--name John"` is not
a named argument because it is surrounded by quotes and contains a space, however `"--name=John"` is valid named argument.

The first versions used a custom parser, but it takes a lot of work to maintain and it is not as powerful.
So now the template engine used is [Tera](https://tera.netlify.app/docs/). The syntax is
based on Jinja2 and Django templates. The syntax is very easy and powerful.

The exported variables are:
- `args`: The arguments passed to the task. If the task is called with `mom say_hi arg1 --name "John"`, then
  `args` will be `["arg1", "--name", "John"]`.
- `kwargs`: The keyword arguments passed to the task. If the task is called with `mom say_hi --name "John"`,
  then `kwargs` will be `{"name": "John"}`. If the same named argument is passed multiple times, the value will be
  the last one.
- `pkwargs`: Same as `kwargs`, but the value is a list of all the values passed for the same named argument.
- `env`: The [environment variables](#env) defined in the task. Note that this does not includes the environment variables
  defined in the system. To access those, use `{{ get_env(name=<value>, default=<default>) }}`.
- `vars`: The [variables](#vars) defined in the task.
- `TASK`: The [task](#task-properties) object and its properties.
- `FILE`: The [file](#tasks-file-properties) object and its properties.

Named arguments are also treated as positional arguments, i.e. if `--name John --surname=Doe` is passed,
`{{ args.0 }}` will be `--name`, `{{ args.1 }}` will be `John`, and `{{ args.2 }}` will be `--surname="Doe"`.
Thus, it is recommended to pass positional arguments first.

In you want to pass all the command line arguments, you can use `{{ args | join(sep=" ") }}`, or `{% for arg in args %} "{{ arg }}" {% %}`
if you want to quote them.

You can check the [Tera documentation](https://tera.netlify.app/docs/#introduction) for more information. Just ignore the Rust specific parts.


<a name="env-and-vars-inheritance"></a>
### Env and vars inheritance

When using the same environment variables ([env](#env)) and variables ([vars](#vars)) values exist in multiple places, the most specific
value will take precedence. For example, values defined using [env](#env) take precedence over values defined using [dotenv](#dotenv),
and [vars](#vars) or [env](#env) defined in a task take precedence over the values defined in the file.

For example, if you have the following file:
```yaml
version: 1

# Default values. The tasks can override these values.
env:
  ENV1: "env1"
  ENV2: "env2"

vars:
  VAR1: "var1"
  VAR2: "var2"

tasks:
  test1:
    env:
      ENV2: "test1_env2"
      ENV3: "test1_env3"
    vars:
      VAR2: "test1_var2"
      VAR3: "test1_var3"
    cmds:
      - echo "{{ env.ENV1 }} {{ env.ENV2 }} {{ env.ENV3 }}"
      - echo "{{ vars.VAR1 }} {{ vars.VAR2 }} {{ vars.VAR3 }}"
      
      # env and vars from the parent will take precedence
      - task: test2
      
      # This subtask will inherit the env and vars from the parent
      # but its own bases, envs and vars will take precedence
      - task:
          # Bases will take precedence over the parent task
          extend: test2

          # env and vars take precedence over the parent task and the bases
          env:
            ENV2: "subtask_env2"
          vars:
            VAR2: "subtask_var2"
  
  test2:
    env:
      ENV2: "test2_env2"
      ENV3: "test2_env3"
    vars:
      VAR2: "test2_var2"
      VAR3: "test2_var3"
    cmds:
      - echo "{{ env.VAR1 }} {{ env.VAR2 }} {{ env.VAR3 }}"
      - echo "{{ vars.VAR1 }} {{ vars.VAR2 }} {{ vars.VAR3 }}"
```

The output will be (excluding debug output):
```console
$ mom test1
env1 test1_env2 test1_env3
var1 test1_var2 test1_var3
env1 test1_env2 test1_env3
var1 test1_var2 test1_var3
env1 subtask_env2 test2_env3
var1 subtask_var2 test2_var3
```

This might be a bit confusing, so let's explain the output:

```console
env1 test1_env2 test1_env3
var1 test1_var2 test1_var3
```
This is the output of the first two commands in the `test1` task. `ENV1` and `VAR1` are only defined in the file, while the
task overrides `ENV2`, `ENV3`, `VAR2` and `VAR3`.

```console
env1 test1_env2 test1_env3
var1 test1_var2 test1_var3
```
This is the output of the third command in the `test1` task, which calls `test2`. Again, `ENV1` and `VAR1` are only defined in the file.
Even though `test2` overrides `ENV2`, `ENV3`, `VAR2` and `VAR3`, the values defined in `test1`, the parent task, take precedence.

```console
env1 subtask_env2 test2_env3
var1 subtask_var2 test2_var3
```
This is the output of the fourth command in the `test1` task, which calls a subtask. `ENV1` and `VAR1` are only defined in the file.
While it might seem like we are calling `task2`, we actually defined a new task that inherits from `task2` and overrides `ENV2` and `VAR2`.
Therefore, the values inherited from `task2` will take precedence over the parent task.


<a name="Contributing"></a>
## Contributing
Contributions welcome! Please read the [contributing guidelines](CONTRIBUTING.md) first.
