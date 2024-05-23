use crate::helpers::create_progress_bar;
use moon::{build_action_graph, generate_project_graph};
use moon_action_pipeline::Pipeline;
use moon_app_components::Console;
use moon_workspace::Workspace;
use starbase::{system, ResourceManager, SystemResult};
use std::sync::Arc;

pub async fn internal_sync(resources: Arc<ResourceManager>) -> SystemResult {
    let done = create_progress_bar("Syncing projects...");

    let mut workspace = resources.get_async::<Workspace>().await;
    let console = resources.get_async::<Console>().await;
    let project_graph = generate_project_graph(&mut workspace).await?;

    let mut project_count = 0;
    let mut action_graph_builder = build_action_graph(&project_graph)?;

    for project in project_graph.get_all_unexpanded() {
        action_graph_builder.sync_project(project)?;
        project_count += 1;
    }

    let action_graph = action_graph_builder.build()?;

    let mut pipeline = Pipeline::new(workspace.to_owned(), project_graph);

    pipeline
        .run(action_graph, Arc::new(console.to_owned()), None)
        .await?;

    done(
        format!("Successfully synced {project_count} projects"),
        true,
    );

    Ok(())
}

#[system]
pub async fn sync(resources: Resources) {
    internal_sync(resources).await?;
}
