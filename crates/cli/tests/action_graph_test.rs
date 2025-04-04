use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs,
    get_tasks_fixture_configs,
};

mod action_graph {
    use super::*;

    #[test]
    fn all_by_default() {
        let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "tasks",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("action-graph").arg("--dot");
        });

        let dot = assert.output();

        // Snapshot is not deterministic
        assert_eq!(dot.split('\n').count(), 447);
    }

    #[test]
    fn focused_by_target() {
        let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "tasks",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("ag").arg("--dot").arg("basic:lint");
        });

        assert_snapshot!(assert.output());
    }

    // #[test]
    // fn focused_by_task_in_cwd() {
    //     let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

    //     let sandbox = create_sandbox_with_config(
    //         "tasks",
    //         Some(workspace_config),
    //         Some(toolchain_config),
    //         Some(tasks_config),
    //     );

    //     let assert = sandbox.run_moon(|cmd| {
    //         cmd.arg("action-graph").arg("--dot").arg("lint");
    //         cmd.current_dir(sandbox.path().join("basic"));
    //     });

    //     assert_snapshot!(assert.output());
    // }

    #[test]
    fn includes_dependencies_when_focused() {
        let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "tasks",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("action-graph").arg("--dot").arg("chain:e");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn includes_dependents_when_focused() {
        let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "tasks",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("action-graph").arg("--dot").arg("basic:build");
        });

        assert_snapshot!(assert.output());

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("action-graph")
                .arg("--dot")
                .arg("--dependents")
                .arg("basic:build");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn outputs_json() {
        let (workspace_config, toolchain_config, tasks_config) = get_tasks_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "tasks",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("action-graph").arg("--json").arg("basic:lint");
        });

        assert_snapshot!(assert.output());
    }

    mod aliases {
        use super::*;

        #[test]
        fn can_focus_using_an_alias() {
            let (workspace_config, toolchain_config, tasks_config) =
                get_project_graph_aliases_fixture_configs();

            let sandbox = create_sandbox_with_config(
                "project-graph/aliases",
                Some(workspace_config),
                Some(toolchain_config),
                Some(tasks_config),
            );

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("action-graph")
                    .arg("--dot")
                    .arg("@scope/pkg-foo:test");
            });

            assert_snapshot!(assert.output());
        }

        #[test]
        fn resolves_aliases_in_task_deps() {
            let (workspace_config, toolchain_config, tasks_config) =
                get_project_graph_aliases_fixture_configs();

            let sandbox = create_sandbox_with_config(
                "project-graph/aliases",
                Some(workspace_config),
                Some(toolchain_config),
                Some(tasks_config),
            );

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("action-graph").arg("--dot").arg("node:aliasDeps");
            });

            assert_snapshot!(assert.output());
        }
    }
}
