use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_codeowners::{CodeownersGenerator, CodeownersHash};
use moon_config::CodeownersOrderBy;
use moon_hash::HashEngine;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

// const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_codeowners(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
) -> miette::Result<ActionStatus> {
    let hash_engine = HashEngine::new(&workspace.cache.dir);
    let mut hasher = hash_engine.create_hasher("CODEOWNERS");

    // Sort the projects based on config
    let mut projects = project_graph.get_all()?;
    let order_by = workspace.config.codeowners.order_by;

    projects.sort_by(|a, d| match order_by {
        CodeownersOrderBy::FileSource => a.source.cmp(&d.source),
        CodeownersOrderBy::ProjectName => a.id.cmp(&d.id),
    });

    // Generate the codeowners file
    let mut codeowners_hash = CodeownersHash::new(&workspace.config.codeowners);
    let mut codeowners = CodeownersGenerator::new(&workspace.root, workspace.config.vcs.provider)?;

    codeowners.add_workspace_entries(&workspace.config.codeowners)?;

    for project in projects {
        codeowners_hash.add_project(&project.id, &project.config.owners);
        codeowners.add_project_entry(&project.id, &project.source, &project.config.owners)?;
    }

    hasher.hash_content(&codeowners_hash);

    // Check the cache before writing the file
    let mut cache = workspace.cache.cache_codeowners_state()?;

    if hasher.generate_hash()? != cache.last_hash {
        codeowners.generate()?;

        cache.last_hash = HashEngine::new(&workspace.cache.dir).save_manifest(hasher)?;
        cache.save()?;
    }

    Ok(ActionStatus::Passed)
}

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> miette::Result<ActionStatus> {
    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    if workspace.config.codeowners.sync_on_run {
        sync_codeowners(&workspace, &project_graph).await?;
    }

    Ok(ActionStatus::Passed)
}
