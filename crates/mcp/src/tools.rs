#![allow(clippy::disallowed_types)]

use moon_common::cacheable;
use moon_project::Project;
use moon_task::{Target, Task};
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, schema_utils::CallToolError},
    tool_box,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
struct ReportError(pub miette::Report);

impl Error for ReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn map_miette_error(report: miette::Report) -> CallToolError {
    CallToolError::new(ReportError(report))
}

#[mcp_tool(
    name = "get_project",
    description = "Get a project and its tasks by `id`."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetProjectTool {
    id: String,

    #[serde(default)]
    include_dependencies: bool,
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

        Ok(CallToolResult::text_content(
            serde_json::to_string_pretty(&GetProjectResponse {
                project,
                project_dependencies,
            })
            .map_err(CallToolError::new)?,
            None,
        ))
    }
}

cacheable!(
    pub struct GetProjectResponse {
        project: Project,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        project_dependencies: Vec<Project>,
    }
);

#[mcp_tool(name = "get_projects", description = "Get all projects.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetProjectsTool {
    #[serde(default)]
    include_tasks: bool,
}

impl GetProjectsTool {
    pub fn call_tool(
        &self,
        workspace_graph: &WorkspaceGraph,
    ) -> Result<CallToolResult, CallToolError> {
        let mut projects = workspace_graph.get_projects().map_err(map_miette_error)?;

        projects.sort_by(|a, d| a.id.cmp(&d.id));

        if self.include_tasks {
            let mut new_projects = vec![];

            for project in projects {
                new_projects.push(Arc::new(
                    workspace_graph
                        .get_project_with_tasks(&project.id)
                        .map_err(map_miette_error)?,
                ));
            }

            projects = new_projects;
        }

        Ok(CallToolResult::text_content(
            serde_json::to_string_pretty(&GetProjectsResponse { projects })
                .map_err(CallToolError::new)?,
            None,
        ))
    }
}

cacheable!(
    pub struct GetProjectsResponse {
        projects: Vec<Arc<Project>>,
    }
);

#[mcp_tool(name = "get_task", description = "Get a task by `target`.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetTaskTool {
    target: String,

    #[serde(default)]
    include_dependencies: bool,
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

        Ok(CallToolResult::text_content(
            serde_json::to_string_pretty(&GetTaskResponse {
                task,
                task_dependencies,
            })
            .map_err(CallToolError::new)?,
            None,
        ))
    }
}

cacheable!(
    pub struct GetTaskResponse {
        task: Arc<Task>,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        task_dependencies: Vec<Arc<Task>>,
    }
);

#[mcp_tool(name = "get_tasks", description = "Get all tasks.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetTasksTool {
    #[serde(default)]
    include_internal: bool,
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

        Ok(CallToolResult::text_content(
            serde_json::to_string_pretty(&GetTasksResponse { tasks })
                .map_err(CallToolError::new)?,
            None,
        ))
    }
}

cacheable!(
    pub struct GetTasksResponse {
        tasks: Vec<Arc<Task>>,
    }
);

tool_box!(
    MoonTools,
    [GetProjectTool, GetProjectsTool, GetTaskTool, GetTasksTool]
);
