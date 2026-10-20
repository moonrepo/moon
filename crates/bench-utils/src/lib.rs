mod generator;

pub use generator::*;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fs;
use std::path::PathBuf;

pub fn handle_unwrap<T>(res: Result<T, miette::Report>) {
    if let Err(error) = res {
        dbg!(&error);
        panic!("{error}");
    }
}

/// Find a test plugin built with `just build-wasm`, which builds into
/// `CARGO_TARGET_DIR` when it's set, otherwise into `wasm/target`. Benchmarks
/// are release builds, which don't resolve the test plugins' locators, so
/// they're located here.
pub fn find_test_plugin(name: &str) -> Option<PathBuf> {
    let file = format!("{name}.wasm");

    ["CARGO_TARGET_DIR", "WARPGATE_PLUGINS_DIR"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .flat_map(|dir| [dir.clone(), dir.join("target")])
        .flat_map(|dir| {
            ["release", "debug"].map(|profile| dir.join("wasm32-wasip1").join(profile).join(&file))
        })
        .find(|path| path.exists())
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

/// Create a workspace like [`create_simple_workspace`], but where every task
/// runs in the `tc-tier3` test toolchain, which supports setting up the
/// toolchain, and locating a dependencies root. Every task resolves its
/// toolchain's install slot, which the `system` toolchain skips. The test
/// plugin locates the root at the workspace root, without members, so no
/// project is in it, and every task falls back to setting up the toolchain.
/// Requires the test plugins (`just build-wasm`), and the toolchain to be
/// registered and configured by the caller.
pub fn create_toolchain_workspace(max: u16) -> Sandbox {
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
    toolchains: ['tc-tier3']
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
    toolchains: ['tc-tier3']
  test2:
    command: 'echo 2'
    deps: ['test1']
    toolchains: ['tc-tier3']
  test3:
    command: 'echo 3'
    deps: ['test2']
    toolchains: ['tc-tier3']
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

/// Create a workspace like [`create_chained_workspace`], but where every
/// `build` task runs its dependencies serially: its own `test` task first,
/// then the `build` task of its dependency project. Every serial link orders
/// the whole chain of `build` tasks below it after a `test` task.
pub fn create_serial_workspace(max: u16) -> Sandbox {
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
  test:
    command: 'echo test'
  build:
    command: 'echo build'
    deps: ['test']
    options:
      runDepsInParallel: false
"#
                .to_owned()
            } else {
                format!(
                    r#"
dependsOn: ['p{}']

tasks:
  test:
    command: 'echo test'
  build:
    command: 'echo build'
    deps: ['test', '^:build']
    options:
      runDepsInParallel: false
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

/// Create a workspace where every project uses the `tc-tier2` test toolchain,
/// for locating dependencies roots, with `testLocateRoots` enabled by the
/// caller. Projects in `packages/*` are members of a dependencies workspace at
/// the root, while every tenth project is in `apps/*`, with a lockfile of its
/// own, so it's a package root. Requires the test plugins (`just build-wasm`),
/// and the toolchain to be registered and configured by the caller.
pub fn create_layout_workspace(max: u16) -> Sandbox {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    for i in 0..=max {
        let dir = if i % 10 == 0 {
            let dir = sandbox.path().join(format!("apps/a{i}"));

            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("tc.lock"), "").unwrap();
            dir
        } else {
            let dir = sandbox.path().join(format!("packages/p{i}"));

            fs::create_dir_all(&dir).unwrap();
            dir
        };

        fs::write(
            dir.join("moon.yml"),
            r#"
toolchains:
  default: 'tc-tier2'

tasks:
  build:
    command: 'echo build'
"#,
        )
        .unwrap();
    }

    fs::write(sandbox.path().join("tc.lock"), "").unwrap();
    fs::write(sandbox.path().join("tc.root.cfg"), "packages/*").unwrap();

    let moon_dir = sandbox.path().join(".moon");

    fs::create_dir_all(&moon_dir).unwrap();
    fs::write(
        moon_dir.join("workspace.yml"),
        "projects: ['apps/*', 'packages/*']",
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
