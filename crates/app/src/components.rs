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
use moon_workspace::WorkspaceBuilderContext;
use std::ops::Deref;
use std::sync::Arc;

pub async fn run_action_pipeline(
    session: &MoonSession,
    action_context: ActionContext,
    action_graph: ActionGraph,
) -> miette::Result<Vec<Action>> {
    let mut pipeline = ActionPipeline::new(
        session.get_app_context().await?,
        session.get_workspace_graph().await?,
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
) -> miette::Result<WorkspaceBuilderContext<'_>> {
    let context = WorkspaceBuilderContext {
        config_loader: &session.config_loader,
        enabled_toolchains: session.toolchains_config.get_enabled(),
        inherited_tasks: &session.tasks_config,
        toolchains_config: &session.toolchains_config,
        toolchain_registry: session.get_toolchain_registry().await?,
        vcs: Some(session.get_vcs_adapter()?),
        working_dir: &session.working_dir,
        workspace_config: &session.workspace_config,
        workspace_root: &session.workspace_root,
    };

    Ok(context)
}

pub async fn create_progress_loader(
    console: Arc<Console>,
    message: impl AsRef<str>,
) -> ProgressInstance {
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

    // Wait a bit for the component to be rendered
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

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

impl Deref for ProgressInstance {
    type Target = ProgressReporter;

    fn deref(&self) -> &Self::Target {
        &self.reporter
    }
}
