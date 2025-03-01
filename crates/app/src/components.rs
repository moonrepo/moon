use crate::app::Commands;
use crate::session::CliSession;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_action_pipeline::ActionPipeline;
use moon_platform::PlatformManager;
use moon_workspace::{
    ExtendProjectData, ExtendProjectEvent, ExtendProjectGraphData, ExtendProjectGraphEvent,
    WorkspaceBuilderContext,
};
use starbase_events::{Emitter, EventState};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run_action_pipeline(
    session: &CliSession,
    action_context: ActionContext,
    action_graph: ActionGraph,
) -> miette::Result<Vec<Action>> {
    let workspace_graph = session.get_workspace_graph().await?;
    let toolchain_registry = session.get_toolchain_registry().await?;
    let mut pipeline = ActionPipeline::new(
        session.get_app_context().await?,
        toolchain_registry,
        workspace_graph,
    );

    if let Some(concurrency) = &session.cli.concurrency {
        pipeline.concurrency = *concurrency;
    }

    match &session.cli.command {
        Commands::Check(cmd) => {
            pipeline.bail = true;
            pipeline.summarize = cmd.summary;
        }
        Commands::Ci(_) => {
            pipeline.report_name = "ciReport.json".into();
            pipeline.summarize = true;
        }
        Commands::Run(cmd) => {
            pipeline.bail = !cmd.no_bail;
            pipeline.summarize = cmd.summary;
        }
        _ => {}
    };

    let results = pipeline
        .run_with_context(action_graph, action_context)
        .await?;

    Ok(results)
}

pub async fn create_workspace_graph_context(
    session: &CliSession,
) -> miette::Result<WorkspaceBuilderContext> {
    let context = WorkspaceBuilderContext {
        config_loader: &session.config_loader,
        enabled_toolchains: session.toolchain_config.get_enabled(),
        extend_project: Emitter::<ExtendProjectEvent>::new(),
        extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
        inherited_tasks: &session.tasks_config,
        toolchain_config: &session.toolchain_config,
        vcs: Some(session.get_vcs_adapter()?),
        working_dir: &session.working_dir,
        workspace_config: &session.workspace_config,
        workspace_root: &session.workspace_root,
    };

    context
        .extend_project
        .on(
            |event: Arc<ExtendProjectEvent>, data: Arc<RwLock<ExtendProjectData>>| async move {
                let mut data = data.write().await;

                for platform in PlatformManager::read().list() {
                    data.dependencies
                        .extend(platform.load_project_implicit_dependencies(
                            &event.project_id,
                            event.project_source.as_str(),
                        )?);

                    data.tasks.extend(
                        platform
                            .load_project_tasks(&event.project_id, event.project_source.as_str())?,
                    );
                }

                Ok(EventState::Continue)
            },
        )
        .await;

    context
        .extend_project_graph
        .on(|event: Arc<ExtendProjectGraphEvent>, data: Arc<RwLock<ExtendProjectGraphData>>| async move {
            let mut data = data.write().await;


            for platform in PlatformManager::write().list_mut() {
                platform.load_project_graph_aliases(&event.sources, &mut data.aliases)?;
            }

            Ok(EventState::Continue)
        })
        .await;

    Ok(context)
}
