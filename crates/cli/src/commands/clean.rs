use crate::helpers::create_progress_bar;
use clap::Args;
use moon::load_workspace;
use starbase::AppResult;

#[derive(Args, Debug)]
pub struct CleanArgs {
    #[arg(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
    lifetime: String,
}

pub async fn clean(args: CleanArgs) -> AppResult {
    let workspace = load_workspace().await?;

    let done = create_progress_bar(format!("Cleaning stale cache older than {}", args.lifetime));

    let (files_deleted, bytes_saved) = workspace.cache2.clean_stale_cache(&args.lifetime)?;

    done(
        format!("Deleted {files_deleted} files and saved {bytes_saved} bytes"),
        true,
    );

    Ok(())
}
