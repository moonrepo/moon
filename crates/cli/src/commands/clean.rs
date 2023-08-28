use crate::helpers::create_progress_bar;
use clap::Args;
use moon_workspace::Workspace;
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct CleanArgs {
    #[arg(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
    lifetime: String,
}

#[system]
pub async fn clean(args: ArgsRef<CleanArgs>, workspace: ResourceRef<Workspace>) {
    let done = create_progress_bar(format!("Cleaning stale cache older than {}", args.lifetime));

    let (files_deleted, bytes_saved) = workspace.cache_engine.clean_stale_cache(&args.lifetime)?;

    done(
        format!("Deleted {files_deleted} files and saved {bytes_saved} bytes"),
        true,
    );
}
