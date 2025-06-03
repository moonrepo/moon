use crate::session::MoonSession;
use clap::Args;
// use iocraft::prelude::element;
// use moon_actions::operations::sync_config_schemas;
// use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainInfoArgs {
    #[arg(help = "ID of the toolchain to inspect")]
    id: String,
}

#[instrument(skip_all)]
pub async fn info(_session: MoonSession, _args: ToolchainInfoArgs) -> AppResult {
    // let context = session.get_app_context().await?;
    // let toolchain_registry = session.get_toolchain_registry().await?;

    // sync_config_schemas(&context, args.force).await?;

    // session.console.render(element! {
    //     Container {
    //         Notice(variant: Variant::Success) {
    //             StyledText(
    //                 content: format!(
    //                     "Generated configuration schemas to <path>{}</path>",
    //                     context.cache_engine.cache_dir.join("schemas").display()
    //                 )
    //             )
    //         }
    //     }
    // })?;

    Ok(None)
}
