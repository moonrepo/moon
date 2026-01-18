#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_project::{Project, ProjectFragment};
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "get_project",
    title = "Get project",
    description = "Get a moon project and its tasks by `id`."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetProjectTool {
    pub id: String,

    #[serde(default)]
    pub include_dependencies: bool,
}

impl GetProjectTool {
    pub fn call_tool(
        &self,
        workspace_graph: &WorkspaceGraph,
    ) -> Result<CallToolResult, CallToolError> {
        let project = workspace_graph
            .get_project_with_tasks(&self.id)
            .map_err(map_miette_error)?;
        let mut project_dependencies = vec![];

        if self.include_dependencies {
            for dep in &project.dependencies {
                project_dependencies.push(
                    workspace_graph
                        .get_project_with_tasks(&dep.id)
                        .map_err(map_miette_error)?,
                );
            }
        }

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetProjectResponse {
                project,
                project_dependencies,
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectResponse {
    pub project: Project,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_dependencies: Vec<Project>,
}

#[mcp_tool(
    name = "get_projects",
    title = "Get projects",
    description = "Get all moon projects."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetProjectsTool {}

impl GetProjectsTool {
    pub fn call_tool(
        &self,
        workspace_graph: &WorkspaceGraph,
    ) -> Result<CallToolResult, CallToolError> {
        let mut projects = workspace_graph.get_projects().map_err(map_miette_error)?;

        projects.sort_by(|a, d| a.id.cmp(&d.id));

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetProjectsResponse {
                projects: projects
                    .into_iter()
                    .map(|proj| proj.to_fragment())
                    .collect(),
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
pub struct GetProjectsResponse {
    pub projects: Vec<ProjectFragment>,
}
