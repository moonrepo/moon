use crate::helpers::{create_progress_bar, load_workspace};

pub struct CleanOptions {
    pub cache_lifetime: String,
}

pub async fn clean(options: CleanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;

    let done = create_progress_bar(format!(
        "Cleaning stale cache older than {}",
        options.cache_lifetime
    ));

    let (files_deleted, bytes_saved) = workspace
        .cache
        .clean_stale_cache(&options.cache_lifetime)
        .await?;

    done(
        format!(
            "Deleted {} files and saved {} bytes",
            files_deleted, bytes_saved
        ),
        true,
    );

    Ok(())
}
