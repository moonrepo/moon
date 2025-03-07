use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use clap::Args;
use moon_actions::operations::sync_config_schemas;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncConfigSchemasArgs {
    #[arg(long, help = "Bypass cache and force create schemas")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: CliSession, args: SyncConfigSchemasArgs) -> AppResult {
    let done = create_progress_bar("Generating configuration schemas...");

    let context = session.get_app_context().await?;

    sync_config_schemas(&context, args.force).await?;

    done("Successfully generated schemas", true);

    Ok(None)
}
