use crate::sandbox::{debug_sandbox_files, Sandbox};
use assert_cmd::assert::Assert;
use dirs::home_dir;
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
            && !line.starts_with("  PROTO_")
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

pub struct SandboxAssert<'s> {
    pub inner: Assert,
    pub sandbox: &'s Sandbox,
}

impl<'s> SandboxAssert<'s> {
    pub fn debug(&self) -> &Self {
        println!("sandbox:");
        debug_sandbox_files(self.sandbox.path());
        println!("\n");

        let output = self.inner.get_output();

        println!("stdout:\n{}\n", output_to_string(&output.stdout));
        println!("stderr:\n{}\n", output_to_string(&output.stderr));
        println!("status: {:#?}", output.status);

        self
    }

    pub fn code(self, num: i32) -> Assert {
        self.inner.code(num)
    }

    pub fn failure(self) -> Assert {
        self.inner.failure()
    }

    pub fn success(self) -> Assert {
        self.inner.success()
    }

    pub fn output(&self) -> String {
        let mut output =
            get_assert_stdout_output(&self.inner) + &get_assert_stderr_output(&self.inner);

        // Replace fixture path
        let root = self.sandbox.path().to_str().unwrap();

        output = output.replace(root, "<WORKSPACE>");
        output = output.replace(&root.replace('\\', "/"), "<WORKSPACE>");

        // Replace home dir
        if let Some(home_dir) = home_dir() {
            let home = home_dir.to_str().unwrap();

            output = output.replace(home, "~");
            output = output.replace(&home.replace('\\', "/"), "~");
        }

        output.replace("/private<", "<")
    }

    pub fn output_standardized(&self) -> String {
        self.output().replace('\\', "/")
    }
}
