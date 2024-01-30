use crate::enums::TouchedStatus;
pub use crate::queries::hash::query_hash;
pub use crate::queries::hash_diff::query_hash_diff;
pub use crate::queries::projects::{
    load_touched_files, query_projects, QueryProjectsOptions, QueryProjectsResult, QueryTasksResult,
};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use clap::Args;
use miette::IntoDiagnostic;
use moon_app_components::StdoutConsole;
use moon_workspace::Workspace;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase::system;
use starbase_styles::color;
use std::collections::BTreeMap;

#[derive(Args, Clone, Debug)]
pub struct QueryHashArgs {
    #[arg(required = true, help = "Hash to inspect")]
    hash: String,

    #[arg(long, help = "Print the manifest in JSON format")]
    json: bool,
}

#[system]
pub async fn hash(
    args: ArgsRef<QueryHashArgs>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<StdoutConsole>,
) {
    let result = query_hash(workspace, &args.hash).await?;

    if !args.json {
        console.write_line(format!("Hash: {}", color::hash(result.0)))?;
        console.print_line()?;
    }

    console.write_line(result.1)?;
    console.flush()?;
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

#[system]
pub async fn hash_diff(
    args: ArgsRef<QueryHashDiffArgs>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<StdoutConsole>,
) {
    let mut result = query_hash_diff(workspace, &args.left, &args.right).await?;
    let is_tty = console.is_terminal();

    if args.json {
        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => result.left_diffs.push(l.trim().to_owned()),
                diff::Result::Right(r) => result.right_diffs.push(r.trim().to_owned()),
                _ => {}
            };
        }

        console.write_line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else {
        console.write_line(format!("Left:  {}", color::hash(&result.left_hash)))?;
        console.write_line(format!("Right: {}", color::hash(&result.right_hash)))?;
        console.print_line()?;

        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => {
                    if is_tty {
                        console.write_line(color::success(l))?
                    } else {
                        console.write_line(format!("+{}", l))?
                    }
                }
                diff::Result::Both(l, _) => {
                    if is_tty {
                        console.write_line(l)?
                    } else {
                        console.write_line(format!(" {}", l))?
                    }
                }
                diff::Result::Right(r) => {
                    if is_tty {
                        console.write_line(color::failure(r))?
                    } else {
                        console.write_line(format!("-{}", r))?
                    }
                }
            };
        }
    }

    console.flush()?;
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

    #[arg(long, help = "Filter projects that match this ID")]
    id: Option<String>,

    #[arg(long, help = "Print the projects in JSON format")]
    json: bool,

    #[arg(long, help = "Filter projects of this programming language")]
    language: Option<String>,

    #[arg(long, help = "Filter projects that match this source path")]
    source: Option<String>,

    #[arg(long, help = "Filter projects that have the following tags")]
    tags: Option<String>,

    #[arg(long, help = "Filter projects that have the following tasks")]
    tasks: Option<String>,

    #[arg(long = "type", help = "Filter projects of this type")]
    type_of: Option<String>,
}

#[system]
pub async fn projects(args: ArgsRef<QueryProjectsArgs>, resources: ResourcesMut) {
    let args = args.to_owned();
    let options = QueryProjectsOptions {
        alias: args.alias,
        affected: args.affected,
        id: args.id,
        json: args.json,
        language: args.language,
        query: args.query,
        source: args.source,
        tags: args.tags,
        tasks: args.tasks,
        touched_files: if args.affected {
            load_touched_files(resources.get::<Workspace>()).await?
        } else {
            FxHashSet::default()
        },
        type_of: args.type_of,
    };

    let mut projects = { query_projects(resources.get_mut::<Workspace>(), &options).await? };

    projects.sort_by(|a, d| a.id.cmp(&d.id));

    // Write to stdout directly to avoid broken pipe panics
    let console = resources.get::<StdoutConsole>();

    if args.json {
        let result = QueryProjectsResult { projects, options };

        console.write_line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else if !projects.is_empty() {
        console.write_line(
            projects
                .iter()
                .map(|p| format!("{} | {} | {} | {}", p.id, p.source, p.type_of, p.language))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    console.flush()?;
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

#[system]
pub async fn tasks(args: ArgsRef<QueryTasksArgs>, resources: ResourcesMut) {
    let args = args.to_owned();
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

    let projects = { query_projects(resources.get_mut::<Workspace>(), &options).await? };
    let touched_files = if args.affected {
        load_touched_files(resources.get::<Workspace>()).await?
    } else {
        FxHashSet::default()
    };

    // Filter and group tasks
    let mut grouped_tasks = FxHashMap::default();

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
    let console = resources.get::<StdoutConsole>();

    if options.json {
        console.write_line(
            serde_json::to_string_pretty(&QueryTasksResult {
                tasks: grouped_tasks,
                options,
            })
            .into_diagnostic()?,
        )?;
    } else if !grouped_tasks.is_empty() {
        for (project_id, tasks) in grouped_tasks {
            console.write_line(project_id.as_str())?;

            for (task_id, task) in tasks {
                console.write_line(format!("\t:{} | {}", task_id, task.command))?;
            }
        }
    }

    console.flush()?;
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

    #[arg(value_enum, long, help = "Filter files based on a touched status")]
    status: Vec<TouchedStatus>,
}

#[system]
pub async fn touched_files(
    args: ArgsRef<QueryTouchedFilesArgs>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<StdoutConsole>,
) {
    let args = args.to_owned();
    let options = QueryTouchedFilesOptions {
        base: args.base,
        default_branch: args.default_branch,
        head: args.head,
        json: args.json,
        local: args.local,
        log: false,
        status: args.status,
    };

    let files = query_touched_files(workspace, &options).await?;

    // Write to stdout directly to avoid broken pipe panics
    if args.json {
        let result = QueryTouchedFilesResult {
            files,
            options: options.clone(),
        };

        console.write_line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else if !files.is_empty() {
        console.write_line(
            files
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    console.flush()?;
}
