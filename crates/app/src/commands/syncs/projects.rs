use crate::components::run_action_pipeline;
use crate::helpers::create_progress_bar;
use crate::session::CliSession;
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
