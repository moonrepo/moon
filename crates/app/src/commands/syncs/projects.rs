use crate::helpers::run_action_pipeline;
use crate::session::{MoonSession, SessionResult};
use iocraft::prelude::element;
use moon_action_graph::{ActionGraphBuilderOptions, BuildOutput, RunRequirements};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use tracing::instrument;

#[instrument(skip_all)]
pub async fn sync(session: MoonSession) -> SessionResult {
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

    action_graph_builder.set_requirements(RunRequirements::default());

    for project in workspace_graph.get_projects()? {
        action_graph_builder.sync_project(&project);
        project_count += 1;
    }

    let BuildOutput {
        context: action_context,
        graph: action_graph,
        ..
    } = action_graph_builder.build().await?;

    run_action_pipeline(&session, action_context, action_graph, None).await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: format!("Synced {project_count} projects"))
            }
        }
    })?;

    Ok(None)
}
