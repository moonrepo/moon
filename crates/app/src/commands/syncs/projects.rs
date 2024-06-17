use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use moon_action_pipeline::Pipeline;
use starbase::AppResult;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync(session: CliSession) -> AppResult {
    let done = create_progress_bar("Syncing projects...");

    let project_graph = session.get_project_graph().await?;
    let mut project_count = 0;
    let mut action_graph_builder = session.build_action_graph(&project_graph).await?;

    for project in project_graph.get_all_unexpanded() {
        action_graph_builder.sync_project(project)?;
        project_count += 1;
    }

    let action_graph = action_graph_builder.build()?;

    let mut pipeline = Pipeline::new(session.get_workspace_legacy()?, project_graph);

    pipeline
        .run(action_graph, session.create_context()?, None)
        .await?;

    done(
        format!("Successfully synced {project_count} projects"),
        true,
    );

    Ok(())
}
