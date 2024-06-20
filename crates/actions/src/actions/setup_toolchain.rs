use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SetupToolchainNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_common::color;
use moon_config::UnresolvedVersionSpec;
use moon_platform::PlatformManager;
use moon_time::now_millis;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

cache_item!(
    pub struct ToolCacheState {
        pub last_versions: FxHashMap<String, UnresolvedVersionSpec>,
        pub last_version_check_time: u128,
    }
);

#[instrument(skip(_action, action_context, app_context))]
pub async fn setup_toolchain(
    _action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    node: &SetupToolchainNode,
) -> miette::Result<ActionStatus> {
    let log_label = node.runtime.label();

    if let Some(value) = should_skip_action_matching(
        "MOON_SKIP_SETUP_TOOLCHAIN",
        format!("{}:{}", &node.runtime, &node.runtime.requirement),
    ) {
        debug!(
            env = value,
            "Skipping {} toolchain setup because {} is set",
            log_label,
            color::symbol("MOON_SKIP_SETUP_TOOLCHAIN")
        );

        return Ok(ActionStatus::Skipped);
    }

    debug!("Setting up {} toolchain", log_label);

    let mut state = app_context
        .cache_engine
        .state
        .load_state::<ToolCacheState>(format!(
            "tool{}-{}.json",
            &node.runtime, &node.runtime.requirement
        ))?;

    // Install and setup the specific tool + version in the toolchain!
    let installed_count = PlatformManager::write()
        .get_mut(&node.runtime)?
        .setup_tool(
            &action_context,
            &node.runtime,
            &mut state.data.last_versions,
        )
        .await?;

    // Update the cache with the timestamp
    state.data.last_version_check_time = now_millis();
    state.save()?;

    Ok(if installed_count > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
