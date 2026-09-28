use crate::tools::action_tools::{SyncProjectsTool, SyncWorkspaceTool};
use crate::tools::codegen_tools::{GenerateTool, GetTemplateTool, GetTemplatesTool};
use crate::tools::project_tools::{GetProjectTool, GetProjectsTool};
use crate::tools::task_tools::{GetTaskTool, GetTasksTool};
use crate::tools::vcs_tools::GetChangedFilesTool;
use async_trait::async_trait;
use moon_app_context::AppContext;
use moon_workspace_graph::WorkspaceGraph;
use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::mcp_server::{McpServerOptions, ServerHandler, server_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, Implementation, ListToolsResult, ListToolsResultCacheScope,
    PaginatedRequestParams, RpcError, ServerCapabilities, ServerCapabilitiesTools, ServerResult,
    schema_utils::CallToolError,
};
use rust_mcp_sdk::{
    McpServer, RequestContext, ServerDetails, StdioTransport, ToMcpServerHandler, TransportOptions,
    tool_box,
};
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
        _request: Option<PaginatedRequestParams>,
        _context: &RequestContext,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            cache_scope: ListToolsResultCacheScope::Private,
            meta: None,
            next_cursor: None,
            result_type: "complete".into(),
            tools: MoonTools::tools(),
            ttl_ms: 0,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequestParams,
        _context: &RequestContext,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ServerResult, CallToolError> {
        let tool_params: MoonTools = MoonTools::try_from(request).map_err(CallToolError::new)?;

        let result = match tool_params {
            MoonTools::GenerateTool(inner) => inner.call_tool(&self.app_context).await,
            MoonTools::GetChangedFilesTool(inner) => inner.call_tool(&self.app_context).await,
            MoonTools::GetProjectTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetProjectsTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTaskTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTasksTool(inner) => inner.call_tool(&self.workspace_graph),
            MoonTools::GetTemplateTool(inner) => inner.call_tool(&self.app_context).await,
            MoonTools::GetTemplatesTool(inner) => inner.call_tool(&self.app_context).await,
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
        }?;

        Ok(ServerResult::from(result))
    }
}

pub async fn run_mcp(
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
) -> SdkResult<()> {
    // STEP 1: Define server details and capabilities
    let server_details = ServerDetails {
        server_info: Implementation {
            name: "moon_mcp_server".to_string(),
            version: env::var("MOON_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            title: Some("moon MCP Server".to_string()),
            website_url: Some("https://moonrepo.dev".into()),
            description: None,
            icons: vec![],
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        instructions: None,
        meta: None,
    };

    // STEP 2: Create an std transport with default options
    let transport = StdioTransport::new(TransportOptions::default())?;

    // STEP 3: Instantiate our custom handler for handling MCP messages
    let handler = MoonMcpHandler {
        app_context,
        workspace_graph,
    };

    // STEP 4: Create the MCP runtime
    let server = server_runtime::create_server(McpServerOptions {
        transport,
        handler: handler.to_mcp_server_handler(),
        server_details,
        message_observer: None,
    });

    // STEP 5: Start the server
    server.start().await
}

tool_box!(
    MoonTools,
    [
        GenerateTool,
        GetChangedFilesTool,
        GetProjectTool,
        GetProjectsTool,
        GetTaskTool,
        GetTasksTool,
        GetTemplateTool,
        GetTemplatesTool,
        SyncProjectsTool,
        SyncWorkspaceTool
    ]
);
