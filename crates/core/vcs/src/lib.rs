mod errors;
mod git;
mod loader;
mod svn;
mod vcs;

use moon_config::{VcsConfig, VcsManager};
use std::path::Path;

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
            Git::load(&VcsConfig::default(), dest_dir)?
                .get_local_branch()
                .await?,
        ));
    }

    if dest_dir.join(".svn").exists() {
        return Ok((
            VcsManager::Svn,
            Svn::load(&VcsConfig::default(), dest_dir)
                .get_local_branch()
                .await?,
        ));
    }

    Ok((VcsManager::Git, "master".into()))
}
