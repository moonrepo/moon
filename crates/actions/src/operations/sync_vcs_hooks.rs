use moon_app_context::AppContext;
use moon_vcs_hooks::{HooksGenerator, HooksHash};
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync_vcs_hooks(app_context: &AppContext, force: bool) -> miette::Result<bool> {
    let vcs_config = &app_context.workspace_config.vcs;
    let generator = HooksGenerator::new(&app_context.vcs, vcs_config, &app_context.workspace_root);

    // Force run the generator and bypass cache
    if force {
        generator.generate().await?;

        return Ok(true);
    }

    // Hash all the hook commands
    let mut hooks_hash = HooksHash::new(&vcs_config.manager);

    hooks_hash.files_exist = generator
        .get_internal_hook_paths()
        .into_iter()
        .all(|file| file.exists());

    for (hook_name, commands) in &vcs_config.hooks {
        hooks_hash.add_hook(hook_name, commands);
    }

    // Only generate if the hash has changed
    app_context
        .cache_engine
        .execute_if_changed("vcsHooks.json", hooks_hash, || async {
            generator.generate().await
        })
        .await
}

#[instrument(skip_all)]
pub async fn unsync_vcs_hooks(app_context: &AppContext) -> miette::Result<()> {
    HooksGenerator::new(
        &app_context.vcs,
        &app_context.workspace_config.vcs,
        &app_context.workspace_root,
    )
    .cleanup()
    .await?;

    Ok(())
}
