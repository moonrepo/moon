use crate::path;
use std::env;
use std::path::{Path, PathBuf};

pub fn create_fixtures_sandbox(dir: &str) -> assert_fs::fixture::TempDir {
    use assert_fs::prelude::*;

    let temp_dir = assert_fs::fixture::TempDir::new().unwrap();

    temp_dir
        .copy_from(get_fixtures_dir(dir), &["**/*"])
        .unwrap();

    temp_dir
}

pub fn get_fixtures_dir(dir: &str) -> PathBuf {
    get_fixtures_root().join(dir)
}

pub fn get_fixtures_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures");

    path.canonicalize().unwrap()
}

pub fn replace_fixtures_dir(value: &str, dir: &Path) -> String {
    value.replace(dir.to_str().unwrap(), "<WORKSPACE>")
}

// We need to do this so slashes are accurate and always forward
pub fn wrap_glob(path: &Path) -> PathBuf {
    PathBuf::from(path::normalize_glob(path))
}

pub fn create_moon_command(fixture: &str) -> assert_cmd::Command {
    let mut cmd = create_moon_command_in(&get_fixtures_dir(fixture));
    // Never cache in these tests since they're not in a sandbox
    cmd.env("MOON_CACHE", "off");
    cmd
}

pub fn create_moon_command_in(path: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("moon").unwrap();
    cmd.current_dir(path);
    // Let our code know were running tests
    cmd.env("MOON_TEST", "true");
    // Hide install output as it disrupts testing snapshots
    cmd.env("MOON_TEST_HIDE_INSTALL_OUTPUT", "true");
    // Standardize file system paths for testing snapshots
    cmd.env("MOON_TEST_STANDARDIZE_PATHS", "true");
    // Uncomment for debugging
    // cmd.arg("--logLevel");
    // cmd.arg("trace");
    cmd
}

pub fn get_assert_output(assert: &assert_cmd::assert::Assert) -> String {
    get_assert_stdout_output(assert) + &get_assert_stderr_output(assert)
}

pub fn get_assert_stderr_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.to_owned()).unwrap()
}

pub fn get_assert_stdout_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.to_owned()).unwrap()
}
