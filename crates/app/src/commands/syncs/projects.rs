use crate::components::run_action_pipeline;
use crate::session::CliSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync(session: CliSession) -> AppResult {
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

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: format!("Synced {project_count} projects"))
            }
        }
    })?;

    Ok(None)
}
