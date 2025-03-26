use moon_app_context::AppContext;
use moon_codeowners::{CodeownersGenerator, CodeownersHash};
use moon_config::CodeownersOrderBy;
use moon_workspace_graph::WorkspaceGraph;
use std::path::PathBuf;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync_codeowners(
    app_context: &AppContext,
    workspace_graph: &WorkspaceGraph,
    force: bool,
) -> miette::Result<Option<PathBuf>> {
    let mut generator = CodeownersGenerator::new(
        &app_context.workspace_root,
        app_context.workspace_config.vcs.provider,
    )?;

    // Sort the projects based on config
    let mut projects = workspace_graph.projects.get_all_unexpanded();
    let order_by = app_context.workspace_config.codeowners.order_by;

    projects.sort_by(|a, d| match order_by {
        CodeownersOrderBy::FileSource => a.source.cmp(&d.source),
        CodeownersOrderBy::ProjectName => a.id.cmp(&d.id),
    });

    // Generate a hash for the codeowners file
    let mut codeowners_hash = CodeownersHash::new(&app_context.workspace_config.codeowners);
    codeowners_hash.file_exists = generator.file_path.exists();

    if !app_context
        .workspace_config
        .codeowners
        .global_paths
        .is_empty()
    {
        generator.add_workspace_entries(&app_context.workspace_config.codeowners)?;
    }

    for project in projects {
        if !project.config.owners.paths.is_empty() {
            codeowners_hash.add_project(&project.id, &project.config.owners);

            generator.add_project_entry(
                &project.id,
                project.source.as_str(),
                &project.config.owners,
                &app_context.workspace_config.codeowners,
            )?;
        }
    }

    let file_path = generator.file_path.clone();

    // Force run the generator and bypass cache
    if force {
        generator.generate()?;

        return Ok(Some(file_path));
    }

    // Only generate if the hash has changed
    if app_context
        .cache_engine
        .execute_if_changed("codeowners.json", codeowners_hash, || async {
            generator.generate()
        })
        .await?
    {
        return Ok(Some(file_path));
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn unsync_codeowners(app_context: &AppContext) -> miette::Result<PathBuf> {
    let codeowners = CodeownersGenerator::new(
        &app_context.workspace_root,
        app_context.workspace_config.vcs.provider,
    )?;

    let file_path = codeowners.file_path.clone();

    codeowners.cleanup()?;

    Ok(file_path)
}
