use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_assert_output,
    get_project_graph_aliases_fixture_configs, get_tasks_fixture_configs,
};

#[test]
fn all_by_default() {
    let (workspace_config, toolchain_config, projects_config) = get_tasks_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("dep-graph");
    });

    let dot = get_assert_output(&assert);

    // Snapshot is not deterministic
    assert_eq!(dot.split('\n').count(), 283);
}

#[test]
fn focused_by_target() {
    let (workspace_config, toolchain_config, projects_config) = get_tasks_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("dep-graph").arg("basic:lint");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn includes_dependencies_when_focused() {
    let (workspace_config, toolchain_config, projects_config) = get_tasks_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("dep-graph").arg("chain:e");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn includes_dependents_when_focused() {
    let (workspace_config, toolchain_config, projects_config) = get_tasks_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("dep-graph").arg("basic:build");
    });

    assert_snapshot!(get_assert_output(&assert));
}

mod aliases {
    use super::*;

    #[test]
    fn can_focus_using_an_alias() {
        let (workspace_config, toolchain_config, projects_config) =
            get_project_graph_aliases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("dep-graph").arg("@scope/pkg-foo:test");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn resolves_aliases_in_task_deps() {
        let (workspace_config, toolchain_config, projects_config) =
            get_project_graph_aliases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("dep-graph").arg("node:aliasDeps");
        });

        assert_snapshot!(get_assert_output(&assert));
    }
}
