#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_app_context::AppContext;
use moon_common::is_ci;
use moon_common::path::WorkspaceRelativePathBuf;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "get_touched_files",
    description = "Get touched files between the current head and base."
)]
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(default)]
pub struct GetTouchedFiles {
    pub base: Option<String>,
    pub head: Option<String>,
    pub remote: Option<bool>,
}

impl GetTouchedFiles {
    pub async fn call_tool(
        &self,
        app_context: &AppContext,
    ) -> Result<CallToolResult, CallToolError> {
        let vcs = &app_context.vcs;
        let default_branch = vcs.get_default_branch().await.map_err(map_miette_error)?;
        let current_branch = vcs.get_local_branch().await.map_err(map_miette_error)?;

        let base = self.base.as_deref().unwrap_or(&default_branch);
        let head = self.head.as_deref().unwrap_or("HEAD");
        let remote = self.remote.unwrap_or(is_ci());

        let check_against_previous =
            self.base.is_none() && self.head.is_none() && vcs.is_default_branch(&current_branch);

        let touched_files = if !remote {
            vcs.get_touched_files().await.map_err(map_miette_error)?
        } else if check_against_previous {
            vcs.get_touched_files_against_previous_revision(&default_branch)
                .await
                .map_err(map_miette_error)?
        } else {
            vcs.get_touched_files_between_revisions(base, head)
                .await
                .map_err(map_miette_error)?
        };

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetTouchedFilesResponse {
                files: touched_files.all().into_iter().cloned().collect(),
            })
            .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
pub struct GetTouchedFilesResponse {
    pub files: Vec<WorkspaceRelativePathBuf>,
}
