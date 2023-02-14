use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_logger::debug;
use moon_platform::Runtime;
use moon_utils::time;
use moon_workspace::Workspace;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:setup-tool";

pub async fn setup_tool(
    _action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
) -> Result<ActionStatus, PipelineError> {
    env::set_var("MOON_RUNNING_ACTION", "setup-tool");

    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    let mut workspace = workspace.write().await;
    let mut cache = workspace.cache.cache_tool_state(runtime)?;
    let platform = workspace.platforms.get_mut(runtime)?;

    // Platform may only have tier 2 support, not 3
    if !platform.is_toolchain_enabled() {
        debug!(
            target: LOG_TARGET,
            "Platform is not toolchain enabled, skipping setup",
        );

        return Ok(ActionStatus::Skipped);
    }

    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        runtime.label()
    );

    // Install and setup the specific tool + version in the toolchain!
    let context = context.read().await;
    let installed_count = platform
        .setup_tool(&context, runtime, &mut cache.last_versions)
        .await?;

    // Update the cache with the timestamp
    cache.last_version_check_time = time::now_millis();
    cache.save()?;

    Ok(if installed_count > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
