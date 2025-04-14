use crate::app::Commands;
use crate::session::MoonSession;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_action_pipeline::ActionPipeline;
use moon_console::ui::{OwnedOrShared, Progress, ProgressDisplay, ProgressReporter};
use moon_console::{Console, ConsoleError};
use moon_platform::PlatformManager;
use moon_workspace::{
    ExtendProjectData, ExtendProjectEvent, ExtendProjectGraphData, ExtendProjectGraphEvent,
    WorkspaceBuilderContext,
};
use starbase_events::{Emitter, EventState};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run_action_pipeline(
    session: &MoonSession,
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
        Commands::Sync { .. } => {
            pipeline.summarize = true;
        }
        _ => {}
    };

    let results = pipeline
        .run_with_context(action_graph, action_context)
        .await?;

    Ok(results)
}

pub async fn create_workspace_graph_context(
    session: &MoonSession,
) -> miette::Result<WorkspaceBuilderContext> {
    let context = WorkspaceBuilderContext {
        config_loader: &session.config_loader,
        enabled_toolchains: session.toolchain_config.get_enabled(),
        extend_project: Emitter::<ExtendProjectEvent>::new(),
        extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
        inherited_tasks: &session.tasks_config,
        toolchain_config: &session.toolchain_config,
        toolchain_registry: session.get_toolchain_registry().await?,
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

pub fn create_progress_loader(console: Arc<Console>, message: impl AsRef<str>) -> ProgressInstance {
    let reporter = Arc::new(ProgressReporter::default());
    let reporter_clone = OwnedOrShared::Shared(reporter.clone());
    let message = message.as_ref().to_owned();

    let handle = tokio::task::spawn(async move {
        console
            .render_interactive(element! {
                Progress(
                    default_message: message,
                    display: ProgressDisplay::Loader,
                    reporter: reporter_clone,
                )
            })
            .await
    });

    ProgressInstance { handle, reporter }
}

pub struct ProgressInstance {
    pub handle: tokio::task::JoinHandle<Result<(), ConsoleError>>,
    pub reporter: Arc<ProgressReporter>,
}

impl ProgressInstance {
    pub async fn stop(self) -> miette::Result<()> {
        self.reporter.exit();
        self.handle.await.into_diagnostic()??;

        Ok(())
    }
}
