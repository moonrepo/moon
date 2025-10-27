use crate::session::MoonSession;
use clap::Args;
use moon_env_var::GlobalEnvBag;
use moon_process::Command;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(help = "The toolchain to query")]
    toolchain: String,
}

#[instrument(skip(session))]
pub async fn bin(session: MoonSession, args: BinArgs) -> AppResult {
    session.console.quiet();

    let mut command = Command::new("proto");
    let toolchain_registry = session.get_toolchain_registry().await?;

    toolchain_registry
        .augment_command(
            &mut command,
            GlobalEnvBag::instance(),
            toolchain_registry.create_command_augments(None),
        )
        .await?;

    let result = command
        .arg("bin")
        .arg(&args.toolchain)
        .exec_stream_output()
        .await?;

    if !result.success() {
        return Ok(Some(result.code().unwrap_or(1) as u8));
    }

    Ok(None)
}
