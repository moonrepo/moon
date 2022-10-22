mod from_package_json;

pub use from_package_json::from_package_json;

use moon_vcs::get_touched_files;
use std::env;

pub async fn is_repo_dirty() -> Result<bool, Box<dyn std::error::Error>> {
    let working_dir = env::current_dir().unwrap();
    let touched_files = get_touched_files(&working_dir).await?;
    Ok(!touched_files.is_empty())
}

pub async fn check_dirty_repo() -> Result<(), Box<dyn std::error::Error>> {
    if is_repo_dirty()
        .await
        .map_err(|_| "Unable to check if repo is dirty. Did you initialize your VCS?")?
    {
        Err("Commit or stash your changes before running this command, or use the `--skipTouchedFilesCheck` flag to disable this check.".to_string())?;
    }
    Ok(())
}
