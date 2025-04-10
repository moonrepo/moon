use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SetupToolchainNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_common::path::encode_component;
use moon_console::Checkpoint;
use moon_pdk_api::SetupToolchainInput;
use moon_time::now_millis;
use std::sync::Arc;
use tracing::{debug, instrument};

// Temporarily match this with the legacy action!
use super::setup_toolchain::ToolCacheState;

#[instrument(skip(_action, _action_context, app_context))]
pub async fn setup_toolchain_plugin(
    _action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    node: &SetupToolchainNode,
) -> miette::Result<ActionStatus> {
    // No version configured, use globals on PATH
    if node.spec.is_global() {
        return Ok(ActionStatus::Skipped);
    }

    let log_label = node.spec.label();
    let action_key = node.spec.target();
    let cache_engine = &app_context.cache_engine;

    // Create a file lock by toolchain ID, as this avoids colliding
    // setup's in this process and other parallel processes
    let _lock = cache_engine.create_lock(format!("setupToolchain-{}", node.spec.id))?;

    // Skip this action if requested by the user
    if let Some(value) = should_skip_action_matching("MOON_SKIP_SETUP_TOOLCHAIN", &action_key) {
        debug!(
            env = value,
            "Skipping {log_label} toolchain setup because {} is set and matches",
            color::symbol("MOON_SKIP_SETUP_TOOLCHAIN")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain and its state
    let toolchain = app_context.toolchain_registry.load(&node.spec.id).await?;

    if !toolchain.supports_tier_3().await {
        debug!(
            "Skipping {log_label} toolchain setup as it does not support tier 3 (downloading and installing tools)"
        );

        return Ok(ActionStatus::Skipped);
    }

    debug!("Setting up {log_label} toolchain");

    let mut state = cache_engine.state.load_state::<ToolCacheState>(format!(
        "setupToolchain-{}.json",
        encode_component(action_key),
    ))?;

    // Run the install and setup flows
    let mut installed = false;

    if node.spec.req != state.data.requirement {
        if let Some(req) = &node.spec.req {
            let registry = &app_context.toolchain_registry;

            let output = toolchain
                .setup_toolchain(
                    SetupToolchainInput {
                        configured_version: req.to_owned(),
                        context: registry.create_context(),
                        toolchain_config: registry
                            .create_config(&toolchain.id, &app_context.toolchain_config),
                        ..Default::default()
                    },
                    || {
                        app_context
                            .console
                            .print_checkpoint(Checkpoint::Setup, format!("installing {log_label}"))
                    },
                )
                .await?;

            installed = output.installed;
        }
    }

    // Update the cache with the timestamp
    state.data.last_version_check_time = now_millis();
    state.data.requirement = node.spec.req.clone();
    state.save()?;

    Ok(if installed {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
