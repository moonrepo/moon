#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_action::Action;
use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_action_pipeline::ActionPipeline;
use moon_app_context::AppContext;
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

async fn run_pipeline(
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    action_graph_builder: ActionGraphBuilder<'_>,
) -> miette::Result<Vec<Action>> {
    let (action_context, action_graph) = action_graph_builder.build();

    let mut pipeline = ActionPipeline::new(app_context, workspace_graph);
    pipeline.bail = true;
    pipeline.quiet = true;

    let results = pipeline
        .run_with_context(action_graph, action_context)
        .await?;

    Ok(results)
}

#[derive(Serialize)]
pub struct SyncResponse {
    pub actions: Vec<Action>,
    pub synced: bool,
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

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&SyncResponse {
                actions,
                synced: true,
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[mcp_tool(
    name = "sync_projects",
    description = "Sync one, many, or all moon projects by `id`."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SyncProjectsTool {
    pub ids: Vec<String>,
}

impl SyncProjectsTool {
    pub async fn call_tool(
        &self,
        app_context: &Arc<AppContext>,
        workspace_graph: &Arc<WorkspaceGraph>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut action_graph = ActionGraphBuilder::new(
            Arc::clone(app_context),
            Arc::clone(workspace_graph),
            ActionGraphBuilderOptions {
                // Called by sync_project
                sync_workspace: false,
                ..Default::default()
            },
        )
        .map_err(map_miette_error)?;

        if self.ids.is_empty() {
            let projects = workspace_graph.get_projects().map_err(map_miette_error)?;

            for project in projects {
                action_graph
                    .sync_project(&project)
                    .await
                    .map_err(map_miette_error)?;
            }
        } else {
            for id in &self.ids {
                let project = workspace_graph.get_project(id).map_err(map_miette_error)?;

                action_graph
                    .sync_project(&project)
                    .await
                    .map_err(map_miette_error)?;
            }
        }

        let actions = run_pipeline(
            Arc::clone(app_context),
            Arc::clone(workspace_graph),
            action_graph,
        )
        .await
        .map_err(map_miette_error)?;

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&SyncResponse {
                actions,
                synced: true,
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}
