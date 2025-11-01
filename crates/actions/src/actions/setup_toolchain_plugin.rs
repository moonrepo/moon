use crate::plugins::*;
use crate::utils::{create_hash_and_return_lock, should_skip_action_matching};
use moon_action::{Action, ActionStatus, Operation, SetupToolchainNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_console::Checkpoint;
use moon_env_var::GlobalEnvBag;
use moon_hash::hash_content;
use moon_pdk_api::SetupToolchainInput;
use moon_platform::is_using_global_toolchain;
use std::sync::Arc;
use tracing::{debug, instrument};

hash_content!(
    struct SetupToolchainHash<'action> {
        action_node: &'action SetupToolchainNode,
    }
);

#[instrument(skip(action, _action_context, app_context))]
pub async fn setup_toolchain_plugin(
    action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    node: &SetupToolchainNode,
) -> miette::Result<ActionStatus> {
    // No version configured, use globals on PATH
    if node.toolchain.is_global()
        || is_using_global_toolchain(GlobalEnvBag::instance(), &node.toolchain.id)
    {
        debug!(
            toolchain_id = node.toolchain.id.as_str(),
            "Skipping toolchain setup because we'll be using global commands on PATH instead",
        );

        return Ok(ActionStatus::Skipped);
    }

    // Skip this action if requested by the user
    if let Some(value) =
        should_skip_action_matching("MOON_SKIP_SETUP_TOOLCHAIN", node.toolchain.target())
    {
        debug!(
            toolchain_id = node.toolchain.id.as_str(),
            version = node.toolchain.req.as_ref().map(|v| v.to_string()),
            env = value,
            "Skipping toolchain setup because {} is set and matches",
            color::symbol("MOON_SKIP_SETUP_TOOLCHAIN")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain
    let toolchain = app_context
        .toolchain_registry
        .load(&node.toolchain.id)
        .await?;

    if !toolchain.supports_tier_3().await {
        debug!(
            toolchain_id = node.toolchain.id.as_str(),
            version = node.toolchain.req.as_ref().map(|v| v.to_string()),
            "Skipping toolchain setup as the toolchain does not support tier 3 (downloading and installing)"
        );

        return Ok(ActionStatus::Skipped);
    }

    // Create a lock to avoid collisions
    let _lock = create_hash_and_return_lock(
        action,
        &app_context,
        SetupToolchainHash { action_node: node },
    )?;

    // Run the install and setup flows
    debug!(
        toolchain_id = node.toolchain.id.as_str(),
        version = node.toolchain.req.as_ref().map(|v| v.to_string()),
        "Setting up {} toolchain",
        toolchain.metadata.name
    );

    let setup_op = Operation::setup_operation(action.get_prefix())?;
    let output = toolchain
        .setup_toolchain(
            SetupToolchainInput {
                configured_version: node.toolchain.req.clone(),
                context: app_context.toolchain_registry.create_context(),
                toolchain_config: app_context
                    .toolchain_registry
                    .create_config(&toolchain.id, &app_context.toolchain_config),
                version: None,
            },
            || {
                app_context.console.print_checkpoint(
                    Checkpoint::Setup,
                    format!("installing {}", node.toolchain.label()),
                )
            },
        )
        .await?;

    finalize_action_operations(
        action,
        &toolchain,
        setup_op,
        output.operations,
        output.changed_files,
    )?;

    Ok(if output.installed {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
