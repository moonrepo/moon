pub use crate::queries::hash::query_hash;
pub use crate::queries::hash_diff::query_hash_diff;
pub use crate::queries::projects::{
    load_touched_files, query_projects, QueryProjectsOptions, QueryProjectsResult, QueryTasksResult,
};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use crate::session::CliSession;
use clap::{Args, Subcommand};
use moon_vcs::TouchedStatus;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::json;
use std::collections::BTreeMap;
use tracing::instrument;

#[derive(Clone, Debug, Subcommand)]
pub enum QueryCommands {
    #[command(
        name = "hash",
        about = "Inspect the contents of a generated hash.",
        long_about = "Inspect the contents of a generated hash, and display all sources and inputs that were used to generate it."
    )]
    Hash(QueryHashArgs),

    #[command(
        name = "hash-diff",
        about = "Query the difference between two hashes.",
        long_about = "Query the difference between two hashes. The left differences will be printed in green, while the right in red, and equal lines in white."
    )]
    HashDiff(QueryHashDiffArgs),

    #[command(
        name = "projects",
        about = "Query for projects within the project graph.",
        long_about = "Query for projects within the project graph. All options support regex patterns."
    )]
    Projects(QueryProjectsArgs),

    #[command(name = "tasks", about = "List all available projects & their tasks.")]
    Tasks(QueryTasksArgs),

    #[command(
        name = "touched-files",
        about = "Query for touched files between revisions."
    )]
    TouchedFiles(QueryTouchedFilesArgs),
}

#[derive(Args, Clone, Debug)]
pub struct QueryHashArgs {
    #[arg(required = true, help = "Hash to inspect")]
    hash: String,

    #[arg(long, help = "Print the manifest in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn hash(session: CliSession, args: QueryHashArgs) -> AppResult {
    let console = &session.console;
    let cache_engine = session.get_cache_engine()?;
    let result = query_hash(&cache_engine, &args.hash).await?;

    if !args.json {
        console
            .out
            .write_line(format!("Hash: {}", color::hash(result.0)))?;
        console.out.write_newline()?;
    }

    console.out.write_line(result.1)?;

    Ok(())
}

#[derive(Args, Clone, Debug)]
pub struct QueryHashDiffArgs {
    #[arg(required = true, help = "Base hash to compare against")]
    left: String,

    #[arg(required = true, help = "Other hash to compare with")]
    right: String,

    #[arg(long, help = "Print the diff in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn hash_diff(session: CliSession, args: QueryHashDiffArgs) -> AppResult {
    let console = &session.console;
    let cache_engine = session.get_cache_engine()?;
    let mut result = query_hash_diff(&cache_engine, &args.left, &args.right).await?;

    if args.json {
        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => result.left_diffs.push(l.trim().to_owned()),
                diff::Result::Right(r) => result.right_diffs.push(r.trim().to_owned()),
                _ => {}
            };
        }

        console.out.write_line(json::format(&result, true)?)?;
    } else {
        console
            .out
            .write_line(format!("Left:  {}", color::hash(&result.left_hash)))?;
        console
            .out
            .write_line(format!("Right: {}", color::hash(&result.right_hash)))?;
        console.out.write_newline()?;

        let is_tty = console.out.is_terminal();

        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => {
                    if is_tty {
                        console.out.write_line(color::success(l))?
                    } else {
                        console.out.write_line(format!("+{}", l))?
                    }
                }
                diff::Result::Both(l, _) => {
                    if is_tty {
                        console.out.write_line(l)?
                    } else {
                        console.out.write_line(format!(" {}", l))?
                    }
                }
                diff::Result::Right(r) => {
                    if is_tty {
                        console.out.write_line(color::failure(r))?
                    } else {
                        console.out.write_line(format!("-{}", r))?
                    }
                }
            };
        }
    }

    Ok(())
}

#[derive(Args, Clone, Debug)]
pub struct QueryProjectsArgs {
    #[arg(help = "Filter projects using a query (takes precedence over options)")]
    query: Option<String>,

    #[arg(long, help = "Filter projects that match this alias")]
    alias: Option<String>,

    #[arg(
        long,
        help = "Filter projects that are affected based on touched files"
    )]
    affected: bool,

    #[arg(long, help = "Include direct dependents of queried projects")]
    dependents: bool,

    #[arg(long, help = "Filter projects that match this ID")]
    id: Option<String>,

    #[arg(long, help = "Print the projects in JSON format")]
    json: bool,

    #[arg(long, help = "Filter projects of this programming language")]
    language: Option<String>,

    #[arg(long, help = "Filter projects that match this source path")]
    stack: Option<String>,

    #[arg(long, help = "Filter projects of this tech stack")]
    source: Option<String>,

    #[arg(long, help = "Filter projects that have the following tags")]
    tags: Option<String>,

    #[arg(long, help = "Filter projects that have the following tasks")]
    tasks: Option<String>,

    #[arg(long = "type", help = "Filter projects of this type")]
    type_of: Option<String>,
}

#[instrument(skip_all)]
pub async fn projects(session: CliSession, args: QueryProjectsArgs) -> AppResult {
    let console = &session.console;
    let vcs = session.get_vcs_adapter()?;
    let project_graph = session.get_project_graph().await?;

    let options = QueryProjectsOptions {
        alias: args.alias,
        affected: args.affected,
        dependents: args.dependents,
        id: args.id,
        json: args.json,
        language: args.language,
        query: args.query,
        stack: args.stack,
        source: args.source,
        tags: args.tags,
        tasks: args.tasks,
        touched_files: if args.affected {
            load_touched_files(&vcs).await?
        } else {
            FxHashSet::default()
        },
        type_of: args.type_of,
    };

    let mut projects = query_projects(&project_graph, &options).await?;
    projects.sort_by(|a, d| a.id.cmp(&d.id));

    // Write to stdout directly to avoid broken pipe panics
    if args.json {
        let result = QueryProjectsResult { projects, options };

        console.out.write_line(json::format(&result, true)?)?;
    } else if !projects.is_empty() {
        console.out.write_line(
            projects
                .iter()
                .map(|p| {
                    format!(
                        "{} | {} | {} | {} | {}",
                        p.id, p.source, p.stack, p.type_of, p.language
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    Ok(())
}

#[derive(Args, Clone, Debug)]
pub struct QueryTasksArgs {
    #[arg(help = "Filter projects using a query (takes precedence over options)")]
    query: Option<String>,

    #[arg(long, help = "Filter projects that match this alias")]
    alias: Option<String>,

    #[arg(
        long,
        help = "Filter projects that are affected based on touched files"
    )]
    affected: bool,

    #[arg(long, help = "Filter projects that match this ID")]
    id: Option<String>,

    #[arg(long, help = "Print the tasks in JSON format")]
    json: bool,

    #[arg(long, help = "Filter projects of this programming language")]
    language: Option<String>,

    #[arg(long, help = "Filter projects that match this source path")]
    source: Option<String>,

    #[arg(long, help = "Filter projects that have the following tasks")]
    tasks: Option<String>,

    #[arg(long = "type", help = "Filter projects of this type")]
    type_of: Option<String>,
}

#[instrument(skip_all)]
pub async fn tasks(session: CliSession, args: QueryTasksArgs) -> AppResult {
    let console = &session.console;
    let vcs = session.get_vcs_adapter()?;
    let project_graph = session.get_project_graph().await?;

    let options = QueryProjectsOptions {
        alias: args.alias,
        id: args.id,
        json: args.json,
        language: args.language,
        query: args.query,
        source: args.source,
        tasks: args.tasks,
        type_of: args.type_of,
        ..QueryProjectsOptions::default()
    };

    let projects = query_projects(&project_graph, &options).await?;
    let touched_files = if args.affected {
        load_touched_files(&vcs).await?
    } else {
        FxHashSet::default()
    };

    // Filter and group tasks
    let mut grouped_tasks = BTreeMap::default();

    for project in projects {
        let filtered_tasks = project
            .tasks
            .iter()
            .filter_map(|(task_id, task)| {
                if !args.affected || task.is_affected(&touched_files).is_ok_and(|v| v) {
                    Some((task_id.to_owned(), task.to_owned()))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();

        if filtered_tasks.is_empty() {
            continue;
        }

        grouped_tasks.insert(project.id.clone(), filtered_tasks);
    }

    // Write to stdout directly to avoid broken pipe panics
    if options.json {
        console.out.write_line(json::format(
            &QueryTasksResult {
                tasks: grouped_tasks,
                options,
            },
            true,
        )?)?;
    } else if !grouped_tasks.is_empty() {
        for (project_id, tasks) in grouped_tasks {
            console.out.write_line(project_id.as_str())?;

            for (task_id, task) in tasks {
                console
                    .out
                    .write_line(format!("\t:{} | {}", task_id, task.command))?;
            }
        }
    }

    Ok(())
}

#[derive(Args, Clone, Debug)]
pub struct QueryTouchedFilesArgs {
    #[arg(long, help = "Base branch, commit, or revision to compare against")]
    base: Option<String>,

    #[arg(
        long = "defaultBranch",
        help = "When on the default branch, compare against the previous revision"
    )]
    default_branch: bool,

    #[arg(long, help = "Current branch, commit, or revision to compare with")]
    head: Option<String>,

    #[arg(long, help = "Print the files in JSON format")]
    json: bool,

    #[arg(long, help = "Gather files from you local state instead of the remote")]
    local: bool,

    #[arg(long, help = "Filter files based on a touched status")]
    status: Vec<TouchedStatus>,
}

#[instrument(skip_all)]
pub async fn touched_files(session: CliSession, args: QueryTouchedFilesArgs) -> AppResult {
    let console = &session.console;
    let vcs = session.get_vcs_adapter()?;

    let options = QueryTouchedFilesOptions {
        base: args.base,
        default_branch: args.default_branch,
        head: args.head,
        json: args.json,
        local: args.local,
        status: args.status,
    };

    let result = query_touched_files(&vcs, &options).await?;

    // Write to stdout directly to avoid broken pipe panics
    if args.json {
        console.out.write_line(json::format(&result, true)?)?;
    } else if !result.files.is_empty() {
        let mut files = result
            .files
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>();
        files.sort();

        console.out.write_line(files.join("\n"))?;
    }

    Ok(())
}
