use moon_action::{Action, ActionContext, ActionStatus};
use moon_config::NodeConfig;
use moon_contract::SupportedPlatform;
use moon_logger::debug;
use moon_toolchain::tools::node::NodeTool;
use moon_workspace::{Workspace, WorkspaceError};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:setup-toolchain";
const HOUR: u128 = 3600;

pub async fn setup_toolchain(
    _action: &mut Action,
    _context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    platform: &SupportedPlatform,
) -> Result<ActionStatus, WorkspaceError> {
    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        platform.label()
    );

    let mut workspace = workspace.write().await;
    let mut cache = workspace.cache.cache_workspace_state().await?;
    let toolchain_paths = workspace.toolchain.get_paths();

    // Only check the versions every 12 hours, as checking every
    // run has considerable overhead spawning all the child processes.
    // Revisit the threshold if need be.
    let now = cache.now_millis();
    let check_versions = cache.item.last_version_check_time == 0
        || (cache.item.last_version_check_time + HOUR * 12) <= now;

    // Install and setup the specific tool + version in the toolchain!
    let installed = match platform {
        SupportedPlatform::Node(version) => {
            let node = &mut workspace.toolchain.node;

            // The workspace version is pre-registered when the toolchain
            // is created, so any missing version must be an override at
            // the project-level. If so, use config defaults.
            if !node.has(version) {
                node.register(
                    NodeTool::new(
                        &toolchain_paths,
                        &NodeConfig {
                            version: version.to_owned(),
                            ..NodeConfig::default()
                        },
                    )?,
                    false,
                );
            }

            node.setup(version, check_versions).await?
        }
        _ => 0,
    };

    // Update the cache with the timestamp
    if check_versions {
        cache.item.last_version_check_time = now;
        cache.save().await?;
    }

    Ok(if installed > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
