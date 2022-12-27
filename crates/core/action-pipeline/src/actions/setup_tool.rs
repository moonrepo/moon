use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_logger::debug;
use moon_node_tool::NodeTool;
use moon_platform::Runtime;
use moon_utils::time;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline:setup-tool";

pub async fn setup_tool(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
) -> Result<ActionStatus, PipelineError> {
    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    debug!(
        target: LOG_TARGET,
        "Setting up {} toolchain",
        runtime.label()
    );

    let mut workspace = workspace.write().await;
    let mut cache = workspace.cache.cache_tool_state(runtime)?;
    let toolchain_paths = workspace.toolchain.get_paths();

    // Install and setup the specific tool + version in the toolchain!
    // TODO remove when toolchain is gone
    let installed_count = match runtime {
        Runtime::Node(version) => {
            let node = &mut workspace.toolchain.node;

            // The workspace version is pre-registered when the toolchain
            // is created, so any missing version must be an override at
            // the project-level. If so clone, and update defaults.
            if !node.has(&version.0) {
                node.register(
                    Box::new(NodeTool::new(
                        &toolchain_paths,
                        &node.get::<NodeTool>().unwrap().config,
                        &version.0,
                    )?),
                    false,
                );
            }

            node.setup(&version.0, &mut cache.last_versions)
                .await
                .unwrap()
        }
        _ => 0,
    };

    workspace
        .platforms
        .get_mut(runtime)?
        .setup_tool(runtime.version(), &mut cache.last_versions)
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
