use std::sync::Arc;

use moon::{generate_project_graph, load_workspace_from_sandbox};
use moon_action_context::ActionContext;
use moon_config::PlatformType;
use moon_console::Console;
use moon_emitter::Emitter;
use moon_platform_runtime::{Runtime, RuntimeReq};
use moon_runner::Runner;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs, Sandbox};
use rustc_hash::FxHashSet;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    )
}

#[tokio::test]
async fn all_inputs_when_no_files_affected() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();

    let project = project_graph.get("noAffected").unwrap();
    let task = project.get_task("primary").unwrap();
    let emitter = Emitter::new(Arc::new(workspace.clone()));

    let runner = Runner::new(
        &emitter,
        &workspace,
        &project,
        task,
        Arc::new(Console::new_testing()),
    )
    .unwrap();

    let cmd = runner
        .create_command(
            &ActionContext {
                affected_only: true,
                touched_files: FxHashSet::from_iter([]),
                ..Default::default()
            },
            &Runtime::new(PlatformType::Node, RuntimeReq::Global),
        )
        .await
        .unwrap();

    assert_eq!(cmd.args, vec!["./affected.js", "./file.txt"]);
}

#[tokio::test]
async fn dot_if_no_input_files() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();

    let project = project_graph.get("noAffected").unwrap();
    let task = project.get_task("misconfigured").unwrap();
    let emitter = Emitter::new(Arc::new(workspace.clone()));

    let runner = Runner::new(
        &emitter,
        &workspace,
        &project,
        task,
        Arc::new(Console::new_testing()),
    )
    .unwrap();

    let cmd = runner
        .create_command(
            &ActionContext {
                affected_only: true,
                touched_files: FxHashSet::from_iter([]),
                ..Default::default()
            },
            &Runtime::new(PlatformType::Node, RuntimeReq::Global),
        )
        .await
        .unwrap();

    assert_eq!(cmd.args, vec!["./affected.js", "."]);
}
