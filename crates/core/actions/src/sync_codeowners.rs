use moon_cache_item::CommonState;
use moon_codeowners::{CodeownersGenerator, CodeownersHash};
use moon_config::CodeownersOrderBy;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::path::PathBuf;

pub async fn sync_codeowners(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    force: bool,
) -> miette::Result<PathBuf> {
    let cache_engine = &workspace.cache_engine;
    let hash_engine = &cache_engine.hash;

    // Sort the projects based on config
    let mut projects = project_graph.get_all_unexpanded();
    let order_by = workspace.config.codeowners.order_by;

    projects.sort_by(|a, d| match order_by {
        CodeownersOrderBy::FileSource => a.source.cmp(&d.source),
        CodeownersOrderBy::ProjectName => a.id.cmp(&d.id),
    });

    // Generate the codeowners file
    let mut codeowners_hash = CodeownersHash::new(&workspace.config.codeowners);
    let mut codeowners = CodeownersGenerator::new(&workspace.root, workspace.config.vcs.provider)?;

    if !workspace.config.codeowners.global_paths.is_empty() {
        codeowners.add_workspace_entries(&workspace.config.codeowners)?;
    }

    for project in &projects {
        if !project.config.owners.paths.is_empty() {
            codeowners_hash.add_project(&project.id, &project.config.owners);

            codeowners.add_project_entry(
                &project.id,
                project.source.as_str(),
                &project.config.owners,
            )?;
        }
    }

    let file_path = codeowners.file_path.clone();

    // Check the cache before writing the file
    let mut state = cache_engine
        .state
        .load_state::<CommonState>("codeowners.json")?;
    let hash = hash_engine.save_manifest_without_hasher("CODEOWNERS", &codeowners_hash)?;

    if force || hash != state.data.last_hash {
        codeowners.generate()?;

        state.data.last_hash = hash;
        state.save()?;
    }

    Ok(file_path)
}

pub async fn unsync_codeowners(workspace: &Workspace) -> miette::Result<PathBuf> {
    let codeowners = CodeownersGenerator::new(&workspace.root, workspace.config.vcs.provider)?;
    let file_path = codeowners.file_path.clone();

    codeowners.cleanup()?;

    Ok(file_path)
}
