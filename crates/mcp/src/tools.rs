use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    tool_box,
};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "load_project",
    description = "Load a project with the provided `id`."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LoadProjectTool {
    id: String,
}

impl LoadProjectTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        Ok(CallToolResult::text_content("test".into(), None))
    }
}

tool_box!(MoonTools, [LoadProjectTool]);
