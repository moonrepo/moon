use super::{HEADING_AFFECTED, HEADING_FILTERS};
use crate::queries::changed_files::*;
use crate::queries::projects::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryProjectsArgs {
    #[arg(help = "Filter projects using a query (takes precedence over options)")]
    query: Option<String>,

    // Affected
    #[arg(
        long,
        help = "Filter projects that are affected based on changed files",
        help_heading = HEADING_AFFECTED,
        group = "affected-args"
    )]
    affected: bool,

    #[arg(
        long,
        default_value_t,
        help = "Include downstream dependents of queried projects",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    downstream: DownstreamScope,

    #[arg(
        long,
        default_value_t,
        help = "Include upstream dependencies of queried projects",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    upstream: UpstreamScope,

    // Filters
    #[arg(long, help = "Filter projects that match this alias", help_heading = HEADING_FILTERS)]
    alias: Option<String>,

    #[arg(long, help = "Filter projects that match this ID", help_heading = HEADING_FILTERS)]
    id: Option<String>,

    #[arg(long, help = "Filter projects of this programming language", help_heading = HEADING_FILTERS)]
    language: Option<String>,

    #[arg(long, help = "Filter projects of this layer", help_heading = HEADING_FILTERS)]
    layer: Option<String>,

    #[arg(long, help = "Filter projects that match this source path", help_heading = HEADING_FILTERS)]
    stack: Option<String>,

    #[arg(long, help = "Filter projects of this tech stack", help_heading = HEADING_FILTERS)]
    source: Option<String>,

    #[arg(long, help = "Filter projects that have the following tags", help_heading = HEADING_FILTERS)]
    tags: Option<String>,

    #[arg(long, help = "Filter projects that have the following tasks", help_heading = HEADING_FILTERS)]
    tasks: Option<String>,
}

#[instrument(skip(session))]
pub async fn projects(session: MoonSession, args: QueryProjectsArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;

    let mut options = QueryProjectsOptions {
        alias: args.alias,
        affected: None,
        id: args.id,
        language: args.language,
        layer: args.layer,
        query: args.query,
        stack: args.stack,
        source: args.source,
        tags: args.tags,
        tasks: args.tasks,
    };

    // Filter down to affected projects only
    if args.affected {
        let vcs = session.get_vcs_adapter()?;
        let changed_files = query_changed_files_for_affected(&vcs).await?;

        let mut affected_tracker = AffectedTracker::new(workspace_graph.clone(), changed_files);
        affected_tracker.with_project_scopes(args.upstream, args.downstream);
        affected_tracker.track_projects()?;

        options.affected = Some(affected_tracker.build());
    }

    // Query for projects that match the filters
    let projects = query_projects(&workspace_graph, &options).await?;

    session.console.out.write_line(json::format(
        &QueryProjectsResult { projects, options },
        true,
    )?)?;

    Ok(None)
}
