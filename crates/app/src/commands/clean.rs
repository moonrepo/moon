use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use clap::Args;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct CleanArgs {
    #[arg(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
    lifetime: String,
}

#[instrument(skip_all)]
pub async fn clean(session: CliSession, args: CleanArgs) -> AppResult {
    let done = create_progress_bar(format!("Cleaning stale cache older than {}", args.lifetime));

    let (files_deleted, bytes_saved) = session
        .get_cache_engine()?
        .clean_stale_cache(&args.lifetime, true)?;

    done(
        format!("Deleted {files_deleted} files and saved {bytes_saved} bytes"),
        true,
    );

    Ok(())
}
