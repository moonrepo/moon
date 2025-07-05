#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_task::{Target, Task};
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[mcp_tool(name = "get_task", description = "Get a moon task by `target`.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetTaskTool {
    pub target: String,

    #[serde(default)]
    pub include_dependencies: bool,
}

impl GetTaskTool {
    pub fn call_tool(
        &self,
        workspace_graph: &WorkspaceGraph,
    ) -> Result<CallToolResult, CallToolError> {
        let target = Target::parse_strict(&self.target).map_err(map_miette_error)?;
        let task = workspace_graph
            .get_task(&target)
            .map_err(map_miette_error)?;
        let mut task_dependencies = vec![];

        if self.include_dependencies {
            for dep in &task.deps {
                task_dependencies.push(
                    workspace_graph
                        .get_task(&dep.target)
                        .map_err(map_miette_error)?,
                );
            }
        }

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetTaskResponse {
                task,
                task_dependencies,
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskResponse {
    pub task: Arc<Task>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub task_dependencies: Vec<Arc<Task>>,
}

#[mcp_tool(name = "get_tasks", description = "Get all moon tasks.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetTasksTool {
    #[serde(default)]
    pub include_internal: bool,
}

impl GetTasksTool {
    pub fn call_tool(
        &self,
        workspace_graph: &WorkspaceGraph,
    ) -> Result<CallToolResult, CallToolError> {
        let mut tasks = if self.include_internal {
            workspace_graph
                .get_tasks_with_internal()
                .map_err(map_miette_error)?
        } else {
            workspace_graph.get_tasks().map_err(map_miette_error)?
        };

        tasks.sort_by(|a, d| a.target.cmp(&d.target));

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetTasksResponse { tasks })
                .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
pub struct GetTasksResponse {
    pub tasks: Vec<Arc<Task>>,
}
