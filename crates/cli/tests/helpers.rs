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
