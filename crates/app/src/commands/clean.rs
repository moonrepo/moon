use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct CleanArgs {
    #[arg(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
    lifetime: String,
}

#[instrument(skip_all)]
pub async fn clean(session: MoonSession, args: CleanArgs) -> AppResult {
    let (files_deleted, bytes_saved) = session
        .get_cache_engine()?
        .clean_stale_cache(&args.lifetime, true)?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: format!(
                    "Deleted {files_deleted} files older than {} and saved {bytes_saved} bytes",
                    args.lifetime,
                ))
            }
        }
    })?;

    Ok(None)
}
