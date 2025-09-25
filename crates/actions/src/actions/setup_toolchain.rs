use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SetupToolchainLegacyNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_common::color;
use moon_common::path::encode_component;
use moon_config::UnresolvedVersionSpec;
use moon_platform::PlatformManager;
use moon_time::now_millis;
use rustc_hash::FxHashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

// Avoid the same tool running in parallel causing issues,
// so use a global lock keyed by tool ID.
static LOCKS: OnceLock<scc::HashMap<String, Mutex<()>>> = OnceLock::new();

cache_item!(
    pub struct ToolCacheState {
        pub last_versions: FxHashMap<String, UnresolvedVersionSpec>,
        pub last_version_check_time: u128,
        pub requirement: Option<UnresolvedVersionSpec>,
    }
);

#[instrument(skip(_action, action_context, app_context))]
pub async fn setup_toolchain(
    _action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    node: &SetupToolchainLegacyNode,
) -> miette::Result<ActionStatus> {
    let log_label = node.runtime.label();
    let cache_engine = &app_context.cache_engine;
    let action_key = node.runtime.target();

    let _lock = app_context
        .cache_engine
        .create_lock(format!("setupToolchain-{action_key}"))?;

    if let Some(value) = should_skip_action_matching("MOON_SKIP_SETUP_TOOLCHAIN", &action_key) {
        debug!(
            env = value,
            "Skipping {} toolchain setup because {} is set",
            log_label,
            color::symbol("MOON_SKIP_SETUP_TOOLCHAIN")
        );

        return Ok(ActionStatus::Skipped);
    }

    debug!("Setting up {} toolchain", log_label);

    let mut state = cache_engine.state.load_state::<ToolCacheState>(format!(
        "setupToolchain-{}.json",
        encode_component(action_key),
    ))?;

    // Acquire a lock for the toolchain ID
    let locks = LOCKS.get_or_init(scc::HashMap::default);
    let entry = locks.entry(node.runtime.id()).or_default();
    let _lock = entry.lock().await;

    // Install and setup the specific tool + version in the toolchain!
    let installed_count = PlatformManager::write()
        .get_by_toolchain_mut(&node.runtime.toolchain)?
        .setup_tool(
            &action_context,
            &node.runtime,
            &mut state.data.last_versions,
        )
        .await?;

    // Update the cache with the timestamp
    state.data.last_version_check_time = now_millis();
    state.data.requirement = node.runtime.requirement.to_spec();
    state.save()?;

    Ok(if installed_count > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
