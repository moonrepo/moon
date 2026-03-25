use crate::helpers::run_action_pipeline;
use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_action_graph::{ActionGraphBuilderOptions, RunRequirements};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;

pub use crate::commands::syncs::SyncCommands;

pub async fn sync(session: MoonSession) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions::default())
        .await?;
    let reqs = RunRequirements::default();

    action_graph_builder.sync_workspace().await?;

    for project in workspace_graph.get_projects()? {
        action_graph_builder.sync_project(&project, &reqs).await?;

        for task in workspace_graph.get_tasks_from_project(&project.id)? {
            if task.options.run_in_sync_phase {
                action_graph_builder
                    .run_task(task.as_ref(), &RunRequirements::default())
                    .await?;
            }
        }
    }

    let (action_context, action_graph) = action_graph_builder.build();

    run_action_pipeline(&session, action_context, action_graph).await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: format!("Synced the workspace and all projects"))
            }
        }
    })?;

    Ok(None)
}
