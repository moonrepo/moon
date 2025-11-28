use crate::session::MoonSession;
use clap::Args;
use moon_common::Id;
use moon_env_var::GlobalEnvBag;
use moon_process_augment::CommandBuilder;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "The toolchain to query")]
    toolchain: Id,
}

#[instrument(skip(session))]
pub async fn bin(session: MoonSession, args: BinArgs) -> AppResult {
    session.console.quiet();

    let app_context = session.get_app_context().await?;

    let mut builder = CommandBuilder::new(&app_context, GlobalEnvBag::instance(), "proto");
    builder.inherit_from_plugins(None, None).await?;

    let result = builder
        .build()
        .arg("bin")
        .arg(&args.toolchain)
        .exec_stream_output()
        .await?;

    if !result.success() {
        return Ok(Some(result.code().unwrap_or(1) as u8));
    }

    Ok(None)
}
