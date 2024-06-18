use super::should_skip_action_matching;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_logger::debug;
use moon_platform::{PlatformManager, Runtime};
use moon_utils::time;
use proto_core::UnresolvedVersionSpec;
use rustc_hash::FxHashMap;
use std::env;
use std::sync::Arc;
use tracing::instrument;

cache_item!(
    pub struct ToolState {
        pub last_versions: FxHashMap<String, UnresolvedVersionSpec>,
        pub last_version_check_time: u128,
    }
);

const LOG_TARGET: &str = "moon:action:setup-tool";

#[instrument(skip_all)]
pub async fn setup_tool(
    _action: &mut Action,
    context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "setup-tool");

    if runtime.platform.is_system() {
        return Ok(ActionStatus::Skipped);
    }

    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        runtime.label()
    );

    if should_skip_action_matching(
        "MOON_SKIP_SETUP_TOOL",
        format!("{}:{}", runtime, runtime.requirement),
    ) {
        debug!(
            target: LOG_TARGET,
            "Skipping setup tool action because MOON_SKIP_SETUP_TOOL is set",
        );

        return Ok(ActionStatus::Skipped);
    }

    let mut state = app_context
        .cache_engine
        .state
        .load_state::<ToolState>(format!("tool{}-{}.json", runtime, runtime.requirement))?;

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
