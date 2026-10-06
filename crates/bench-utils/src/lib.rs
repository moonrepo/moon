use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fs;

pub fn handle_unwrap<T>(res: Result<T, miette::Report>) {
    if let Err(error) = res {
        dbg!(&error);
        panic!("{error}");
    }
}

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

/// Create a workspace where every project depends on the previous one,
/// forming a single chain (`p0 <- p1 <- ... <- pN`), with a `build`
/// task that depends on the `build` task of its dependency project.
/// Useful for measuring relationship traversal, as every project and
/// task has a deep upstream and downstream.
pub fn create_chained_workspace(max: u16) -> Sandbox {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    for i in 0..=max {
        let dir = sandbox.path().join(format!("p{i}"));

        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("moon.yml"),
            if i == 0 {
                r#"
tasks:
  build:
    command: 'echo build'
"#
                .to_owned()
            } else {
                format!(
                    r#"
dependsOn: ['p{}']

tasks:
  build:
    command: 'echo build'
    deps: ['^:build']
"#,
                    i - 1
                )
            },
        )
        .unwrap();
    }

    let moon_dir = sandbox.path().join(".moon");

    fs::create_dir_all(&moon_dir).unwrap();
    fs::write(moon_dir.join("workspace.yml"), "projects: ['*']").unwrap();

    sandbox.run_git(|cmd| {
        cmd.args(["add", "--all"]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["commit", "-m", "Initial commit"]);
    });

    sandbox
}
