use crate::components::run_action_pipeline;
use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_action_graph::ActionGraphBuilderOptions;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync(session: MoonSession) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut project_count = 0;
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions {
            install_dependencies: false.into(),
            setup_environment: false.into(),
            setup_toolchains: false.into(),
            sync_projects: true.into(),
            sync_project_dependencies: true,
            sync_workspace: false,
        })
        .await?;

    for project in workspace_graph.projects.get_all_unexpanded() {
        action_graph_builder.sync_project(project).await?;
        project_count += 1;
    }

    let (action_context, action_graph) = action_graph_builder.build();

    run_action_pipeline(&session, action_context, action_graph).await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: format!("Synced {project_count} projects"))
            }
        }
    })?;

    Ok(None)
}
