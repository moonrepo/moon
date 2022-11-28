use moon_test_utils::{
    assert_snapshot, create_sandbox, create_sandbox_with_config, get_assert_output,
    get_project_graph_aliases_fixture_configs, get_projects_fixture_configs,
};

#[test]
fn no_projects() {
    let mut sandbox = create_sandbox("base");

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project-graph");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn many_projects() {
    let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

    let mut sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project-graph");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_with_dependencies() {
    let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

    let mut sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project-graph").arg("foo");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_no_dependencies() {
    let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

    let mut sandbox = create_sandbox_with_config(
        "projects",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("project-graph").arg("baz");
    });

    assert_snapshot!(get_assert_output(&assert));
}

mod aliases {
    use super::*;

    #[test]
    fn uses_ids_in_graph() {
        let (workspace_config, toolchain_config, projects_config) =
            get_project_graph_aliases_fixture_configs();

        let mut sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("project-graph");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_focus_using_an_alias() {
        let (workspace_config, toolchain_config, projects_config) =
            get_project_graph_aliases_fixture_configs();

        let mut sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("project-graph").arg("@scope/pkg-foo");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn resolves_aliases_in_depends_on() {
        let (workspace_config, toolchain_config, projects_config) =
            get_project_graph_aliases_fixture_configs();

        let mut sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("project-graph").arg("noLang");
        });

        assert_snapshot!(get_assert_output(&assert));
    }
}
