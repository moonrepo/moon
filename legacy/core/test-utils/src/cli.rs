use crate::sandbox::{Sandbox, debug_sandbox_files};
use assert_cmd::assert::Assert;
use assert_cmd::cargo::cargo_bin;
use starbase_utils::dirs::home_dir;
use std::path::Path;

pub fn create_moon_command_std<T: AsRef<Path>>(path: T) -> std::process::Command {
    let path = path.as_ref();

    let mut cmd = std::process::Command::new(cargo_bin("moon"));
    cmd.current_dir(path);
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("NO_COLOR", "1");
    // Store plugins in the sandbox
    cmd.env(
        "MOON_HOME",
        path.join(".moon-home").to_string_lossy().to_string(),
    );
    // Let our code know we're running tests
    cmd.env("MOON_TEST", "true");
    cmd.env("STARBASE_TEST", "true");
    // Hide install output as it disrupts testing snapshots
    cmd.env("MOON_TEST_HIDE_INSTALL_OUTPUT", "true");
    // Standardize file system paths for testing snapshots
    cmd.env("MOON_TEST_STANDARDIZE_PATHS", "true");
    // Enable logging for code coverage
    cmd.env("MOON_LOG", "trace");
    // Advanced debugging
    // cmd.env("PROTO_LOG", "trace");
    // cmd.env("MOON_DEBUG_WASM", "true");
    cmd
}

pub fn create_moon_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = assert_cmd::Command::from_std(create_moon_command_std(path));
    cmd.timeout(std::time::Duration::from_secs(120));
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
        if !line.starts_with("[ERROR")
            && !line.starts_with("[ WARN")
            && !line.starts_with("[ INFO")
            && !line.starts_with("[DEBUG")
            && !line.starts_with("[TRACE")
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

impl SandboxAssert<'_> {
    pub fn debug(&self) -> &Self {
        println!("SANDBOX:");
        debug_sandbox_files(self.sandbox.path());
        println!("\n");

        let output = self.inner.get_output();

        println!("STDERR:\n{}\n", output_to_string(&output.stderr));
        println!("STDOUT:\n{}\n", output_to_string(&output.stdout));
        println!("STATUS:\n{:#?}", output.status);

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
        let root = self
            .sandbox
            .path()
            .to_str()
            .unwrap()
            .replace("C:\\Users\\ADMINI~1", "C:\\Users\\Administrator")
            .replace("C:/Users/ADMINI~1", "C:/Users/Administrator");

        output = output
            .replace("C:\\Users\\ADMINI~1", "C:\\Users\\Administrator")
            .replace("C:/Users/ADMINI~1", "C:/Users/Administrator")
            .replace(&root, "<WORKSPACE>")
            .replace(&root.replace('\\', "/"), "<WORKSPACE>");

        // Replace home dir
        if let Some(home_dir) = home_dir() {
            let home = home_dir.to_str().unwrap();
            let root_without_home = root.replace(home, "~");

            output = output
                .replace(&root_without_home, "<WORKSPACE>")
                .replace(&root_without_home.replace('\\', "/"), "<WORKSPACE>")
                .replace(home, "~")
                .replace(&home.replace('\\', "/"), "~");
        }

        output.replace("/private<", "<")
    }

    pub fn output_standardized(&self) -> String {
        self.output().replace('\\', "/")
    }
}
