use crate::operations::{ExecCommandOptions, exec_plugin_commands, handle_on_exec};
use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SetupEnvironmentNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_pdk_api::SetupEnvironmentInput;
use moon_workspace_graph::WorkspaceGraph;
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip(action, _action_context, app_context, workspace_graph))]
pub async fn setup_environment(
    action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    node: &SetupEnvironmentNode,
) -> miette::Result<ActionStatus> {
    let log_label = node.toolchain_id.as_str();
    let cache_engine = &app_context.cache_engine;

    // Create a file lock by toolchain ID, as this avoids colliding
    // setup's in this process and other parallel processes
    let _lock = cache_engine.create_lock(format!("setupEnvironment-{}", node.toolchain_id))?;

    // Skip this action if requested by the user
    if let Some(value) =
        should_skip_action_matching("MOON_SKIP_SETUP_ENVIRONMENT", &node.toolchain_id)
    {
        debug!(
            env = value,
            "Skipping {log_label} environment setup because {} is set and matches",
            color::symbol("MOON_SKIP_SETUP_ENVIRONMENT")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain and its state
    let toolchain = app_context
        .toolchain_registry
        .load(&node.toolchain_id)
        .await?;

    if !toolchain.has_func("setup_environment").await {
        debug!("Skipping {log_label} environment setup as the toolchain does not support it");

        return Ok(ActionStatus::Skipped);
    }

    debug!("Setting up {log_label} environment");

    // Build input params
    let mut input = SetupEnvironmentInput {
        context: app_context.toolchain_registry.create_context(),
        project: None,
        root: toolchain.to_virtual_path(node.root.to_logical_path(&app_context.workspace_root)),
        toolchain_config: app_context
            .toolchain_registry
            .create_config(&toolchain.id, &app_context.toolchain_config),
    };

    if let Some(project_id) = &node.project_id {
        let project = workspace_graph.get_project(project_id)?;

        input.project = Some(project.to_fragment());
        input.toolchain_config = app_context.toolchain_registry.create_merged_config(
            &toolchain.id,
            &app_context.toolchain_config,
            &project.config,
        );
    }

    // Extract commands from output
    let output = toolchain.setup_environment(input).await?;

    if output.commands.is_empty() {
        return Ok(ActionStatus::Skipped);
    }

    // Execute all commands
    action.operations.extend(
        exec_plugin_commands(
            app_context.clone(),
            output.commands,
            ExecCommandOptions {
                prefix: "setup-environment".into(),
                working_dir: node.root.to_logical_path(&app_context.workspace_root),
                on_exec: Some(Arc::new(move |cmd, attempts| {
                    handle_on_exec(&app_context.console, cmd, attempts)
                })),
            },
        )
        .await?,
    );

    Ok(ActionStatus::Passed)
}
