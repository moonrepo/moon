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

}

