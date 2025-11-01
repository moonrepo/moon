use moon_test_utils2::{MoonSandbox, create_empty_moon_sandbox, create_moon_sandbox};
use starbase_sandbox::assert_snapshot;

fn create_projects_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("projects");
    sandbox.with_default_projects();
    sandbox
}

mod project_graph {
    use super::*;

    #[test]
    fn no_projects() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn many_projects() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_with_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("dep-foo").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_with_dependents() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph")
                .arg("dep-bar")
                .arg("--dot")
                .arg("--dependents");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_no_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("dep-baz").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn outputs_json() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("dep-foo").arg("--json");
        });

        assert_ne!(assert.output(), "{}");
    }

    // mod aliases {
    //     use super::*;

    //     #[test]
    //     fn uses_ids_in_graph() {
    //         let (workspace_config, toolchain_config, tasks_config) =
    //             get_project_graph_aliases_fixture_configs();

    //         let sandbox = create_sandbox_with_config(
    //             "project-graph/aliases",
    //             Some(workspace_config),
    //             Some(toolchain_config),
    //             Some(tasks_config),
    //         );

    //         let assert = sandbox.run_bin(|cmd| {
    //             cmd.arg("project-graph").arg("--dot");
    //         });

    //         assert_snapshot!(assert.output());
    //     }

    //     #[test]
    //     fn can_focus_using_an_alias() {
    //         let (workspace_config, toolchain_config, tasks_config) =
    //             get_project_graph_aliases_fixture_configs();

    //         let sandbox = create_sandbox_with_config(
    //             "project-graph/aliases",
    //             Some(workspace_config),
    //             Some(toolchain_config),
    //             Some(tasks_config),
    //         );

    //         let assert = sandbox.run_bin(|cmd| {
    //             cmd.arg("project-graph").arg("@scope/pkg-foo").arg("--dot");
    //         });

    //         assert_snapshot!(assert.output());
    //     }

    //     #[test]
    //     fn resolves_aliases_in_depends_on() {
    //         let (workspace_config, toolchain_config, tasks_config) =
    //             get_project_graph_aliases_fixture_configs();

    //         let sandbox = create_sandbox_with_config(
    //             "project-graph/aliases",
    //             Some(workspace_config),
    //             Some(toolchain_config),
    //             Some(tasks_config),
    //         );

    //         let assert = sandbox.run_bin(|cmd| {
    //             cmd.arg("project-graph").arg("noLang").arg("--dot");
    //         });

    //         assert_snapshot!(assert.output());
    //     }
    // }
}
