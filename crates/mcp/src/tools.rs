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
        project_dependencies: Vec<Project>,
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
        let target = Target::parse(&self.target).map_err(map_miette_error)?;
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
        task_dependencies: Vec<Arc<Task>>,
    }
);

tool_box!(MoonTools, [GetProjectTool, GetTaskTool]);
