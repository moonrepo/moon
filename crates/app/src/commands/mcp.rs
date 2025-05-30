use crate::session::MoonSession;
use clap::Args;
use miette::IntoDiagnostic;
use moon_mcp::{SdkResult, run_mcp};
use moon_process::ProcessRegistry;
use starbase::AppResult;
use tokio::task::JoinHandle;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct McpArgs {}

#[instrument(skip_all)]
pub async fn mcp(session: MoonSession, _args: McpArgs) -> AppResult {
    let app_context = session.get_app_context().await?;
    let workspace_graph = session.get_workspace_graph().await?;

    let handle_server: JoinHandle<SdkResult<()>> =
        tokio::spawn(async move { run_mcp(app_context, workspace_graph).await });

    let handle: JoinHandle<miette::Result<()>> = tokio::spawn(async move {
        let mut listener = ProcessRegistry::instance().receive_signal();

        if listener.recv().await.is_ok() {
            handle_server.abort();
        }

        Ok(())
    });

    handle.await.into_diagnostic()??;

    Ok(None)
}
