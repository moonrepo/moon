use crate::helpers::{create_progress_bar, AnyError};
use moon::load_workspace;

pub struct CleanOptions {
    pub cache_lifetime: String,
}

pub async fn clean(options: CleanOptions) -> Result<(), AnyError> {
    let workspace = load_workspace().await?;

    let done = create_progress_bar(format!(
        "Cleaning stale cache older than {}",
        options.cache_lifetime
    ));

    let (files_deleted, bytes_saved) =
        workspace.cache.clean_stale_cache(&options.cache_lifetime)?;

    done(
        format!("Deleted {files_deleted} files and saved {bytes_saved} bytes"),
        true,
    );

    Ok(())
}
