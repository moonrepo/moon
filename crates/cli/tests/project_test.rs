use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_assert_stderr_output,
    get_cases_fixture_configs, get_projects_fixture_configs,
};

#[test]
fn unknown_project() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("unknown");
    });

    assert_snapshot!(get_assert_stderr_output(&assert.inner));

    assert.failure().code(1);
}

#[test]
fn empty_config() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("emptyConfig");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn no_config() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("noConfig");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn basic_config() {
    // with dependsOn and fileGroups
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("basic");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn advanced_config() {
    // with project metadata
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("advanced");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn depends_on_paths() {
    // shows dependsOn paths when they exist
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("foo");
    });

    assert_snapshot!(assert.output_standardized());
}

#[test]
fn with_tasks() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("tasks");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn root_level() {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project").arg("root");
    });

    assert_snapshot!(assert.output());
}
