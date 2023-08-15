use moon_cache_item::CommonState;
use moon_vcs_hooks::{HooksGenerator, HooksHash};
use moon_workspace::Workspace;

pub async fn sync_vcs_hooks(workspace: &Workspace, force: bool) -> miette::Result<()> {
    let vcs_config = &workspace.config.vcs;
    let cache_engine = &workspace.cache2;

    // Hash all the hook commands
    let mut hooks_hash = HooksHash::new(&vcs_config.manager);

    for (hook_name, commands) in &vcs_config.hooks {
        hooks_hash.add_hook(hook_name, commands);
    }

    // Check the cache before creating the files
    let mut state = cache_engine.cache_state::<CommonState>("vcsHooks.json")?;

    let hash = cache_engine
        .hash
        .save_manifest_without_hasher("VCS hooks", &hooks_hash)?;

    if force || hash != state.data.last_hash {
        HooksGenerator::new(&workspace.root, &workspace.vcs, vcs_config)
            .generate()
            .await?;

        state.data.last_hash = hash;
        state.save()?;
    }

    Ok(())
}

pub async fn unsync_vcs_hooks(workspace: &Workspace) -> miette::Result<()> {
    HooksGenerator::new(&workspace.root, &workspace.vcs, &workspace.config.vcs)
        .cleanup()
        .await?;

    Ok(())
}
