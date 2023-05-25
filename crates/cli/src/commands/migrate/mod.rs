mod from_package_json;
mod from_turborepo;

pub use from_package_json::from_package_json;
pub use from_turborepo::*;

use miette::miette;
use moon_workspace::Workspace;
use starbase::AppResult;

pub async fn check_dirty_repo(workspace: &Workspace) -> AppResult {
    if !workspace.vcs.get_touched_files().await?.all.is_empty() {
        return Err(
            miette!("Commit or stash your changes before running this command, or use the `--skipTouchedFilesCheck` flag to disable this check.")
        );
    }

    Ok(())
}
