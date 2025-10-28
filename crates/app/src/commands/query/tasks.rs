use super::{HEADING_AFFECTED, HEADING_FILTERS};
use crate::queries::changed_files::*;
use crate::queries::tasks::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use starbase::AppResult;
use starbase_utils::json;
use std::collections::BTreeMap;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryTasksArgs {
    #[arg(help = "Filter tasks using a query (takes precedence over options)")]
    query: Option<String>,

    // Affected
    #[arg(
        long,
        help = "Filter tasks that are affected based on changed files",
        help_heading = HEADING_AFFECTED,
        group = "affected-args"
    )]
    affected: bool,

    #[arg(
        long,
        default_value_t,
        help = "Include downstream dependents of queried tasks",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    downstream: DownstreamScope,

    #[arg(
        long,
        default_value_t,
        help = "Include upstream dependencies of queried tasks",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    upstream: UpstreamScope,

    // Filters
    #[arg(long, help = "Filter tasks that match this ID", help_heading = HEADING_FILTERS)]
    id: Option<String>,

    #[arg(long, help = "Filter tasks with the provided command", help_heading = HEADING_FILTERS)]
    command: Option<String>,

    #[arg(long, help = "Filter tasks that belong to a project", help_heading = HEADING_FILTERS)]
    project: Option<String>,

    #[arg(long, help = "Filter tasks with the provided script", help_heading = HEADING_FILTERS)]
    script: Option<String>,

    #[arg(long, help = "Filter tasks that belong to a toolchain", help_heading = HEADING_FILTERS)]
    toolchain: Option<String>,

    #[arg(long = "type", help = "Filter projects of this type", help_heading = HEADING_FILTERS)]
    type_of: Option<String>,
}

#[instrument(skip(session))]
pub async fn tasks(session: MoonSession, args: QueryTasksArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;

    let mut options = QueryTasksOptions {
        affected: None,
        id: args.id,
        command: args.command,
        query: args.query,
        project: args.project,
        script: args.script,
        toolchain: args.toolchain,
        type_of: args.type_of,
    };

    // Filter down to affected tasks only
    if args.affected {
        let vcs = session.get_vcs_adapter()?;
        let changed_files = query_changed_files_for_affected(&vcs).await?;

        let mut affected_tracker = AffectedTracker::new(workspace_graph.clone(), changed_files);
        affected_tracker.with_task_scopes(args.upstream, args.downstream);
        affected_tracker.track_tasks()?;

        options.affected = Some(affected_tracker.build());
    }

    // Query for tasks that match the filters
    let tasks = query_tasks(&workspace_graph, &options).await?;

    let mut result = QueryTasksResult {
        tasks: BTreeMap::default(),
        options,
    };

    for task in tasks {
        let Ok(project_id) = task.target.get_project_id() else {
            continue;
        };

        result
            .tasks
            .entry(project_id.to_owned())
            .or_default()
            .insert(task.id.clone(), task);
    }

    session
        .console
        .out
        .write_line(json::format(&result, true)?)?;

    Ok(None)
}
