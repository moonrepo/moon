use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_assert_stderr_output,
    get_projects_fixture_configs,
};

#[test]
fn unknown_task() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("task").arg("tasks:unknown");
    });

    assert_snapshot!(get_assert_stderr_output(&assert.inner));

    assert.failure().code(1);
}

#[test]
fn shows_inputs() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("task").arg("tasks:test");
    });

    if cfg!(windows) {
        assert_snapshot!("shows_inputs_windows", assert.output());
    } else {
        assert_snapshot!("shows_inputs_unix", assert.output());
    }
}

#[test]
fn shows_outputs() {
    let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("task").arg("tasks:lint");
    });

    if cfg!(windows) {
        assert_snapshot!("shows_outputs_windows", assert.output());
    } else {
        assert_snapshot!("shows_outputs_unix", assert.output());
    }
}
