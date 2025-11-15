#![allow(clippy::disallowed_types)]

use moon_config::{
    PartialExtensionsConfig, PartialToolchainsConfig, PartialWorkspaceConfig,
    PartialWorkspaceProjects, PartialWorkspaceProjectsConfig,
};
pub use starbase_sandbox::{Sandbox, SandboxAssert, SandboxSettings, create_temp_dir};
use starbase_utils::yaml;
use std::collections::HashMap;
use std::ops::Deref;

pub struct MoonSandbox {
    pub sandbox: Sandbox,
}

impl MoonSandbox {
    pub fn new(mut sandbox: Sandbox, create: bool) -> Self {
        apply_settings(&mut sandbox);

        if create {
            create_workspace_files(&sandbox);
        }

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

    pub fn update_extensions_config(&self, op: impl FnOnce(&mut PartialExtensionsConfig)) {
        let path = self.path().join(".moon/extensions.yml");

        let mut config: PartialExtensionsConfig = if path.exists() {
            yaml::read_file(&path).unwrap()
        } else {
            Default::default()
        };

        op(&mut config);

        yaml::write_file(path, &config).unwrap();
    }

    pub fn update_toolchains_config(&self, op: impl FnOnce(&mut PartialToolchainsConfig)) {
        let path = self.path().join(".moon/toolchains.yml");

        let mut config: PartialToolchainsConfig = if path.exists() {
            yaml::read_file(&path).unwrap()
        } else {
            Default::default()
        };

        op(&mut config);

        yaml::write_file(path, &config).unwrap();
    }

    pub fn update_workspace_config(&self, op: impl FnOnce(&mut PartialWorkspaceConfig)) {
        let path = self.path().join(".moon/workspace.yml");

        let mut config: PartialWorkspaceConfig = if path.exists() {
            yaml::read_file(&path).unwrap()
        } else {
            Default::default()
        };

        op(&mut config);

        yaml::write_file(path, &config).unwrap();
    }

    pub fn with_default_projects(&self) {
        self.update_workspace_config(|config| {
            let mut projects = PartialWorkspaceProjectsConfig {
                globs: Some(vec![
                    "*/moon.yml".into(),
                    "!.home".into(),
                    "!.moon".into(),
                    "!.proto".into(),
                ]),
                ..Default::default()
            };

            if self.path().join("moon.yml").exists() {
                projects
                    .sources
                    .get_or_insert_default()
                    .insert("root".try_into().unwrap(), ".".into());
            }

            config.projects = Some(PartialWorkspaceProjects::Both(projects));
        });
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
    let root_dir = std::env::current_dir().unwrap(); // crates/cli
    let wasm_dir = root_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wasm/prebuilts");

    let mut env = HashMap::new();
    env.insert("RUST_BACKTRACE", "1");
    env.insert("WASMTIME_BACKTRACE_DETAILS", "1");
    env.insert("NO_COLOR", "1");
    env.insert("COLUMNS", "150");
    // Store plugins in the sandbox
    env.insert("MOON_HOME", moon_dir.to_str().unwrap());
    env.insert("WASM_PREBUILTS_DIR", wasm_dir.to_str().unwrap());
    // env.insert("PROTO_HOME", path.join(".proto"));
    // Let our code know we're running tests
    env.insert("MOON_TEST", "true");
    env.insert("STARBASE_TEST", "true");
    // Don't exhaust all cores on the machine
    env.insert("MOON_CONCURRENCY", "2");
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

pub fn create_empty_sandbox() -> MoonSandbox {
    MoonSandbox::new(starbase_sandbox::create_empty_sandbox(), false)
}

pub fn create_empty_moon_sandbox() -> MoonSandbox {
    MoonSandbox::new(starbase_sandbox::create_empty_sandbox(), true)
}

pub fn create_moon_sandbox<N: AsRef<str>>(fixture: N) -> MoonSandbox {
    MoonSandbox::new(starbase_sandbox::create_sandbox(fixture), true)
}
