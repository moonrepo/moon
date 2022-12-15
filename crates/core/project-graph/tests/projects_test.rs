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
use moon_utils::{glob, string_vec};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;

async fn tasks_sandbox() -> (Sandbox, ProjectGraph) {
    tasks_sandbox_with_config(|_, _| {}).await
}

async fn tasks_sandbox_with_config<C>(callback: C) -> (Sandbox, ProjectGraph)
where
    C: FnOnce(&mut WorkspaceConfig, &mut GlobalProjectConfig),
{
    tasks_sandbox_internal(callback, |_| {}).await
}

async fn tasks_sandbox_with_setup<C>(callback: C) -> (Sandbox, ProjectGraph)
where
    C: FnOnce(&Sandbox),
{
    tasks_sandbox_internal(|_, _| {}, callback).await
}

async fn tasks_sandbox_internal<C, S>(cfg_callback: C, box_callback: S) -> (Sandbox, ProjectGraph)
where
    C: FnOnce(&mut WorkspaceConfig, &mut GlobalProjectConfig),
    S: FnOnce(&Sandbox),
{
    let (mut workspace_config, toolchain_config, mut projects_config) = get_tasks_fixture_configs();

    cfg_callback(&mut workspace_config, &mut projects_config);

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    box_callback(&sandbox);

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

        #[tokio::test]
        async fn resolves_file_group_tokens() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            assert_eq!(
                *project_graph
                    .get("tokens")
                    .unwrap()
                    .get_task("argsFileGroups")
                    .unwrap()
                    .args,
                if cfg!(windows) {
                    vec![
                        "--dirs",
                        ".\\dir",
                        ".\\dir\\subdir",
                        "--files",
                        ".\\file.ts",
                        ".\\dir\\other.tsx",
                        ".\\dir\\subdir\\another.ts",
                        "--globs",
                        "./**/*.{ts,tsx}",
                        "./*.js",
                        "--root",
                        ".\\dir",
                    ]
                } else {
                    vec![
                        "--dirs",
                        "./dir",
                        "./dir/subdir",
                        "--files",
                        "./file.ts",
                        "./dir/other.tsx",
                        "./dir/subdir/another.ts",
                        "--globs",
                        "./**/*.{ts,tsx}",
                        "./*.js",
                        "--root",
                        "./dir",
                    ]
                },
            );
        }

        #[tokio::test]
        async fn resolves_file_group_tokens_from_workspace() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();

            assert_eq!(
                *project.get_task("argsFileGroupsWorkspace").unwrap().args,
                vec![
                    "--dirs",
                    project.root.join("dir").to_str().unwrap(),
                    project.root.join("dir").join("subdir").to_str().unwrap(),
                    "--files",
                    project.root.join("file.ts").to_str().unwrap(),
                    project.root.join("dir").join("other.tsx").to_str().unwrap(),
                    project
                        .root
                        .join("dir")
                        .join("subdir")
                        .join("another.ts")
                        .to_str()
                        .unwrap(),
                    "--globs",
                    glob::remove_drive_prefix(
                        glob::normalize(project.root.join("**/*.{ts,tsx}")).unwrap()
                    )
                    .as_str(),
                    glob::remove_drive_prefix(glob::normalize(project.root.join("*.js")).unwrap())
                        .as_str(),
                    "--root",
                    project.root.join("dir").to_str().unwrap(),
                ],
            );
        }

        #[tokio::test]
        async fn resolves_var_tokens() {
            let (sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();

            assert_eq!(
                *project.get_task("argsVars").unwrap().args,
                vec![
                    "some/$unknown/var",
                    "--pid",
                    "tokens/foo",
                    "--proot",
                    project.root.to_str().unwrap(),
                    "--psource",
                    // This is wonky but also still valid
                    if cfg!(windows) {
                        "foo\\tokens"
                    } else {
                        "foo/tokens"
                    },
                    "--target",
                    "foo/tokens:argsVars/bar",
                    "--tid=argsVars",
                    "--wsroot",
                    sandbox.path().to_str().unwrap(),
                    "--last",
                    "unknown-javascript"
                ]
            );
        }
    }

    mod expand_deps {
        use super::*;

        #[tokio::test]
        async fn inherits_implicit_deps() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|workspace_config, _| {
                workspace_config.runner.implicit_deps =
                    string_vec!["build", "~:build", "project:task",]
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
            tasks_sandbox_with_setup(|sandbox| {
                sandbox.create_file(
                    "scope-all/moon.yml",
                    r#"tasks:
                build:
                  command: webpack
                  deps:
                    - :build"#,
                );
            })
            .await;
        }
    }

    mod expand_env {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "Error parsing line: 'FOO', error at line index: 3")]
        async fn errors_on_invalid_file() {
            tasks_sandbox_with_setup(|sandbox| {
                sandbox.create_file("expand-env/.env", "FOO");
            })
            .await;
        }

        #[tokio::test]
        // Windows = "The system cannot find the file specified"
        // Unix = "No such file or directory"
        #[should_panic(expected = "InvalidEnvFile")]
        async fn errors_on_missing_file() {
            // `expand_env` has a CI check that avoids this from crashing, so emulate it
            if moon_utils::is_ci() {
                panic!("InvalidEnvFile");
            } else {
                tasks_sandbox_with_setup(|sandbox| {
                    std::fs::remove_file(sandbox.path().join("expand-env/.env")).unwrap();
                })
                .await;
            }
        }

        #[tokio::test]
        async fn loads_using_bool() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("expandEnv").unwrap();
            let task = project.get_task("envFile").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("FOO".to_owned(), "abc".to_owned()),
                    ("BAR".to_owned(), "123".to_owned())
                ])
            );

            assert!(task.inputs.contains(&".env".to_owned()));
            assert!(task.input_paths.contains(&project.root.join(".env")));
        }

        #[tokio::test]
        async fn loads_using_custom_name() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("expandEnv").unwrap();
            let task = project.get_task("envFileNamed").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("FOO".to_owned(), "xyz".to_owned()),
                    ("BAR".to_owned(), "456".to_owned())
                ])
            );

            assert!(task.inputs.contains(&".env.production".to_owned()));
            assert!(task
                .input_paths
                .contains(&project.root.join(".env.production")));
        }

        #[tokio::test]
        async fn doesnt_override_other_env() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("expandEnv").unwrap();
            let task = project.get_task("mergeWithEnv").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("FOO".to_owned(), "original".to_owned()),
                    ("BAR".to_owned(), "123".to_owned())
                ])
            );
        }
    }

    mod expand_inputs {
        use super::*;

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

        #[tokio::test]
        async fn resolves_file_group_tokens() {
            let (sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();
            let task = project.get_task("inputsFileGroups").unwrap();

            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([
                    glob::normalize(project.root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project.root.join("*.js")).unwrap()
                ]),
            );

            let a: FxHashSet<PathBuf> =
                FxHashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: FxHashSet<PathBuf> = FxHashSet::from_iter(
                vec![
                    sandbox.path().join(".moon/workspace.yml"),
                    sandbox.path().join(".moon/toolchain.yml"),
                    sandbox.path().join(".moon/project.yml"),
                    sandbox.path().join("package.json"),
                    project.root.join("package.json"),
                    project.root.join("file.ts"),
                    project.root.join("dir"),
                    project.root.join("dir/subdir"),
                    project.root.join("file.ts"),
                    project.root.join("dir/other.tsx"),
                    project.root.join("dir/subdir/another.ts"),
                ]
                .iter()
                .map(PathBuf::from),
            );

            assert_eq!(a, b);
        }

        #[tokio::test]
        async fn resolves_var_tokens() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();
            let task = project.get_task("inputsVars").unwrap();

            assert!(task
                .input_globs
                .contains(&glob::normalize(project.root.join("$unknown.*")).unwrap()));

            assert!(task
                .input_paths
                .contains(&project.root.join("dir/javascript/file")));

            assert!(task
                .input_paths
                .contains(&project.root.join("file.unknown")));
        }

        #[tokio::test]
        async fn expands_into_correct_containers() {
            let (sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();
            let task = project.get_task("inputs").unwrap();

            assert!(task
                .input_globs
                .contains(&glob::normalize(project.root.join("glob/*")).unwrap()));
            assert!(task
                .input_globs
                .contains(&glob::normalize(sandbox.path().join("glob.*")).unwrap()));

            assert!(task.input_paths.contains(&project.root.join("path.ts")));
            assert!(task.input_paths.contains(&sandbox.path().join("path/dir")));

            assert!(task.input_vars.contains("VAR"));
            assert!(task.input_vars.contains("FOO_BAR"));
            assert!(!task.input_vars.contains("UNKNOWN"));
        }
    }

    mod expand_outputs {
        use super::*;

        #[tokio::test]
        async fn expands_into_correct_containers() {
            let (_sandbox, project_graph) = tasks_sandbox().await;

            let project = project_graph.get("tokens").unwrap();
            let task = project.get_task("outputs").unwrap();

            assert!(task.output_paths.contains(&project.root.join("dir")));
        }

        #[tokio::test]
        #[should_panic(expected = "NoOutputGlob")]
        async fn errors_for_globs() {
            tasks_sandbox_with_setup(|sandbox| {
                sandbox.create_file(
                    "expand-outputs/moon.yml",
                    r#"tasks:
                        command:
                            outputs:
                                - 'glob/*'"#,
                );
            })
            .await;
        }
    }
}
