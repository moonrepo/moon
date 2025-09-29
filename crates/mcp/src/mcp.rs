use crate::tools::action_tools::{SyncProjectsTool, SyncWorkspaceTool};
use crate::tools::project_tools::{GetProjectTool, GetProjectsTool};
use crate::tools::task_tools::{GetTaskTool, GetTasksTool};
use crate::tools::vcs_tools::GetTouchedFiles;
use async_trait::async_trait;
use moon_app_context::AppContext;
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::mcp_server::{ServerHandler, ServerRuntime, server_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, Implementation, InitializeResult, LATEST_PROTOCOL_VERSION,
    ListToolsRequest, ListToolsResult, RpcError, ServerCapabilities, ServerCapabilitiesTools,
    schema_utils::CallToolError,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions, tool_box};
use std::env;
use std::sync::Arc;

pub struct MoonMcpHandler {
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
}

#[async_trait]
impl ServerHandler for MoonMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: MoonTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params: MoonTools =
            MoonTools::try_from(request.params).map_err(CallToolError::new)?;

        match tool_params {
            MoonTools::GetProjectTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetProjectsTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTaskTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTasksTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTouchedFiles(inner) => inner.call_tool(&self.app_context).await,
            MoonTools::SyncProjectsTool(inner) => {
                inner
                    .call_tool(&self.app_context, &self.workspace_graph)
                    .await
            }
            MoonTools::SyncWorkspaceTool(inner) => {
                inner
                    .call_tool(&self.app_context, &self.workspace_graph)
                    .await
            }
        }
    }
}

pub async fn run_mcp(
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
) -> SdkResult<()> {
    // STEP 1: Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "moon_mcp_server".to_string(),
            version: env::var("MOON_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            title: Some("moon MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: None,
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // STEP 2: Create an std transport with default options
    let transport = StdioTransport::new(TransportOptions::default())?;

    // STEP 3: Instantiate our custom handler for handling MCP messages
    let handler = MoonMcpHandler {
        app_context,
        workspace_graph,
    };

    // STEP 4: Create the MCP runtime
    let server: Arc<ServerRuntime> =
        server_runtime::create_server(server_details, transport, handler);

    // STEP 5: Start the server
    server.start().await
}

tool_box!(
    MoonTools,
    [
        GetProjectTool,
        GetProjectsTool,
        GetTaskTool,
        GetTasksTool,
        GetTouchedFiles,
        SyncProjectsTool,
        SyncWorkspaceTool
    ]
);
