use crate::fs;
use std::env;
use std::path::{Path, PathBuf};

pub fn get_fixtures_dir(dir: &str) -> PathBuf {
    get_fixtures_root().join(dir)
}

pub fn get_fixtures_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures");

    path.canonicalize().unwrap()
}

// We need to do this so slashes are accurate and always forward
pub fn wrap_glob(path: &Path) -> PathBuf {
    PathBuf::from(fs::normalize_glob(path))
}

pub fn create_moon_command(fixture: &str) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("moon").unwrap();
    cmd.current_dir(get_fixtures_dir(fixture));
    cmd.env("MOON_TEST", "true");
    cmd.env("MOON_CACHE", "off"); // Never cache in tests
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
