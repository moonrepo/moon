use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_actions::operations::sync_config_schemas;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncConfigSchemasArgs {
    #[arg(long, help = "Bypass cache and force create schemas")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: MoonSession, args: SyncConfigSchemasArgs) -> AppResult {
    let context = session.get_app_context().await?;

    sync_config_schemas(&context, args.force).await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Generated configuration schemas to <path>{}</path>",
                        context.cache_engine.cache_dir.join("schemas").display()
                    )
                )
            }
        }
    })?;

    Ok(None)
}
