use moon_config::{
    InheritedTasksConfig, RustConfig, TaskCommandArgs, TaskConfig, ToolchainConfig,
    WorkspaceConfig, WorkspaceProjects,
};
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_node_depman_fixture_configs,
    get_node_fixture_configs, get_typescript_fixture_configs, predicates::prelude::*, Sandbox,
};
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::{collections::BTreeMap, fs::read_to_string};

fn create_cargo_task(bin: &str) -> (String, TaskConfig) {
    (
        bin.to_owned(),
        TaskConfig {
            command: Some(TaskCommandArgs::String("cargo".into())),
            args: Some(TaskCommandArgs::String(format!("run --quiet --bin {bin}"))),
            ..TaskConfig::default()
        },
    )
}

fn rust_sandbox() -> Sandbox {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([(
            "rust".to_owned(),
            ".".to_owned(),
        )])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = ToolchainConfig {
        rust: Some(RustConfig::default()),
        ..ToolchainConfig::default()
    };

    let tasks_config = InheritedTasksConfig {
        tasks: BTreeMap::from_iter([
            create_cargo_task("args"),
            create_cargo_task("exit_nonzero"),
            create_cargo_task("exit_zero"),
            create_cargo_task("panic"),
            create_cargo_task("standard"),
        ]),
        ..InheritedTasksConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "rust/cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );
    sandbox.enable_git();
    sandbox
}

#[test]
fn runs_standard_script() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:standard");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_zero() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:exit_zero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_nonzero() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:exit_nonzero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_panic() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:panic");
    });

    let output = assert.output();

    assert!(predicate::str::contains("thread 'main' panicked at 'Oops'").eval(&output));
}
