use moon_hash::HashEngine;
use moon_vcs_hooks::{HooksGenerator, HooksHash};
use moon_workspace::Workspace;

pub async fn sync_vcs_hooks(workspace: &Workspace) -> miette::Result<()> {
    let vcs_config = &workspace.config.vcs;

    let hash_engine = HashEngine::new(&workspace.cache.dir);
    let mut hasher = hash_engine.create_hasher("VCS hooks");

    // Hash all the hook commands
    let mut hooks_hash = HooksHash::new(&vcs_config.manager);

    for (hook_name, commands) in &vcs_config.hooks {
        hooks_hash.add_hook(hook_name, commands);
    }

    hasher.hash_content(&hooks_hash);

    // Check the cache before creating the files
    let mut cache = workspace.cache.cache_vcs_hooks_state()?;

    if hasher.generate_hash()? != cache.last_hash {
        HooksGenerator::new(&workspace.root, &workspace.vcs, vcs_config)
            .generate()
            .await?;

        cache.last_hash = hash_engine.save_manifest(hasher)?;
        cache.save()?;
    }

    Ok(())
}
