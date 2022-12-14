// This test is testing the project crate in the context of the project graph,
// as we need to test task inheritance, task expansion, etc...

use moon::{generate_project_graph, load_workspace_from};
use moon_config::{
    GlobalProjectConfig, PlatformType, TaskCommandArgs, TaskConfig, TaskOptionsConfig,
    WorkspaceConfig,
};
use moon_project_graph::ProjectGraph;
use moon_task::Target;
use moon_test_utils::{create_sandbox_with_config, get_tasks_fixture_configs, Sandbox};
use moon_utils::string_vec;
use rustc_hash::{FxHashMap, FxHashSet};

pub fn create_file_groups_config() -> FxHashMap<String, Vec<String>> {
    let mut map = FxHashMap::default();

    map.insert(
        String::from("static"),
        string_vec![
            "file.ts",
            "dir",
            "dir/other.tsx",
            "dir/subdir",
            "dir/subdir/another.ts",
        ],
    );

    map.insert(String::from("dirs_glob"), string_vec!["**/*"]);

    map.insert(String::from("files_glob"), string_vec!["**/*.{ts,tsx}"]);

    map.insert(String::from("globs"), string_vec!["**/*.{ts,tsx}", "*.js"]);

    map.insert(String::from("no_globs"), string_vec!["config.js"]);

    map
}

async fn tasks_sandbox() -> (Sandbox, ProjectGraph) {
    tasks_sandbox_with_config(|_, _| {}).await
}

async fn tasks_sandbox_with_config<C>(callback: C) -> (Sandbox, ProjectGraph)
where
    C: FnOnce(&mut WorkspaceConfig, &mut GlobalProjectConfig),
{
    let (mut workspace_config, toolchain_config, mut projects_config) = get_tasks_fixture_configs();

    callback(&mut workspace_config, &mut projects_config);

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).unwrap();

    (sandbox, graph)
}

mod task_inheritance {
    use super::*;

    #[tokio::test]
    async fn inherits_global_tasks() {
        let (_sandbox, project_graph) = tasks_sandbox().await;

        assert_eq!(
            project_graph
                .get("noTasks")
                .unwrap()
                .get_task("standard")
                .unwrap()
                .command,
            "cmd".to_string()
        );

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("withArgs")
                .unwrap()
                .args,
            string_vec!["--foo", "--bar", "baz"]
        );
    }

    #[tokio::test]
    async fn inherits_implicit_deps() {
        let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
            workspace_config.runner.implicit_deps = string_vec!["build", "~:build", "project:task",]
        })
        .await;

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("build")
                .unwrap()
                .deps,
            // No circular!
            vec![Target::new("project", "task").unwrap()]
        );

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("lint")
                .unwrap()
                .deps,
            vec![
                Target::new("basic", "build").unwrap(),
                Target::new("project", "task").unwrap()
            ]
        );

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("test")
                .unwrap()
                .deps,
            vec![
                Target::new("basic", "build").unwrap(),
                Target::new("project", "task").unwrap()
            ]
        );
    }

    #[tokio::test]
    async fn resolves_implicit_deps_parent_depends_on() {
        let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
            workspace_config.runner.implicit_deps = string_vec!["^:build"]
        })
        .await;

        assert_eq!(
            project_graph
                .get("buildA")
                .unwrap()
                .get_task("build")
                .unwrap()
                .deps,
            vec![
                Target::new("basic", "build").unwrap(),
                Target::new("buildC", "build").unwrap()
            ]
        );
    }

    #[tokio::test]
    async fn avoids_implicit_deps_matching_target() {
        let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
            workspace_config.runner.implicit_deps = string_vec!["basic:build"]
        })
        .await;

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("build")
                .unwrap()
                .deps,
            vec![]
        );

        assert_eq!(
            project_graph
                .get("basic")
                .unwrap()
                .get_task("lint")
                .unwrap()
                .deps,
            vec![Target::new("basic", "build").unwrap()]
        );
    }

    #[tokio::test]
    async fn inherits_implicit_inputs() {
        let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
            workspace_config.runner.implicit_inputs =
                string_vec!["package.json", "/.moon/workspace.yml"]
        })
        .await;

        assert_eq!(
            project_graph
                .get("inputA")
                .unwrap()
                .get_task("a")
                .unwrap()
                .inputs,
            string_vec!["a.ts", "package.json", "/.moon/workspace.yml"]
        );

        assert_eq!(
            project_graph
                .get("inputC")
                .unwrap()
                .get_task("c")
                .unwrap()
                .inputs,
            string_vec!["**/*", "package.json", "/.moon/workspace.yml"]
        );
    }

    #[tokio::test]
    async fn inherits_implicit_inputs_env_vars() {
        let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
            workspace_config.runner.implicit_inputs = string_vec!["$FOO", "$BAR"]
        })
        .await;

        assert_eq!(
            project_graph
                .get("inputA")
                .unwrap()
                .get_task("a")
                .unwrap()
                .input_vars,
            FxHashSet::from_iter(string_vec!["FOO", "BAR"])
        );

        assert_eq!(
            project_graph
                .get("inputC")
                .unwrap()
                .get_task("c")
                .unwrap()
                .input_vars,
            FxHashSet::from_iter(string_vec!["FOO", "BAR"])
        );
    }

    mod merge_strategies {
        use super::*;
        use moon_test_utils::pretty_assertions::assert_eq;

        fn stub_global_env_vars() -> FxHashMap<String, String> {
            FxHashMap::from_iter([
                ("GLOBAL".to_owned(), "1".to_owned()),
                ("KEY".to_owned(), "a".to_owned()),
            ])
        }

        fn stub_global_task_config() -> TaskConfig {
            TaskConfig {
                args: Some(TaskCommandArgs::Sequence(string_vec!["--a"])),
                command: Some(TaskCommandArgs::String("standard".to_owned())),
                deps: Some(string_vec!["a:standard"]),
                env: Some(stub_global_env_vars()),
                local: false,
                inputs: Some(string_vec!["a.*"]),
                outputs: Some(string_vec!["a.ts"]),
                options: TaskOptionsConfig {
                    cache: Some(true),
                    retry_count: Some(1),
                    run_deps_in_parallel: Some(true),
                    run_in_ci: Some(true),
                    ..TaskOptionsConfig::default()
                },
                platform: PlatformType::Node,
            }
        }

        #[tokio::test]
        async fn replace() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, projects_config| {
                projects_config
                    .tasks
                    .insert("standard".into(), stub_global_task_config());
            })
            .await;

            let project = project_graph.get("mergeReplace").unwrap();
            let task = project.get_task("standard").unwrap();

            assert_eq!(task.command, "newcmd".to_string());
            assert_eq!(task.args, string_vec!["--b"]);
            assert_eq!(task.env, FxHashMap::from_iter([("KEY".into(), "b".into())]));
            assert_eq!(
                task.inputs,
                string_vec![
                    "b.*",
                    "package.json",
                    "/.moon/project.yml",
                    "/.moon/toolchain.yml",
                    "/.moon/workspace.yml",
                ]
            );
            assert_eq!(task.outputs, string_vec!["b.ts"]);
        }

        #[tokio::test]
        async fn append() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, projects_config| {
                projects_config
                    .tasks
                    .insert("standard".into(), stub_global_task_config());
            })
            .await;

            let project = project_graph.get("mergeAppend").unwrap();
            let task = project.get_task("standard").unwrap();

            assert_eq!(task.command, "standard".to_string());
            assert_eq!(task.args, string_vec!["--a", "--b"]);
            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("GLOBAL".to_owned(), "1".to_owned()),
                    ("KEY".to_owned(), "b".to_owned()),
                ])
            );
            assert_eq!(
                task.inputs,
                string_vec![
                    "a.*",
                    "b.*",
                    "package.json",
                    "/.moon/project.yml",
                    "/.moon/toolchain.yml",
                    "/.moon/workspace.yml",
                ]
            );
            assert_eq!(task.outputs, string_vec!["a.ts", "b.ts"]);
        }

        #[tokio::test]
        async fn prepend() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, projects_config| {
                projects_config
                    .tasks
                    .insert("standard".into(), stub_global_task_config());
            })
            .await;

            let project = project_graph.get("mergePrepend").unwrap();
            let task = project.get_task("standard").unwrap();

            assert_eq!(task.command, "newcmd".to_string());
            assert_eq!(task.args, string_vec!["--b", "--a"]);
            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("GLOBAL".to_owned(), "1".to_owned()),
                    ("KEY".to_owned(), "a".to_owned()),
                ])
            );
            assert_eq!(
                task.inputs,
                string_vec![
                    "b.*",
                    "a.*",
                    "package.json",
                    "/.moon/project.yml",
                    "/.moon/toolchain.yml",
                    "/.moon/workspace.yml",
                ]
            );
            assert_eq!(task.outputs, string_vec!["b.ts", "a.ts"]);
        }

        #[tokio::test]
        async fn all() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, projects_config| {
                projects_config
                    .tasks
                    .insert("standard".into(), stub_global_task_config());
            })
            .await;

            let project = project_graph.get("mergeAllStrategies").unwrap();
            let task = project.get_task("standard").unwrap();

            assert_eq!(task.command, "standard".to_string());
            assert_eq!(task.args, string_vec!["--a", "--b"]);
            assert_eq!(
                task.env,
                FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned()),])
            );
            assert_eq!(
                task.inputs,
                string_vec![
                    "b.*",
                    "package.json",
                    "/.moon/project.yml",
                    "/.moon/toolchain.yml",
                    "/.moon/workspace.yml",
                ]
            );
            assert_eq!(task.outputs, string_vec!["a.ts", "b.ts"]);
        }
    }
}

mod task_expansion {
    use super::*;

    mod expand_args {
        use super::*;

        // #[tokio::test]
        // async fn resolves_file_group_tokens() {
        //     let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, projects_config| {
        //         projects_config
        //             .file_groups
        //             .extend(create_file_groups_config());
        //     })
        //     .await;

        //     assert_eq!(
        //         *project_graph
        //             .get("expandArgs")
        //             .unwrap()
        //             .get_task("fileGroups")
        //             .unwrap()
        //             .args,
        //         if cfg!(windows) {
        //             vec![
        //                 "--dirs",
        //                 ".\\dir",
        //                 ".\\dir\\subdir",
        //                 "--files",
        //                 ".\\file.ts",
        //                 ".\\dir\\other.tsx",
        //                 ".\\dir\\subdir\\another.ts",
        //                 "--globs",
        //                 "./**/*.{ts,tsx}",
        //                 "./*.js",
        //                 "--root",
        //                 ".\\dir",
        //             ]
        //         } else {
        //             vec![
        //                 "--dirs",
        //                 "./dir",
        //                 "./dir/subdir",
        //                 "--files",
        //                 "./file.ts",
        //                 "./dir/other.tsx",
        //                 "./dir/subdir/another.ts",
        //                 "--globs",
        //                 "./**/*.{ts,tsx}",
        //                 "./*.js",
        //                 "--root",
        //                 "./dir",
        //             ]
        //         },
        //     );
        // }
    }

    mod expand_deps {
        use super::*;

        #[tokio::test]
        async fn resolves_self_scope() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            assert_eq!(
                project_graph
                    .get("scopeSelf")
                    .unwrap()
                    .get_task("lint")
                    .unwrap()
                    .deps,
                vec![
                    Target::new("scopeSelf", "clean").unwrap(),
                    Target::new("scopeSelf", "build").unwrap()
                ]
            );

            // Dedupes
            assert_eq!(
                project_graph
                    .get("scopeSelf")
                    .unwrap()
                    .get_task("lintNoDupes")
                    .unwrap()
                    .deps,
                vec![Target::new("scopeSelf", "build").unwrap()]
            );

            // Ignores self
            assert_eq!(
                project_graph
                    .get("scopeSelf")
                    .unwrap()
                    .get_task("filtersSelf")
                    .unwrap()
                    .deps,
                vec![]
            );
        }

        #[tokio::test]
        async fn resolves_deps_scope() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            assert_eq!(
                project_graph
                    .get("scopeDeps")
                    .unwrap()
                    .get_task("build")
                    .unwrap()
                    .deps,
                vec![
                    Target::new("buildC", "build").unwrap(),
                    Target::new("buildA", "build").unwrap(),
                    Target::new("buildB", "build").unwrap(),
                ]
            );

            // Dedupes
            assert_eq!(
                project_graph
                    .get("scopeDeps")
                    .unwrap()
                    .get_task("buildNoDupes")
                    .unwrap()
                    .deps,
                vec![
                    Target::new("buildA", "build").unwrap(),
                    Target::new("buildC", "build").unwrap(),
                    Target::new("buildB", "build").unwrap(),
                ]
            );
        }

        #[tokio::test]
        #[should_panic(expected = "Target(NoProjectAllInTaskDeps(\":build\"))")]
        async fn errors_for_all_scope() {
            let (workspace_config, toolchain_config, projects_config) = get_tasks_fixture_configs();

            let sandbox = create_sandbox_with_config(
                "tasks",
                Some(&workspace_config),
                Some(&toolchain_config),
                Some(&projects_config),
            );

            sandbox.create_file(
                "scope-all/moon.yml",
                r#"tasks:
            build:
              command: webpack
              deps:
                - :build"#,
            );

            let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
            generate_project_graph(&mut workspace).unwrap();
        }
    }
}
