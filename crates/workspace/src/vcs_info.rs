use moon_vcs::BoxedVcs;
use std::sync::Arc;

#[derive(Default)]
pub struct VcsInfo {
    pub branch: String,
    pub repository: String,
    pub revision: String,
}

pub async fn load_vcs_info(vcs: Arc<BoxedVcs>) -> miette::Result<VcsInfo> {
    // Most tests don't have git initialized
    if !vcs.is_enabled() {
        return Ok(VcsInfo {
            branch: vcs.get_default_branch().await?,
            ..Default::default()
        });
    }

    let (branch, revision, repository) = tokio::join!(
        vcs.get_local_branch(),
        vcs.get_local_branch_revision(),
        vcs.get_repository_slug(),
    );

    Ok(VcsInfo {
        branch: branch?,
        // The repository may not have a remote
        repository: repository.unwrap_or_default(),
        revision: revision?,
    })
}
