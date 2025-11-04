#![allow(dead_code)]

use moon_common::Id;
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
