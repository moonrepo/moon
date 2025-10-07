use moon_app_context::AppContext;
use moon_vcs_hooks::{HooksGenerator, HooksHash};
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync_vcs_hooks(app_context: &AppContext, force: bool) -> miette::Result<bool> {
    let vcs_config = &app_context.workspace_config.vcs;
    let generator = HooksGenerator::new(app_context, vcs_config, &app_context.workspace_root);

    // Generate the hash
    let mut hooks_hash = HooksHash::new(&vcs_config.manager);

    for (hook_name, commands) in &vcs_config.hooks {
        hooks_hash.add_hook(hook_name, commands);
    }

    // Force run the generator
    if force || !generator.verify_hooks_exist().await? {
        generator.generate().await?;

        app_context
            .cache_engine
            .hash
            .save_manifest_without_hasher("vcs-hooks", hooks_hash)?;

        return Ok(true);
    }

    // Only generate if the hash has changed
    app_context
        .cache_engine
        .execute_if_changed("vcs-hooks", hooks_hash, async |_| {
            generator.generate().await
        })
        .await
        .map(|result| result.unwrap_or_default())
}

#[instrument(skip_all)]
pub async fn unsync_vcs_hooks(app_context: &AppContext) -> miette::Result<()> {
    HooksGenerator::new(
        app_context,
        &app_context.workspace_config.vcs,
        &app_context.workspace_root,
    )
    .cleanup()
    .await?;

    Ok(())
}
