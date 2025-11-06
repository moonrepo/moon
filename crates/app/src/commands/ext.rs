use crate::session::MoonSession;
use clap::Args;
use moon_common::Id;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ExtArgs {
    #[arg(required = true, help = "Extension ID to execute")]
    id: Id,

    // Passthrough args (after --)
    #[arg(last = true, help = "Arguments to pass through to the extension")]
    passthrough: Vec<String>,
}

#[instrument(skip(session))]
pub async fn ext(session: MoonSession, args: ExtArgs) -> AppResult {
    let extension_registry = session.get_extension_registry().await?;

    extension_registry
        .load(&args.id)
        .await?
        .execute(args.passthrough, extension_registry.create_context())
        .await?;

    Ok(None)
}
