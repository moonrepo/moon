use crate::components::run_action_pipeline;
use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use starbase::AppResult;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync(session: CliSession) -> AppResult {
    let done = create_progress_bar("Syncing projects...");

    let workspace_graph = session.get_workspace_graph().await?;
    let mut project_count = 0;
    let mut action_graph_builder = session.build_action_graph(&workspace_graph).await?;

    for project in workspace_graph.projects.get_all_unexpanded() {
        action_graph_builder.sync_project(project)?;
        project_count += 1;
    }

    run_action_pipeline(
        &session,
        action_graph_builder.build_context(),
        action_graph_builder.build(),
    )
    .await?;

    done(
        format!("Successfully synced {project_count} projects"),
        true,
    );

    Ok(())
}
