use assert_cmd::assert::Assert;
use std::path::Path;

pub fn create_moon_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("moon").unwrap();
    cmd.current_dir(path);
    cmd.timeout(std::time::Duration::from_secs(90));
    cmd.env("RUST_BACKTRACE", "1");
    // Let our code know we're running tests
    cmd.env("MOON_TEST", "true");
    // Hide install output as it disrupts testing snapshots
    cmd.env("MOON_TEST_HIDE_INSTALL_OUTPUT", "true");
    // Standardize file system paths for testing snapshots
    cmd.env("MOON_TEST_STANDARDIZE_PATHS", "true");
    // Enable logging for code coverage
    cmd.env("MOON_LOG", "trace");
    cmd
}

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

pub fn get_assert_output(assert: &Assert) -> String {
    get_assert_stdout_output(assert) + &get_assert_stderr_output(assert)
}

pub fn get_assert_stderr_output(assert: &Assert) -> String {
    let mut output = String::new();
    let stderr = output_to_string(&assert.get_output().stderr);

    // We need to always show logs for proper code coverage,
    // but this breaks snapshots, and as such, we need to manually
    // filter out log lines and env vars!
    for line in stderr.split('\n') {
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

pub fn get_assert_stdout_output(assert: &Assert) -> String {
    output_to_string(&assert.get_output().stdout)
}
