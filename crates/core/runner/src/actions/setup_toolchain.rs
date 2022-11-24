use moon_action::{Action, ActionStatus};
use moon_logger::debug;
use moon_platform::Runtime;
use moon_runner_context::RunnerContext;
use moon_toolchain::tools::node::NodeTool;
use moon_utils::time;
use moon_workspace::{Workspace, WorkspaceError};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:setup-toolchain";

pub async fn setup_toolchain(
    _action: &mut Action,
    _context: Arc<RwLock<RunnerContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
) -> Result<ActionStatus, WorkspaceError> {
    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        runtime.label()
    );

    let mut workspace = workspace.write().await;
    let mut cache = workspace.cache.cache_tool_state(runtime).await?;
    let toolchain_paths = workspace.toolchain.get_paths();

    // Install and setup the specific tool + version in the toolchain!
    let installed = match runtime {
        Runtime::Node(version) => {
            let node = &mut workspace.toolchain.node;

            // The workspace version is pre-registered when the toolchain
            // is created, so any missing version must be an override at
            // the project-level. If so clone, and update defaults.
            if !node.has(&version.0) {
                node.register(
                    NodeTool::new(
                        &toolchain_paths,
                        &node.get()?.config.with_project_override(&version.0),
                    )?,
                    false,
                );
            }

            cache.last_version = version.0.clone();

            node.setup(&version.0).await?
        }
        _ => 0,
    };

    // Update the cache with the timestamp

    cache.last_version_check_time = time::now_millis();
    cache.save().await?;

    Ok(if installed > 0 {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
