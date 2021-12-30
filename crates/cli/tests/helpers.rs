use assert_cmd::Command;
use std::env;

pub fn create_test_command(fixture: &str) -> Command {
    let mut path = env::current_dir().unwrap();
    path.push("../../tests/fixtures");
    path.push(fixture);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir(path.canonicalize().unwrap());
    cmd
}

#[allow(dead_code)]
pub fn get_assert_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.to_owned()).unwrap()
}

#[allow(dead_code)]
pub fn get_assert_stderr_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.to_owned()).unwrap()
}
