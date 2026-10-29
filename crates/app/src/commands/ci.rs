use super::exec::*;
use crate::app_options::SummaryOption;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_affected::Depth;
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_task::TargetLocator;
use std::path::PathBuf;
use tracing::instrument;

#[with_affected_args(always_affected)]
#[with_shared_exec_args]
#[derive(Args, Clone, Debug)]
pub struct CiArgs {
    #[arg(help = "List of explicit task targets to run")]
    targets: Vec<TargetLocator>,
}

#[with_affected_args(always_affected)]
#[derive(Args, Clone, Debug)]
pub struct QueryPreExecArgs {
    #[arg(help = "List of explicit task targets to inspect")]
    targets: Vec<TargetLocator>,

    #[arg(
        long,
        short = 'f',
        env = "MOON_FORCE",
        help = "Include unaffected targets"
    )]
    force: bool,

    #[arg(long, short = 'i', help = "Resolve tasks in interactive mode")]
    interactive: bool,

    #[arg(long, env = "MOON_NO_ACTIONS", help = "Exclude sync and setup actions")]
    no_actions: bool,

    #[arg(
        long,
        short = 'p',
        env = "MOON_EXEC_PLAN",
        help = "Execution plan to resolve"
    )]
    plan: Option<PathBuf>,

    #[arg(long, help = "Include dependents of selected targets", help_heading = super::HEADING_GRAPH)]
    dependents: Option<Depth>,

    #[arg(long, help = "Exclude unselected dependency tasks", help_heading = super::HEADING_GRAPH)]
    no_dependencies: bool,

    #[arg(long, env = "MOON_JOB", help = "Zero-based job index", help_heading = super::HEADING_PARALLELISM)]
    job: Option<usize>,

    #[arg(long, env = "MOON_JOB_TOTAL", help = "Total number of jobs", help_heading = super::HEADING_PARALLELISM)]
    job_total: Option<usize>,
}

#[instrument(skip(session))]
pub async fn ci(session: MoonSession, args: CiArgs) -> SessionResult {
    let mut exec = args.to_exec_args();
    args.apply_affected_to_exec_args(&mut exec);
    exec.targets = args.targets;

    run_ci(session, exec).await
}

#[instrument(skip(session))]
pub async fn pre_exec(session: MoonSession, args: QueryPreExecArgs) -> SessionResult {
    let mut exec = ExecArgs {
        targets: args.targets.clone(),
        force: args.force,
        interactive: args.interactive,
        no_actions: args.no_actions,
        plan: args.plan.clone(),
        dependents: args.dependents,
        no_dependencies: args.no_dependencies,
        job: args.job,
        job_total: args.job_total,
        emit_action_graph: true,
        ..Default::default()
    };
    args.apply_affected_to_exec_args(&mut exec);

    run_ci(session, exec).await
}

async fn run_ci(session: MoonSession, mut exec: ExecArgs) -> SessionResult {
    let mut targets = exec.targets.clone();
    let mut skip_persistent = false;

    if targets.is_empty() && exec.plan.is_none() {
        let workspace_graph = session.get_workspace_graph().await?;

        for task in workspace_graph.get_tasks_unexpanded()? {
            targets.push(TargetLocator::Qualified(task.target.clone()));
        }

        // Persistent tasks never complete, so only run them when another
        // task depends on them, as none of these have been requested
        skip_persistent = true;
    }

    exec.targets = targets;
    exec.on_failure = OnFailure::Continue;
    exec.ignore_ci_checks = false;
    exec.ci = Some(true);
    exec.skip_persistent = skip_persistent;

    // Show full output in CI
    if exec.summary.is_none() {
        exec.summary = Some(Some(SummaryOption::Detailed));
    }

    super::exec::exec(session, exec).await
}
