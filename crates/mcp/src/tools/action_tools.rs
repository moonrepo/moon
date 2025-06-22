#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_action::Action;
use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_action_pipeline::ActionPipeline;
use moon_app_context::AppContext;
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, schema_utils::CallToolError},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

async fn run_pipeline(
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    action_graph_builder: ActionGraphBuilder<'_>,
) -> miette::Result<Vec<Action>> {
    let (action_context, action_graph) = action_graph_builder.build();
    let toolchain_registry = Arc::clone(&app_context.toolchain_registry);

    let mut pipeline = ActionPipeline::new(app_context, toolchain_registry, workspace_graph);
    pipeline.bail = true;
    pipeline.summarize = true;

    let results = pipeline
        .run_with_context(action_graph, action_context)
        .await?;

    Ok(results)
}

#[mcp_tool(name = "sync_workspace", description = "Sync the moon workspace.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SyncWorkspaceTool {}

impl SyncWorkspaceTool {
    pub async fn call_tool(
        &self,
        app_context: &Arc<AppContext>,
        workspace_graph: &Arc<WorkspaceGraph>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut action_graph = ActionGraphBuilder::new(
            Arc::clone(app_context),
            Arc::clone(workspace_graph),
            ActionGraphBuilderOptions {
                sync_workspace: true,
                ..Default::default()
            },
        )
        .map_err(map_miette_error)?;

        action_graph
            .sync_workspace()
            .await
            .map_err(map_miette_error)?;

        let actions = run_pipeline(
            Arc::clone(app_context),
            Arc::clone(workspace_graph),
            action_graph,
        )
        .await
        .map_err(map_miette_error)?;

        Ok(CallToolResult::text_content(
            serde_json::to_string_pretty(&SyncWorkspaceToolResponse {
                actions,
                synced: true,
            })
            .map_err(CallToolError::new)?,
            None,
        ))
    }
}

#[derive(Serialize)]
pub struct SyncWorkspaceToolResponse {
    pub actions: Vec<Action>,
    pub synced: bool,
}
