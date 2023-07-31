use moon_codeowners::{CodeownersGenerator, CodeownersHash};
use moon_config::CodeownersOrderBy;
use moon_hash::HashEngine;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::path::PathBuf;

pub async fn sync_codeowners(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    force: bool,
) -> miette::Result<PathBuf> {
    let hash_engine = HashEngine::new(&workspace.cache.dir);
    let mut hasher = hash_engine.create_hasher("CODEOWNERS");

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

    hasher.hash_content(&codeowners_hash);

    // Check the cache before writing the file
    let mut cache = workspace.cache.cache_codeowners_state()?;
    let file_path = codeowners.file_path.clone();

    if force || hasher.generate_hash()? != cache.last_hash {
        codeowners.generate()?;

        cache.last_hash = hash_engine.save_manifest(hasher)?;
        cache.save()?;
    }

    Ok(file_path)
}

pub async fn unsync_codeowners(workspace: &Workspace) -> miette::Result<PathBuf> {
    let codeowners = CodeownersGenerator::new(&workspace.root, workspace.config.vcs.provider)?;
    let file_path = codeowners.file_path.clone();

    codeowners.cleanup()?;

    Ok(file_path)
}
