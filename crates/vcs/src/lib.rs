mod errors;
mod git;
mod loader;
mod svn;
mod vcs;

use moon_config::VcsManager;
use std::{collections::HashSet, path::Path};

pub use errors::VcsError;
pub use git::Git;
pub use loader::*;
pub use svn::Svn;
pub use vcs::*;

/// Detect the version control system being used and the current branch
pub async fn detect_vcs(
    dest_dir: &Path,
) -> Result<(VcsManager, String), Box<dyn std::error::Error>> {
    if dest_dir.join(".git").exists() {
        return Ok((
            VcsManager::Git,
            Git::new("master", dest_dir)?.get_local_branch().await?,
        ));
    }
    if dest_dir.join(".svn").exists() {
        return Ok((
            VcsManager::Svn,
            Svn::new("trunk", dest_dir).get_local_branch().await?,
        ));
    }
    Ok((VcsManager::Git, "master".into()))
}

/// Get all the touched/dirty files in the repository
pub async fn get_touched_files(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let (using_vcs, local_branch) = detect_vcs(path).await?;
    let vcs: Box<dyn Vcs> = match using_vcs {
        VcsManager::Git => Box::new(Git::new(&local_branch, path)?),
        VcsManager::Svn => Box::new(Svn::new(&local_branch, path)),
    };
    Ok(vcs.get_touched_files().await?.all)
}
