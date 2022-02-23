#[cfg(test)]
pub fn create_test_command(fixture: &str) -> assert_cmd::Command {
    let mut path = std::env::current_dir().unwrap();
    path.push("../../tests/fixtures");
    path.push(fixture);

    let mut cmd = assert_cmd::Command::cargo_bin("moon").unwrap();
    cmd.current_dir(path.canonicalize().unwrap());
    cmd.env("MOON_TEST", "true");
    cmd
}

#[cfg(test)]
pub fn get_assert_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.to_owned()).unwrap()
}

#[cfg(test)]
pub fn get_assert_stderr_output(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.to_owned()).unwrap()
}
