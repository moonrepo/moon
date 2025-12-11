#![allow(dead_code)]

use moon_common::{Id, is_ci};
use moon_config::PartialToolchainPluginConfig;
use moon_test_utils2::{MoonSandbox, create_moon_sandbox};
use proto_core::UnresolvedVersionSpec;

pub fn create_projects_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("projects");
    sandbox.with_default_projects();
    sandbox
}

pub fn create_tasks_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("tasks");
    sandbox.with_default_projects();
    sandbox.update_toolchains_config(|config| {
        let plugins = config.plugins.get_or_insert_default();
        plugins.insert(
            Id::raw("javascript"),
            PartialToolchainPluginConfig::default(),
        );
        plugins.insert(
            Id::raw("node"),
            PartialToolchainPluginConfig {
                version: Some(UnresolvedVersionSpec::parse("24.0.0").unwrap()),
                ..Default::default()
            },
        );
    });
    sandbox
}

pub fn create_pipeline_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("pipeline");
    sandbox.with_default_projects();
    sandbox.enable_git();
    sandbox
}

pub fn create_query_sandbox() -> MoonSandbox {
    let sandbox = create_projects_sandbox();
    sandbox.enable_git();
    sandbox
}

pub fn change_branch<T: AsRef<str>>(sandbox: &MoonSandbox, branch: T) {
    sandbox.run_git(|cmd| {
        cmd.args(["checkout", "-b", branch.as_ref()]);
    });
}

pub fn change_files<I: IntoIterator<Item = V>, V: AsRef<str>>(sandbox: &MoonSandbox, files: I) {
    let files = files
        .into_iter()
        .map(|file| file.as_ref().to_string())
        .collect::<Vec<_>>();

    for file in &files {
        sandbox.create_file(file, "contents");
    }

    // CI uses `git diff` while local uses `git status`
    if is_ci() {
        change_branch(sandbox, "branch");

        sandbox.run_git(|cmd| {
            cmd.arg("add").args(files);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Change"]);
        });
    }
}
