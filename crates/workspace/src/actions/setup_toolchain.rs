use crate::action::ActionStatus;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

const SECOND: u128 = 1000;
const MINUTE: u128 = SECOND * 60;
const HOUR: u128 = MINUTE * 60;

pub async fn setup_toolchain(
    workspace: Arc<RwLock<Workspace>>,
) -> Result<ActionStatus, WorkspaceError> {
    debug!(
        target: "moon:action:setup-toolchain",
        "Setting up toolchain",
    );

    let mut workspace = workspace.write().await;
    let mut cache = workspace.cache.cache_workspace_state().await?;

    // Only check the versions of some tools every 12 hours,
    // as checking every run has considerable overhead spawning all
    // the child processes. Revisit the threshold if need be.
    let now = cache.now_millis();
    let check_versions = cache.item.last_version_check_time == 0
        || (cache.item.last_version_check_time + HOUR * 12) <= now;

    // Install all tools
    let installed_tools = workspace.toolchain.setup(check_versions).await?;

    // Update the cache with the timestamp
    if check_versions {
        cache.item.last_version_check_time = now;
        cache.save().await?;
    }

    Ok(if installed_tools {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
