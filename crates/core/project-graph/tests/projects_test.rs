// This test is testing the project crate in the context of the project graph,
// as we need to test task inheritance, task expansion, etc...

use moon::{generate_project_graph, load_workspace_from};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{
    InputPath, LanguageType, OutputPath, PartialInheritedTasksConfig, PartialNodeConfig,
    PartialRustConfig, PartialTaskCommandArgs, PartialTaskConfig, PartialTaskOptionsConfig,
    PartialToolchainConfig, PartialWorkspaceConfig, PartialWorkspaceProjects, PlatformType,
};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_test_utils::{
    create_sandbox, create_sandbox_with_config, get_tasks_fixture_configs, Sandbox,
};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::string_vec;
use std::collections::BTreeMap;
use std::env;
use std::fs;

async fn tasks_sandbox() -> (Sandbox, ProjectGraph) {
    tasks_sandbox_with_config(|_, _| {}).await
}

async fn tasks_sandbox_with_config<C>(callback: C) -> (Sandbox, ProjectGraph)
where
    C: FnOnce(&mut PartialWorkspaceConfig, &mut PartialInheritedTasksConfig),
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
    C: FnOnce(&mut PartialWorkspaceConfig, &mut PartialInheritedTasksConfig),
    S: FnOnce(&Sandbox),
{
    let (mut workspace_config, toolchain_config, mut tasks_config) = get_tasks_fixture_configs();

    cfg_callback(&mut workspace_config, &mut tasks_config);

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    box_callback(&sandbox);

    env::set_var("MOON_DISABLE_OVERLAPPING_OUTPUTS", "true");

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    env::remove_var("MOON_DISABLE_OVERLAPPING_OUTPUTS");

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
    async fn inherits_global_file_groups() {
        let (_sandbox, project_graph) = tasks_sandbox().await;

        assert_eq!(
            *project_graph
                .get("noTasks")
                .unwrap()
                .file_groups
                .get("files_glob")
                .unwrap()
                .globs,
            string_vec!["no-tasks/**/*.{ts,tsx}"]
        );

        assert_eq!(
            *project_graph
                .get("noTasks")
                .unwrap()
                .file_groups
                .get("static")
                .unwrap()
                .files,
            string_vec![
                "no-tasks/file.ts",
                "no-tasks/dir",
                "no-tasks/dir/other.tsx",
                "no-tasks/dir/subdir",
                "no-tasks/dir/subdir/another.ts"
            ]
        );
    }

    #[tokio::test]
    async fn can_override_global_file_groups() {
        let (_sandbox, project_graph) = tasks_sandbox().await;

        assert_eq!(
            *project_graph
                .get("fileGroups")
                .unwrap()
                .file_groups
                .get("files_glob")
                .unwrap()
                .globs,
            string_vec!["file-groups/**/*.{ts,tsx}"]
        );

        assert_eq!(
            *project_graph
                .get("fileGroups")
                .unwrap()
                .file_groups
                .get("static")
                .unwrap()
                .files,
            string_vec!["file-groups/file.js"]
        );
    }

    #[tokio::test]
    async fn inherits_tag_based_tasks() {
        let (_sandbox, project_graph) = tasks_sandbox_with_setup(|sandbox| {
            fs::create_dir_all(sandbox.path().join(".moon/tasks")).unwrap();

            fs::write(
                sandbox.path().join(".moon/tasks/tag-will-inherit.yml"),
                r#"
tasks:
    fromTagCommand:
        command: 'from-tag'
"#,
            )
            .unwrap();

            fs::write(
                sandbox.path().join(".moon/tasks/tag-wont-inherit.yml"),
                r#"
tasks:
    otherTagCommand:
        command: 'other-tag'
"#,
            )
            .unwrap();
        })
        .await;

        let project = project_graph.get("inheritTags").unwrap();

        assert_eq!(
            project.get_task("nonTagCommand").unwrap().command,
            "non-tag".to_string()
        );
        assert_eq!(
            project.get_task("fromTagCommand").unwrap().command,
            "from-tag".to_string()
        );
        assert_eq!(
            project.tasks.keys().cloned().collect::<Vec<_>>(),
            string_vec![
                "fromTagCommand",
                "nonTagCommand",
                "standard",
                "withArgs",
                "withInputs",
                "withOutputs"
            ]
        );
    }

    mod merge_strategies {
        use super::*;

        fn stub_global_env_vars() -> FxHashMap<String, String> {
            FxHashMap::from_iter([
                ("GLOBAL".to_owned(), "1".to_owned()),
                ("KEY".to_owned(), "a".to_owned()),
            ])
        }

        fn stub_global_task_config() -> PartialTaskConfig {
            PartialTaskConfig {
                args: Some(PartialTaskCommandArgs::List(string_vec!["--a"])),
                command: Some(PartialTaskCommandArgs::String("standard".to_owned())),
                deps: Some(vec![Target::parse("a:standard").unwrap()]),
                env: Some(stub_global_env_vars()),
                inputs: Some(vec![InputPath::ProjectGlob("a.*".into())]),
                outputs: Some(vec![OutputPath::ProjectFile("a.ts".into())]),
                options: Some(PartialTaskOptionsConfig {
                    cache: Some(true),
                    retry_count: Some(1),
                    run_deps_in_parallel: Some(true),
                    run_in_ci: Some(true),
                    ..PartialTaskOptionsConfig::default()
                }),
                platform: Some(PlatformType::Node),
                ..PartialTaskConfig::default()
            }
        }

        #[tokio::test]
        async fn replace() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, tasks_config| {
                tasks_config
                    .tasks
                    .as_mut()
                    .unwrap()
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
                vec![
                    InputPath::ProjectGlob("b.*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into())
                ]
            );
            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("b.ts".into())]);
        }

        #[tokio::test]
        async fn append() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, tasks_config| {
                tasks_config
                    .tasks
                    .as_mut()
                    .unwrap()
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
                vec![
                    InputPath::ProjectGlob("a.*".into()),
                    InputPath::ProjectGlob("b.*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("a.ts".into()),
                    OutputPath::ProjectFile("b.ts".into())
                ]
            );
        }

        #[tokio::test]
        async fn prepend() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, tasks_config| {
                tasks_config
                    .tasks
                    .as_mut()
                    .unwrap()
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
                vec![
                    InputPath::ProjectGlob("b.*".into()),
                    InputPath::ProjectGlob("a.*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("b.ts".into()),
                    OutputPath::ProjectFile("a.ts".into())
                ]
            );
        }

        #[tokio::test]
        async fn all() {
            let (_sandbox, project_graph) = tasks_sandbox_with_config(|_, tasks_config| {
                tasks_config
                    .tasks
                    .as_mut()
                    .unwrap()
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
                vec![
                    InputPath::ProjectGlob("b.*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("a.ts".into()),
                    OutputPath::ProjectFile("b.ts".into())
                ]
            );
        }
    }

    mod workspace_override {
        use super::*;
        use std::collections::BTreeMap;

        async fn tasks_inheritance_sandbox() -> (Sandbox, ProjectGraph) {
            let workspace_config = PartialWorkspaceConfig {
                projects: Some(PartialWorkspaceProjects::Globs(string_vec!["*"])),
                ..PartialWorkspaceConfig::default()
            };

            let toolchain_config = PartialToolchainConfig {
                node: Some(PartialNodeConfig::default()),
                ..PartialToolchainConfig::default()
            };

            let tasks_config = PartialInheritedTasksConfig {
                tasks: Some(BTreeMap::from_iter([
                    (
                        "a".into(),
                        PartialTaskConfig {
                            command: Some(PartialTaskCommandArgs::String("a".into())),
                            inputs: Some(vec![InputPath::ProjectFile("a".into())]),
                            platform: Some(PlatformType::Unknown),
                            ..PartialTaskConfig::default()
                        },
                    ),
                    (
                        "b".into(),
                        PartialTaskConfig {
                            command: Some(PartialTaskCommandArgs::String("b".into())),
                            inputs: Some(vec![InputPath::ProjectFile("b".into())]),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        },
                    ),
                    (
                        "c".into(),
                        PartialTaskConfig {
                            command: Some(PartialTaskCommandArgs::String("c".into())),
                            inputs: Some(vec![InputPath::ProjectFile("c".into())]),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        },
                    ),
                ])),
                ..PartialInheritedTasksConfig::default()
            };

            let sandbox = create_sandbox_with_config(
                "task-inheritance",
                Some(workspace_config),
                Some(toolchain_config),
                Some(tasks_config),
            );

            let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
            let graph = generate_project_graph(&mut workspace).await.unwrap();

            (sandbox, graph)
        }

        fn get_project_task_ids(project: &Project) -> Vec<String> {
            let mut ids = project
                .tasks
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<String>>();
            ids.sort();
            ids
        }

        #[tokio::test]
        async fn include() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("include").unwrap()),
                string_vec!["a", "c"]
            );
        }

        #[tokio::test]
        async fn include_none() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("include-none").unwrap()),
                string_vec![]
            );
        }

        #[tokio::test]
        async fn exclude() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("exclude").unwrap()),
                string_vec!["b"]
            );
        }

        #[tokio::test]
        async fn exclude_all() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("exclude-all").unwrap()),
                string_vec![]
            );
        }

        #[tokio::test]
        async fn exclude_none() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("exclude-none").unwrap()),
                string_vec!["a", "b", "c"]
            );
        }

        #[tokio::test]
        async fn exclude_scoped_inheritance() {
            let sandbox = create_sandbox("config-inheritance/override");
            let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
            let project_graph = generate_project_graph(&mut workspace).await.unwrap();

            assert_eq!(
                get_project_task_ids(project_graph.get("excluded").unwrap()),
                string_vec![]
            );
        }

        #[tokio::test]
        async fn rename() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let ids = string_vec!["bar", "baz", "foo"];

            assert_eq!(
                get_project_task_ids(project_graph.get("rename").unwrap()),
                ids
            );

            for id in &ids {
                let task = project_graph.get("rename").unwrap().get_task(id).unwrap();

                assert_eq!(task.id, id.to_owned());
                assert_eq!(task.target.id, format!("rename:{id}"));
            }
        }

        #[tokio::test]
        async fn rename_scoped_inheritance() {
            let sandbox = create_sandbox("config-inheritance/override");
            let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
            let project_graph = generate_project_graph(&mut workspace).await.unwrap();

            assert_eq!(
                get_project_task_ids(project_graph.get("renamed").unwrap()),
                string_vec!["cmd"]
            );

            let task = project_graph
                .get("renamed")
                .unwrap()
                .get_task("cmd")
                .unwrap();

            assert_eq!(task.id, "cmd");
            assert_eq!(task.target.id, "renamed:cmd");
        }

        #[tokio::test]
        async fn rename_merge() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("rename-merge").unwrap()),
                string_vec!["b", "c", "foo"]
            );

            let task = project_graph
                .get("rename-merge")
                .unwrap()
                .get_task("foo")
                .unwrap();

            assert_eq!(task.id, "foo");
            assert_eq!(task.target.id, "rename-merge:foo");
            assert_eq!(task.args, string_vec!["renamed-and-merge-foo"]);
        }

        #[tokio::test]
        async fn include_exclude() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("include-exclude").unwrap()),
                string_vec!["a"]
            );
        }

        #[tokio::test]
        async fn include_exclude_rename() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            assert_eq!(
                get_project_task_ids(project_graph.get("include-exclude-rename").unwrap()),
                string_vec!["only"]
            );

            let task = project_graph
                .get("include-exclude-rename")
                .unwrap()
                .get_task("only")
                .unwrap();

            assert_eq!(task.id, "only");
            assert_eq!(task.target.id, "include-exclude-rename:only");
        }

        #[tokio::test]
        async fn handles_platforms() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let project = project_graph.get("platform-detect").unwrap();

            assert_eq!(
                project.get_task("a").unwrap().platform,
                PlatformType::System
            );
            assert_eq!(
                project.get_task("b").unwrap().platform,
                PlatformType::System
            );
            assert_eq!(
                project.get_task("c").unwrap().platform,
                PlatformType::System
            );
        }

        #[tokio::test]
        async fn handles_platforms_with_language() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let project = project_graph.get("platform-detect-lang").unwrap();

            assert_eq!(project.get_task("a").unwrap().platform, PlatformType::Node);
            assert_eq!(
                project.get_task("b").unwrap().platform,
                PlatformType::System
            );
            assert_eq!(
                project.get_task("c").unwrap().platform,
                PlatformType::System
            );
        }

        #[tokio::test]
        async fn resets_inputs_to_empty() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let project = project_graph.get("inputs").unwrap();

            assert_eq!(
                project.get_task("a").unwrap().inputs,
                vec![
                    InputPath::ProjectFile("a".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
        }

        #[tokio::test]
        async fn replaces_inputs() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let project = project_graph.get("inputs").unwrap();

            assert_eq!(
                project.get_task("b").unwrap().inputs,
                vec![
                    InputPath::ProjectFile("other".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
        }

        #[tokio::test]
        async fn appends_inputs() {
            let (_sandbox, project_graph) = tasks_inheritance_sandbox().await;

            let project = project_graph.get("inputs").unwrap();

            assert_eq!(
                project.get_task("c").unwrap().inputs,
                vec![
                    InputPath::ProjectFile("c".into()),
                    InputPath::ProjectFile("other".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
        }
    }
}

