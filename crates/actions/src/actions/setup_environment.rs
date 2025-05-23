use crate::plugins::*;
use crate::utils::{create_hash_and_return_lock_if_changed, should_skip_action_matching};
use moon_action::{Action, ActionStatus, Operation, SetupEnvironmentNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_hash::hash_content;
use moon_pdk_api::SetupEnvironmentInput;
use moon_project::ProjectFragment;
use moon_workspace_graph::WorkspaceGraph;
use starbase_utils::json::JsonValue;
use std::sync::Arc;
use tracing::{debug, instrument};

hash_content!(
    struct SetupEnvironmentHash<'action> {
        action_node: &'action SetupEnvironmentNode,
        project: Option<&'action ProjectFragment>,
        toolchain_config: &'action JsonValue,
    }
);

#[instrument(skip(action, _action_context, app_context, workspace_graph))]
pub async fn setup_environment(
    action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    node: &SetupEnvironmentNode,
) -> miette::Result<ActionStatus> {
    // Skip this action if requested by the user
    if let Some(value) =
        should_skip_action_matching("MOON_SKIP_SETUP_ENVIRONMENT", &node.toolchain_id)
    {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            env = value,
            "Skipping environment setup because {} is set and matches",
            color::symbol("MOON_SKIP_SETUP_ENVIRONMENT")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain
    let toolchain = app_context
        .toolchain_registry
        .load(&node.toolchain_id)
        .await?;

    if !toolchain.has_func("setup_environment").await {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "Skipping environment setup as the toolchain does not support it"
        );

        return Ok(ActionStatus::Skipped);
    }

    // Build the input
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

    // Create a lock if we haven't run before
    let Some(_lock) = create_hash_and_return_lock_if_changed(
        action,
        &app_context,
        SetupEnvironmentHash {
            action_node: node,
            project: input.project.as_ref(),
            toolchain_config: &input.toolchain_config,
        },
    )
    .await?
    else {
        debug!(
            toolchain_id = node.toolchain_id.as_str(),
            "No {} toolchain changes since last run, skipping setup", toolchain.metadata.name
        );

        return Ok(ActionStatus::Skipped);
    };

    // Extract from output
    let setup_op = Operation::setup_operation(action.get_prefix())?;
    let output = toolchain.setup_environment(input).await?;
    let skipped = output.commands.is_empty() && output.operations.is_empty();

    // Execute all commands
    debug!(
        root = node.root.as_str(),
        toolchain_id = node.toolchain_id.as_str(),
        "Setting up {} environment",
        toolchain.metadata.name
    );

    if !output.commands.is_empty() {
        let operations = exec_plugin_commands(
            &toolchain,
            app_context.clone(),
            output.commands,
            ExecCommandOptions {
                prefix: action.get_prefix().into(),
                working_dir: node.root.to_logical_path(&app_context.workspace_root),
                on_exec: Some(Arc::new(move |cmd, attempts| {
                    handle_on_exec(&app_context.console, cmd, attempts)
                })),
            },
        )
        .await?;

        action.operations.extend(operations);
    }

    finalize_action_operations(
        action,
        &toolchain,
        setup_op,
        output.operations,
        output.changed_files,
    )?;

    Ok(if skipped {
        ActionStatus::Skipped
    } else {
        ActionStatus::Passed
    })
}
