use assert_cmd::Command;
use insta::assert_snapshot;
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
pub fn snap(assert: &assert_cmd::assert::Assert) {
    assert_snapshot!(String::from_utf8(assert.get_output().stdout.to_owned()).unwrap());
}
