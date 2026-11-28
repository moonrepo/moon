use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_mcp::run_mcp;
use moon_process::ProcessRegistry;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct McpArgs {}

#[instrument(skip(session))]
pub async fn mcp(session: MoonSession, _args: McpArgs) -> SessionResult {
    let app_context = session.get_app_context().await?;
    let workspace_graph = session.get_workspace_graph().await?;
    let mut listener = ProcessRegistry::instance().receive_signal();

    tokio::select! {
        // The server stops once stdin is closed, which is how a client
        // shuts down a stdio MCP server, even one that was killed
        result = run_mcp(app_context, workspace_graph) => {
            result.map_err(|error| miette::miette!("{error}"))?;

            debug!("MCP client closed stdin, shutting down the MCP server");
        }

        // stdin is read on a blocking thread that can't be cancelled, and the
        // runtime waits for that thread when shutting down, so exit immediately
        _ = listener.recv() => {
            std::process::exit(0);
        }
    }

    Ok(None)
}
