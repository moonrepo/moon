#![allow(clippy::disallowed_types)]

pub use starbase_sandbox::{Sandbox, SandboxAssert, SandboxSettings, create_temp_dir};
use std::collections::HashMap;
use std::ops::Deref;

pub struct MoonSandbox {
    pub sandbox: Sandbox,
}

impl MoonSandbox {
    pub fn new(mut sandbox: Sandbox) -> Self {
        apply_settings(&mut sandbox);
        create_workspace_files(&sandbox);

        Self { sandbox }
    }

    pub fn capture_plugin_logs(&self) {
        extism::set_log_callback(
            |line| {
                println!("{line}");
            },
            "debug",
        )
        .unwrap();
    }
}

impl Deref for MoonSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

fn apply_settings(sandbox: &mut Sandbox) {
    let moon_dir = sandbox.path().join(".moon");

    let mut env = HashMap::new();
    env.insert("RUST_BACKTRACE", "1");
    env.insert("WASMTIME_BACKTRACE_DETAILS", "1");
    env.insert("NO_COLOR", "1");
    env.insert("COLUMNS", "150");
    // Store plugins in the sandbox
    env.insert("MOON_HOME", moon_dir.to_str().unwrap());
    // env.insert("PROTO_HOME", path.join(".proto"));
    // Let our code know we're running tests
    env.insert("MOON_TEST", "true");
    env.insert("STARBASE_TEST", "true");
    // Hide install output as it disrupts testing snapshots
    env.insert("MOON_TEST_HIDE_INSTALL_OUTPUT", "true");
    // Standardize file system paths for testing snapshots
    env.insert("MOON_TEST_STANDARDIZE_PATHS", "true");
    // Enable logging for code coverage
    env.insert("MOON_LOG", "trace");
    // Advanced debugging
    // env.insert("PROTO_LOG", "trace");
    // env.insert("MOON_DEBUG_WASM", "true");

    sandbox.settings.bin = "moon".into();

    sandbox
        .settings
        .env
        .extend(env.into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())));
}

fn create_workspace_files(sandbox: &Sandbox) {
    if !sandbox.path().join(".moon/workspace.yml").exists() {
        sandbox.create_file(".moon/workspace.yml", "projects: ['*']");
    }
}

pub fn create_empty_moon_sandbox() -> MoonSandbox {
    MoonSandbox::new(starbase_sandbox::create_empty_sandbox())
}

pub fn create_moon_sandbox<N: AsRef<str>>(fixture: N) -> MoonSandbox {
    MoonSandbox::new(starbase_sandbox::create_sandbox(fixture))
}
