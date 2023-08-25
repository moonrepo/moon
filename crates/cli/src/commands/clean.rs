use crate::helpers::create_progress_bar;
use clap::Args;
use moon_workspace::Workspace;
use starbase::AppResult;

#[derive(Args, Clone, Debug)]
pub struct CleanArgs {
    #[arg(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
    lifetime: String,
}

pub async fn clean(args: CleanArgs, workspace: Workspace) -> AppResult {
    let done = create_progress_bar(format!("Cleaning stale cache older than {}", args.lifetime));

    let (files_deleted, bytes_saved) = workspace.cache_engine.clean_stale_cache(&args.lifetime)?;

    done(
        format!("Deleted {files_deleted} files and saved {bytes_saved} bytes"),
        true,
    );

    Ok(())
}
