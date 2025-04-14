use crate::session::MoonSession;
use clap::Args;
use miette::IntoDiagnostic;
use moon_tool::{get_proto_env_vars, get_proto_paths, prepend_path_env_var};
use starbase::AppResult;
use tokio::process::Command;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(help = "The tool to query")]
    tool: String,
}

#[instrument(skip_all)]
pub async fn bin(session: MoonSession, args: BinArgs) -> AppResult {
    session.console.quiet();

    let result = Command::new("proto")
        .arg("bin")
        .arg(&args.tool)
        .env(
            "PATH",
            prepend_path_env_var(get_proto_paths(&session.proto_env)),
        )
        .envs(get_proto_env_vars())
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    if !result.success() {
        return Ok(Some(result.code().unwrap_or(1) as u8));
    }

    Ok(None)
}
