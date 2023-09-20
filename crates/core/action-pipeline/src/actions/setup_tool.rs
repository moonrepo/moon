use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_cache_item::cache_item;
use moon_logger::debug;
use moon_platform::{PlatformManager, Runtime};
use moon_utils::time;
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

cache_item!(
    pub struct ToolState {
        pub last_versions: FxHashMap<String, proto_core::Version>,
        pub last_version_check_time: u128,
    }
);

const LOG_TARGET: &str = "moon:action:setup-tool";

pub async fn setup_tool(
    _action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "setup-tool");

    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        runtime.label()
    );

    let workspace = workspace.write().await;
    let context = context.read().await;

    let mut state = workspace.cache_engine.cache_state::<ToolState>(format!(
        "tool{}-{}.json",
        runtime,
        runtime.version()
    ))?;

    // Install and setup the specific tool + version in the toolchain!
    let installed_count = PlatformManager::write()
        .get_mut(runtime)?
        .setup_tool(&context, runtime, &mut state.data.last_versions)
        .await?;

    // Update the cache with the timestamp
    state.data.last_version_check_time = time::now_millis();
    state.save()?;

    Ok(if installed_count > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
