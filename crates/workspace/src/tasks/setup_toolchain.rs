use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

const SECOND: u128 = 1000;
const MINUTE: u128 = SECOND * 60;
const HOUR: u128 = MINUTE * 60;
const DAY: u128 = HOUR * 24;

pub async fn setup_toolchain(workspace: Arc<RwLock<Workspace>>) -> Result<(), WorkspaceError> {
    debug!(
        target: "moon:task-runner:setup-toolchain",
        "Setting up toolchain",
    );

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.cache_workspace_state().await?;
    let mut root_package = workspace.load_package_json().await?;

    // Only check the versions of some tools every 24 hours,
    // as checking every run has considerable overhead spawning all
    // the child processes. Revisit the threshold if need be.
    let now = cache.now_millis();
    let check_versions = cache.item.last_version_check_time == 0
        || (cache.item.last_version_check_time + DAY) <= now;

    workspace
        .toolchain
        .setup(&mut root_package, check_versions)
        .await?;

    // Update the cache with the timestamp
    if check_versions {
        cache.item.last_version_check_time = now;
        cache.save().await?;
    }

    Ok(())
}
