use crate::helpers::create_progress_bar;
use moon_workspace::Workspace;

pub struct CleanOptions {
    pub cache_liftime: String,
}

pub async fn clean(options: CleanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    let done = create_progress_bar(format!(
        "Cleaning stale cache older than {}",
        options.cache_liftime
    ));

    let (files_deleted, bytes_saved) = workspace
        .cache
        .clean_stale_cache(&options.cache_liftime)
        .await?;

    done(format!(
        "Deleted {} files and saved {} bytes",
        files_deleted, bytes_saved
    ));

    Ok(())
}
