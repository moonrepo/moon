use crate::helpers::run_action_pipeline;
use crate::session::{MoonSession, SessionResult};
use iocraft::prelude::element;
use moon_action_graph::{ActionGraphBuilderOptions, BuildOutput, RunRequirements};
use moon_console::ui::{Container, Notice, StyledText, Variant};

pub use crate::commands::syncs::SyncCommands;

pub async fn sync(session: MoonSession) -> SessionResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions::default())
        .await?;

    action_graph_builder.set_requirements(RunRequirements::default());
    action_graph_builder.sync_workspace();

    for project in workspace_graph.get_projects()? {
        action_graph_builder.sync_project(&project);

        for task in workspace_graph.get_tasks_from_project(&project.id)? {
            if task.options.run_in_sync_phase {
                action_graph_builder.run_task(task.as_ref());
            }
        }
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
                StyledText(content: "Synced the workspace and all projects".to_string())
            }
        }
    })?;

    Ok(None)
}
