// At the moment assert_cmd::Command only works in tests outside of the src directory.

use std::{fs::File, io::Write};

use assert_cmd::Command;
use assert_fs::TempDir;
use mom_task::print_utils::MomOutput;
use mom_task::tasks::DRY_RUN_MESSAGE;
use predicates::prelude::{predicate, PredicateBooleanExt};

#[test]
fn test_no_mom_file_discovered() {
    let tmp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("echo");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("[mom] Task echo not found"));
}

#[test]
fn test_run_simple_task() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
    version: 1
    tasks:
        hello:
            script: echo "hello world"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello world"));

    Ok(())
}

#[test]
fn test_args() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
    version: 1
    tasks:
        hello:
            script: echo {{ args.0 }} {{ args.1 }} {{ args }}
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello");
    cmd.arg("arg1");
    cmd.arg("arg2");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("echo arg1 arg2 [arg1, arg2]"));

    Ok(())
}

#[test]
fn test_kwargs() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
    version: 1
    tasks:
        hello:
            script: echo {{ kwargs.k1 }} {{ kwargs.k2 }}
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello");
    cmd.arg("--k1=arg1");
    cmd.arg("--k1=arg2");
    cmd.arg("--k2");
    cmd.arg("arg3");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("echo arg2 arg3"));

    Ok(())
}

#[test]
fn test_pkwargs() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
    version: 1

    tasks:
        hello:
            script: echo {{ pkwargs.k1.0 }} {{ pkwargs.k1.1 }} {{ pkwargs.k2.0 }}
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello");
    cmd.arg("--k1=arg1");
    cmd.arg("--k1=arg2");
    cmd.arg("--k2");
    cmd.arg("arg3");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("echo arg1 arg2 arg3"));

    Ok(())
}

#[test]
fn test_file_option() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("sample.mom.yml"))?;
    file.write_all(
        r#"
    version: 1

    tasks:
        hello:
            script: "ls"

        hello.windows:
            script: "dir"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.args(["-f=sample.mom.yml", "hello"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("sample.mom.yml"));
    drop(file);
    drop(tmp_dir);
    Ok(())
}

#[test]
fn test_run_os_task() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
    version: 1

    tasks:
        hello.windows:
            script: echo hello windows

        hello.linux:
            script: echo hello linux

        hello.macos:
            script: echo hello macos

        hello_again:
            script: echo hello windows

        hello_again.linux:
            script: echo hello linux

        hello_again.macos:
            script: echo hello macos
    "#
        .as_bytes(),
    )?;

    let expected = if cfg!(target_os = "windows") {
        "hello windows"
    } else if cfg!(target_os = "linux") {
        "hello linux"
    } else {
        "hello macos"
    };

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected));

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("hello_again");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected));
    Ok(())
}

#[test]
fn test_set_env() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1

env:
    greeting: "hello world"
    one_plus_one: "two"

tasks:
    hello.windows:
        script: echo %greeting%, one plus one is %one_plus_one%
        env:
            greeting: "hi world"

    hello:
        script: "echo $greeting, one plus one is $one_plus_one"
        env:
            greeting: "hi world"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hi world, one plus one is two"));
    Ok(())
}

#[test]
fn test_env_file() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut env_file = File::create(tmp_dir.join(".env"))?;
    env_file
        .write_all(
            r#"
    VAR1=VAL1
    VAR2=VAL2
    VAR3=VAL3
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut env_file_2 = File::create(tmp_dir.join(".env_2"))?;
    env_file_2
        .write_all(
            r#"
    VAR1=OTHER_VAL1
    VAR2=OTHER_VAL2
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1
dotenv: ".env"

tasks:
    test:
        cmds:
        - "echo $VAR1 $VAR2 $VAR3"

    test_2:
        cmds:
            - echo $VAR1 $VAR2 $VAR3 ${INTO1.VAR1} ${INTO2.VAR2}
        dotenv:
            - path: ".env_2"
              required: false
              overwrite: true
            - path: ".env"
              into: "INTO1."
              overwrite: true
            - path: ".env_2"
              into: "INTO2."
            - path: ".non_existent_env_file"
              required: false
        env:
            VAR1: "TASK_VAL1"
            INTO1.VAR1: "TASK_VAL1"
            INTO2.VAR2: "TASK_VAL2"
            "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("VAL1 VAL2 VAL3"));

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test_2");
    cmd.assert().success().stdout(predicate::str::contains(
        "OTHER_VAL1 OTHER_VAL2 VAL3 VAL1 TASK_VAL2",
    ));

    Ok(())
}

#[test]
fn test_vars_file() {
    let tmp_dir = TempDir::new().unwrap();
    let mut vars_file = File::create(tmp_dir.join("vars.yml")).unwrap();
    vars_file
        .write_all(
            r#"
    VAR1: YAML_OTHER_VAL1
    "#
            .as_bytes(),
        )
        .unwrap();

    let other_vars_file = tmp_dir.join("other_vars.json");
    let mut vars_file = File::create(&other_vars_file).unwrap();
    vars_file
        .write_all(
            r#"
    {
        "VAR2": "JSON_OTHER_VAL2"
    }
    "#
            .as_bytes(),
        )
        .unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml")).unwrap();
    file.write_all(
        r#"
version: 1

vars:
    VAR1: VAL1
    VAR2: VAL2
    VAR3: VAL3
    
tasks:
    test:
        cmds:
            - echo {{ vars.VAR1 }} {{ vars.VAR2 }} {{ vars.VAR3 }}
        vars_file: "vars.yml"
    
    test2:
        cmds:
            - echo {{ vars.VAR1 }} {{ vars.VAR2 }} {{ vars.VAR3 }}
        vars_file: 
            - "vars.yml"
            - path: "other_vars.json"
              overwrite: true
            - path: "non_existent_vars_file.json"
              required: false
    test3:
        vars:
            inner1:
                VAR1: OTHER_VAL1
                VAR1_1: OTHER_VAL1.1
            inner2:
                VAR2: VAL2
        vars_file: 
            - path: "vars.yml"
              overwrite: true
              into: "inner1"
            - path: "other_vars.json"
              overwrite: false
              into: "inner2"
            - path: "other_vars.json"
              into: "inner3"
            - path: "non_existent_vars_file.json"
              required: false
        cmds:
            - echo {{ vars.inner1.VAR1 }} {{ vars.inner1.VAR1_1 }} {{ vars.inner2.VAR2 }} {{ vars.inner3.VAR2 }}
    "#
        .as_bytes(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("VAL1 VAL2 VAL3"));

    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test2");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("JSON_OTHER_VAL2 VAL3"));

    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test3");
    cmd.assert().success().stdout(predicate::str::contains(
        "YAML_OTHER_VAL1 OTHER_VAL1.1 VAL2 JSON_OTHER_VAL2",
    ));
}

#[test]
fn test_run_program() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let (program, param, batch_file_name, batch_file_content) = if cfg!(target_os = "windows") {
        ("cmd", "/C", "echo_args.cmd", "echo %1 %2 %*".as_bytes())
    } else {
        ("bash", "", "echo_args.sh", "echo $1 $2 $*".as_bytes())
    };
    let mut batch_file = File::create(tmp_dir.join(batch_file_name))?;
    batch_file.write_all(batch_file_content).unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        format!(
            r#"
    version: 1

    tasks:
        hello:
            program: {}
            args: {} {} hello world
            "#,
            program, param, batch_file_name
        )
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello world hello world"));

    Ok(())
}

#[test]
fn test_run_cmds() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;

    file.write_all(
        r#"
    version: 1

    env:
        greeting: "hello world"

    tasks:
        task_1:
            cmds:
                - some command
                - some other command
        
        task_2:
            script: "some script"
        
        task_3:
            program: program
            args: "{{ env.GREETING }}"
            env:
                GREETING: "hi world"

        testing:
            cmds:
                - some command
                - cmd: some other command
                - task: task_1
                - task: task_3
                - task:
                    extend: task_3
                    env:
                        GREETING: "hello"
            "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("testing");
    cmd.arg("hi");
    cmd.arg("--name=world");
    cmd.assert().success().stdout(predicate::str::contains(
        format!(
            r#"testing.cmds.0: some command
{DRY_RUN_MESSAGE}
testing.cmds.1: some other command
{DRY_RUN_MESSAGE}
testing.cmds.2.task_1.cmds.0: some command
{DRY_RUN_MESSAGE}
testing.cmds.2.task_1.cmds.1: some other command
{DRY_RUN_MESSAGE}
testing.cmds.3.task_3: program hi world
{DRY_RUN_MESSAGE}
testing.cmds.4: program hello
{DRY_RUN_MESSAGE}"#,
        )
        .mom_just_prefix(),
    ));
    Ok(())
}

#[test]
fn test_env_inheritance() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1

env:
    VAR1: VAL1
    VAR2: VAL2
    VAR3: VAL3

tasks:
    test1:
        env:
            VAR1: VAL1.2

    test2:
        env:
            VAR2: VAL2.2

    test3:
        extend: [test1, test2]
        env:
            # Env should inherit from test1 and test2 and take precedence over the child env
            VAR1: VAL1.3
        cmds:
            - "echo 'VAR1: {{ env.VAR1 }}'"
            - "echo 'VAR2: {{ env.VAR2 }}'"
            - "echo 'VAR3: {{ env.VAR3 }}'"
            - task: test4
            - task:
                # Because we used bases, the env should be inherited from test4, not test3
                extend: test4
                env:
                    VAR4: VAL4.3

            

    test4:
        env:
            VAR1: VAL1.4
            VAR2: VAL2.4
            VAR3: VAL3.4
            VAR4: VAL4
        cmds:
            - "echo 'VAR1: {{ env.VAR1 }}'"
            - "echo 'VAR2: {{ env.VAR2 }}'"
            - "echo 'VAR3: {{ env.VAR3 }}'"
            - "echo 'VAR4: {{ env.VAR4 }}'"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("test3");
    cmd.assert().success().stdout(predicate::str::contains(
        format!(
            r#"test3.cmds.0: echo 'VAR1: VAL1.3'
{DRY_RUN_MESSAGE}
test3.cmds.1: echo 'VAR2: VAL2.2'
{DRY_RUN_MESSAGE}
test3.cmds.2: echo 'VAR3: VAL3'
{DRY_RUN_MESSAGE}
test3.cmds.3.test4.cmds.0: echo 'VAR1: VAL1.3'
{DRY_RUN_MESSAGE}
test3.cmds.3.test4.cmds.1: echo 'VAR2: VAL2.2'
{DRY_RUN_MESSAGE}
test3.cmds.3.test4.cmds.2: echo 'VAR3: VAL3.4'
{DRY_RUN_MESSAGE}
test3.cmds.3.test4.cmds.3: echo 'VAR4: VAL4'
{DRY_RUN_MESSAGE}
test3.cmds.4.cmds.0: echo 'VAR1: VAL1.4'
{DRY_RUN_MESSAGE}
test3.cmds.4.cmds.1: echo 'VAR2: VAL2.4'
{DRY_RUN_MESSAGE}
test3.cmds.4.cmds.2: echo 'VAR3: VAL3.4'
{DRY_RUN_MESSAGE}
test3.cmds.4.cmds.3: echo 'VAR4: VAL4.3'
{DRY_RUN_MESSAGE}
"#
        )
        .mom_just_prefix(),
    ));
    Ok(())
}

#[test]
fn test_extend_args() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1
tasks:
    echo-program:
        program: "echo"
        args: "hello"
        private: true

    hello:
        extend: echo-program
        args_extend: "world"

    hello-and-bye:
        extend: hello
        args+: "and bye world"

    other:
        linux:
            cmds:
                - echo hello linux
        windows:
            cmds:
                - echo hello windows
        macos:
            cmds:
                - echo hello macos
    
    hello-os:
        extend: other
"#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello-and-bye");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("echo hello world and bye world"));

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello-os");

    let expected = if cfg!(target_os = "windows") {
        "hello windows"
    } else if cfg!(target_os = "macos") {
        "hello macos"
    } else {
        "hello linux"
    };

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected));

    Ok(())
}

#[test]
fn test_specify_script_runner() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1
tasks:
    hello:
        script_runner: "python -m {{ script_path }}"
        script_ext: ".py"
        script: "print('hello world')"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("hello");
    cmd.assert()
        .success()
        .stdout(
            predicate::str::contains("[mom] hello: python -m").and(predicate::str::contains(
                "[mom] Script Begin:\nprint('hello world')\n[mom] Script End.\n[mom] Dry run mode, nothing executed.",
            )),
        );
    Ok(())
}

#[test]
fn test_vars() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1

vars:
    var1: val1
    var2: val2
    var3: [1, 2, 3]
    user:
        name: "user1"
        age: 18

tasks:
    test:
        vars:
            # override var2 from root.vars, applies to children
            var2: val2.1
            # override var3 from root.vars, applies to children
            var3: [4, 5, 6]
        cmds:
            - "echo 'var1: {{ vars.var1 }}'"
            - "echo \"var2: {{ vars.var2 }}\""
            - "echo var3: {{ vars.var3[0] }} {{ vars.var3[1] }} {{ vars.var3[2] }}"
            - "echo user: {{ vars.user.name }} {{ vars.user.age }}"
            - task:
                vars:
                    # override var1 from test.vars
                    var1: val1.1
                cmds:
                    - "echo var1: {{ vars.var1 }}"
                    - "echo var2: {{ vars.var2 }}"
                    - "echo var3: {{ vars.var3[0] }} {{ vars.var3[1] }} {{ vars.var3[2] }}"
                    - "echo user: {{ vars.user.name }} {{ vars.user.age }}"
            # Because the parent task takes precedence, should override var2 from other-test.vars
            - task: other-test
    
    other-test:
        vars:
            var2: val2.2
        cmds:
            - "echo var2: {{ vars.var2 }}"
    "#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("test");
    cmd.assert().success().stdout(predicate::str::contains(
        format!(
            r#"test.cmds.0: echo 'var1: val1'
{DRY_RUN_MESSAGE}
test.cmds.1: echo 'var2: val2.1'
{DRY_RUN_MESSAGE}
test.cmds.2: echo 'var3:' 4 5 6
{DRY_RUN_MESSAGE}
test.cmds.3: echo 'user:' user1 18
{DRY_RUN_MESSAGE}
test.cmds.4.cmds.0: echo 'var1:' val1.1
{DRY_RUN_MESSAGE}
test.cmds.4.cmds.1: echo 'var2:' val2.1
{DRY_RUN_MESSAGE}
test.cmds.4.cmds.2: echo 'var3:' 4 5 6
{DRY_RUN_MESSAGE}
test.cmds.4.cmds.3: echo 'user:' user1 18
{DRY_RUN_MESSAGE}
test.cmds.5.other-test.cmds.0: echo 'var2:' val2.1
{DRY_RUN_MESSAGE}
"#
        )
        .mom_just_prefix(),
    ));

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("other-test");
    cmd.assert().success().stdout(predicate::str::contains(
        "other-test.cmds.0: echo 'var2:' val2.2",
    ));
    Ok(())
}

#[test]
fn test_templates() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();
    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1

incl: 
    echo: >
        {% macro echo(msg, type="text") %}echo {{ msg }}{% endmacro %}
    echo2: >
        echo {{ args[0] }}

tasks:
    test:
        incl:
            echo2: >
                echo {{ args.0 | upper }}
            bye: >
                and bye
        cmds:
            - |
                {% import "incl.echo" as macros %}{{ macros::echo(msg="hello") }} world
            - |
                {% include "incl.echo2" %} {% include "incl.bye" %}
"#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("test");
    cmd.arg("hello");
    cmd.assert().success().stdout(predicate::str::contains(
        format!(
            r#"test.cmds.0: echo hello world
{DRY_RUN_MESSAGE}
test.cmds.1: echo HELLO and bye
{DRY_RUN_MESSAGE}
"#
        )
        .mom_just_prefix(),
    ));
    Ok(())
}

#[test]
fn test_task_condition() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = TempDir::new().unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml"))?;
    file.write_all(
        r#"
version: 1

tasks:
    task1:
        condition: "{{ args.0 == \"t1\" }}"
        cmds:
            - echo "task1 executed"
    
    task2:
        condition: "{{ args.0 == \"t2\" }}"
        cmds: 
            - echo "task2 executed"
    
    run:
        cmds:
            - task: task1
            - task: task2
"#
        .as_bytes(),
    )?;

    let mut cmd = Command::cargo_bin("mom")?;
    cmd.current_dir(tmp_dir.path());
    cmd.arg("--dry");
    cmd.arg("run");
    cmd.arg("t2");
    cmd.assert().success().stdout(predicate::str::contains(
        format!(
            r#"run.cmds.0.task1 skipped
run.cmds.1.task2.cmds.0: echo 'task2 executed'
{DRY_RUN_MESSAGE}
"#
        )
        .mom_just_prefix(),
    ));
    Ok(())
}

#[test]
fn test_echo_builtin() {
    let tmp_dir = TempDir::new().unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml")).unwrap();
    file.write_all(
        r#"
version: 1

tasks:
    test:
        cmds:
            - echo "hello world"

"#
        .as_bytes(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            r#"test.cmds.0: echo 'hello world'
hello world
"#
        )));
}

#[test]
fn test_cmd_args_parse_error() {
    let tmp_dir = TempDir::new().unwrap();

    let mut file = File::create(tmp_dir.join("mom.root.yml")).unwrap();
    file.write_all(
        r#"
version: 1

tasks:
    test:
        cmds:
            - echo "hello world
"#
        .as_bytes(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mom").unwrap();
    cmd.current_dir(tmp_dir.path());
    cmd.arg("test");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("missing closing quote"));
}
