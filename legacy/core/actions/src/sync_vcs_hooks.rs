use moon_vcs_hooks::{HooksGenerator, HooksHash};
use moon_workspace::Workspace;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync_vcs_hooks(workspace: &Workspace, force: bool) -> miette::Result<bool> {
    let vcs_config = &workspace.config.vcs;
    let generator = HooksGenerator::new(&workspace.root, &workspace.vcs, vcs_config);

    // Force run the generator and bypass cache
    if force {
        generator.generate().await?;

        return Ok(true);
    }

    // Hash all the hook commands
    let mut hooks_hash = HooksHash::new(&vcs_config.manager);

    for (hook_name, commands) in &vcs_config.hooks {
        hooks_hash.add_hook(hook_name, commands);
    }

    // Only generate if the hash has changed
    workspace
        .cache_engine
        .execute_if_changed("vcsHooks.json", hooks_hash, || async {
            generator.generate().await
        })
        .await
}

#[instrument(skip_all)]
pub async fn unsync_vcs_hooks(workspace: &Workspace) -> miette::Result<()> {
    HooksGenerator::new(&workspace.root, &workspace.vcs, &workspace.config.vcs)
        .cleanup()
        .await?;

    Ok(())
}
