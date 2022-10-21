mod errors;
mod git;
mod loader;
mod svn;
mod vcs;

use std::{collections::HashSet, path::Path};
use strum::Display;

pub use errors::VcsError;
pub use git::Git;
pub use loader::*;
pub use svn::Svn;
pub use vcs::*;

#[derive(Debug, Display)]
pub enum SupportedVcs {
    #[strum(serialize = "git")]
    Git,
    #[strum(serialize = "svn")]
    Svn,
}

/// Detect the version control system being used and the current branch
pub async fn detect_vcs(
    dest_dir: &Path,
) -> Result<(SupportedVcs, String), Box<dyn std::error::Error>> {
    if dest_dir.join(".git").exists() {
        return Ok((
            SupportedVcs::Git,
            Git::new("master", dest_dir)?.get_local_branch().await?,
        ));
    }
    if dest_dir.join(".svn").exists() {
        return Ok((
            SupportedVcs::Svn,
            Svn::new("trunk", dest_dir).get_local_branch().await?,
        ));
    }
    Ok((SupportedVcs::Git, "master".into()))
}

/// Get all the touched/dirty files in the repository
pub async fn get_touched_files(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let (using_vcs, local_branch) = detect_vcs(path).await?;
    let vcs: Box<dyn Vcs> = match using_vcs {
        SupportedVcs::Git => Box::new(Git::new(&local_branch, path)?),
        SupportedVcs::Svn => Box::new(Svn::new(&local_branch, path)),
    };
    Ok(vcs.get_touched_files().await?.all)
}
