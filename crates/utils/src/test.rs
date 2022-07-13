use crate::glob;
use crate::path;
use crate::process::output_to_string;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_git_command<P, T, F>(dir: P, msg: T, handler: F)
where
    P: AsRef<Path>,
    T: AsRef<str>,
    F: FnOnce(&mut Command),
{
    let mut cmd = Command::new(if cfg!(windows) { "git.exe" } else { "git" });
    cmd.current_dir(dir.as_ref());

    handler(&mut cmd);

    let out = cmd.output().unwrap_or_else(|e| {
        println!("{:#?}", e);
        panic!("{}: {:#?}", msg.as_ref(), dir.as_ref());
    });

    if !out.status.success() {
        eprintln!("{}", output_to_string(&out.stdout));
        eprintln!("{}", output_to_string(&out.stderr));
    }
}

pub fn create_fixtures_skeleton_sandbox<T: AsRef<str>>(dir: T) -> assert_fs::fixture::TempDir {
    use assert_fs::prelude::*;

    let temp_dir = assert_fs::fixture::TempDir::new().unwrap();

    temp_dir
        .copy_from(get_fixtures_dir(dir), &["**/*"])
        .unwrap();

    temp_dir
}

pub fn create_fixtures_sandbox<T: AsRef<str>>(dir: T) -> assert_fs::fixture::TempDir {
    let temp_dir = create_fixtures_skeleton_sandbox(dir);

    // Initialize a git repo so that VCS commands work
    run_git_command(
        temp_dir.path(),
        "Failed to initialize git for fixtures sandbox",
        |cmd| {
            cmd.args(["init", "--initial-branch", "master"]);
        },
    );

    // We must also add the files to the index
    run_git_command(
        temp_dir.path(),
        "Failed to add files to git index for fixtures sandbox",
        |cmd| {
            cmd.args(["add", "--all", "."]);
        },
    );

    // And commit them... this seems like a lot of overhead?
    run_git_command(
        temp_dir.path(),
        "Failed to commit files for fixtures sandbox",
        |cmd| {
            cmd.args(["commit", "-m", "'Fixtures'"])
                .env("GIT_AUTHOR_NAME", "moon tests")
                .env("GIT_AUTHOR_EMAIL", "fakeemail@moonrepo.dev")
                .env("GIT_COMMITTER_NAME", "moon tests")
                .env("GIT_COMMITTER_EMAIL", "fakeemail@moonrepo.dev");
        },
    );

    temp_dir
}

pub fn get_fixtures_dir<T: AsRef<str>>(dir: T) -> PathBuf {
    get_fixtures_root().join(dir.as_ref())
}

pub fn get_fixtures_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures");

    path.canonicalize().unwrap()
}

pub fn replace_fixtures_dir<T: AsRef<str>, P: AsRef<Path>>(value: T, dir: P) -> String {
    let dir_str = dir.as_ref().to_str().unwrap();

    // Replace both forward and backward slashes
    value
        .as_ref()
        .replace(dir_str, "<WORKSPACE>")
        .replace(&path::standardize_separators(dir_str), "<WORKSPACE>")
}

// We need to do this so slashes are accurate and always forward
pub fn wrap_glob<T: AsRef<Path>>(path: T) -> PathBuf {
    PathBuf::from(glob::normalize(path).unwrap())
}

pub fn create_moon_command<T: AsRef<str>>(fixture: T) -> assert_cmd::Command {
    let mut cmd = create_moon_command_in(get_fixtures_dir(fixture));
    // Never cache in these tests since they're not in a sandbox
    cmd.env("MOON_CACHE", "off");
    cmd
}

pub fn create_moon_command_in<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("moon").unwrap();
    cmd.current_dir(path);
    // Let our code know were running tests
    cmd.env("MOON_TEST", "true");
    // Hide install output as it disrupts testing snapshots
    cmd.env("MOON_TEST_HIDE_INSTALL_OUTPUT", "true");
    // Standardize file system paths for testing snapshots
    cmd.env("MOON_TEST_STANDARDIZE_PATHS", "true");
    // Enable logging for code coverage
    cmd.env("MOON_LOG", "trace");
    cmd
}

pub fn get_assert_output(assert: &assert_cmd::assert::Assert) -> String {
    get_assert_stdout_output(assert) + &get_assert_stderr_output_clean(assert)
}

pub fn get_assert_stderr_output_clean(assert: &assert_cmd::assert::Assert) -> String {
    let mut output = String::new();

    // We need to always show logs for proper code coverage,
    // but this breaks snapshots, and as such, we need to manually
    // filter out log lines and env vars!
    for line in get_assert_stderr_output(assert).split('\n') {
        if !line.starts_with("[error")
            && !line.starts_with("[ warn")
            && !line.starts_with("[ info")
            && !line.starts_with("[debug")
            && !line.starts_with("[trace")
            && !line.starts_with("  MOON_")
            && !line.starts_with("  NODE_")
        {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}

pub fn get_assert_stderr_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.to_owned()).unwrap()
}

pub fn get_assert_stdout_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.to_owned()).unwrap()
}

pub fn debug_sandbox_files(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            debug_sandbox_files(&path);
        } else {
            println!("- {}", path.to_string_lossy());
        }
    }
}

pub fn debug_sandbox(fixture: &assert_fs::fixture::TempDir, assert: &assert_cmd::assert::Assert) {
    // List all files in the sandbox
    println!("sandbox:");
    debug_sandbox_files(fixture.path());
    println!("\n");

    // Debug outputs
    println!("stdout:\n{}\n", get_assert_stdout_output(assert));
    println!("stderr:\n{}\n", get_assert_stderr_output(assert));
    println!("status: {:#?}", assert.get_output().status);
}
