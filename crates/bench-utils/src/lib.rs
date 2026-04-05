use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fs;

pub fn create_simple_workspace(max: u16) -> Sandbox {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    for i in 0..=max {
        let dir = sandbox.path().join(format!("p{i}"));

        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("moon.yml"),
            r#"
tasks:
  build:
    command: 'echo build'
"#,
        )
        .unwrap();
    }

    let moon_dir = sandbox.path().join(".moon");

    fs::create_dir_all(&moon_dir).unwrap();
    fs::write(moon_dir.join("workspace.yml"), "projects: ['*']").unwrap();

    fs::create_dir_all(moon_dir.join("tasks")).unwrap();
    fs::write(
        moon_dir.join("tasks/all.yml"),
        r#"
tasks:
  test1:
    command: 'echo 1'
  test2:
    command: 'echo 2'
    deps: ['test1']
  test3:
    command: 'echo 3'
    deps: ['test2']
"#,
    )
    .unwrap();

    sandbox.run_git(|cmd| {
        cmd.args(["add", "--all"]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["commit", "-m", "Initial commit"]);
    });

    sandbox
}
