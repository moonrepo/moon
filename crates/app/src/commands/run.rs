use super::exec::*;
use crate::prompts::select_targets;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::with_shared_exec_props;
use moon_console::ui::{SelectOption, SelectProps};
use moon_task::TargetLocator;
use starbase::AppResult;
use std::mem;
use tracing::instrument;

#[with_shared_exec_props]
#[derive(Args, Clone, Debug, Default)]
pub struct RunArgs {
    #[arg(help = "List of explicit task targets to run")]
    pub targets: Vec<TargetLocator>,

    #[arg(
        long,
        help = "Filter tasks based on the result of a query",
        help_heading = super::HEADING_WORKFLOW,
    )]
    pub query: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    pub passthrough: Vec<String>,
}

#[instrument(skip(session))]
pub async fn run(session: MoonSession, mut args: RunArgs) -> AppResult {
    let mut targets = mem::take(&mut args.targets);
    let passthrough = mem::take(&mut args.passthrough);
    let query = args.query.take();

    if targets.is_empty() {
        let workspace_graph = session.get_workspace_graph().await?;
        let tasks = workspace_graph.get_tasks()?;

        let run_targets = select_targets(&session.console, &[], || {
            Ok(SelectProps {
                label: "Which task(s) to run?".into(),
                options: tasks
                    .iter()
                    .map(|task| {
                        SelectOption::new(&task.target).description_opt(task.description.clone())
                    })
                    .collect(),
                multiple: true,
                ..Default::default()
            })
        })
        .await?;

        for target in run_targets {
            targets.push(TargetLocator::Qualified(target));
        }
    }

    exec(session, {
        let mut args = args.into_exec_args();
        args.targets = targets;
        args.on_failure = OnFailure::Bail;
        args.passthrough = passthrough;
        args.query = query;
        args
    })
    .await
}
