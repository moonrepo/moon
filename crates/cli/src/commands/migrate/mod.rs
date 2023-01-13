mod from_package_json;
mod from_turborepo;

pub use from_package_json::from_package_json;
pub use from_turborepo::from_turborepo;

use moon_workspace::Workspace;

pub async fn check_dirty_repo(workspace: &Workspace) -> Result<(), Box<dyn std::error::Error>> {
    if !workspace.vcs.get_touched_files().await?.all.is_empty() {
        return Err("Commit or stash your changes before running this command, or use the `--skipTouchedFilesCheck` flag to disable this check.".to_string())?;
    }
    Ok(())
}
