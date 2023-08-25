use crate::enums::TouchedStatus;
pub use crate::queries::hash::query_hash;
pub use crate::queries::hash_diff::query_hash_diff;
pub use crate::queries::projects::{
    query_projects, QueryProjectsOptions, QueryProjectsResult, QueryTasksResult,
};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use clap::Args;
use console::Term;
use miette::IntoDiagnostic;
use moon_terminal::ExtendedTerm;
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_styles::color;
use std::io::{self, IsTerminal};

#[derive(Args, Clone, Debug)]
pub struct QueryHashArgs {
    #[arg(required = true, help = "Hash to inspect")]
    hash: String,

    #[arg(long, help = "Print the manifest in JSON format")]
    json: bool,
}

pub async fn hash(args: QueryHashArgs, workspace: Workspace) -> AppResult {
    let result = query_hash(&workspace, &args.hash).await?;

    if !args.json {
        println!("Hash: {}\n", color::id(result.0));
    }

    println!("{}", result.1);

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

pub async fn hash_diff(args: QueryHashDiffArgs, mut workspace: Workspace) -> AppResult {
    let mut result = query_hash_diff(&mut workspace, &args.left, &args.right).await?;

    let is_tty = io::stdout().is_terminal();
    let term = Term::buffered_stdout();

    if args.json {
        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => result.left_diffs.push(l.trim().to_owned()),
                diff::Result::Right(r) => result.right_diffs.push(r.trim().to_owned()),
                _ => {}
            };
        }

        term.line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else {
        term.line(format!("Left:  {}", color::id(&result.left_hash)))?;
        term.line(format!("Right: {}\n", color::id(&result.right_hash)))?;

        for diff in diff::lines(&result.left, &result.right) {
            match diff {
                diff::Result::Left(l) => {
                    if is_tty {
                        term.line(color::success(l))?
                    } else {
                        term.line(format!("+{}", l))?
                    }
                }
                diff::Result::Both(l, _) => {
                    if is_tty {
                        term.line(l)?
                    } else {
                        term.line(format!(" {}", l))?
                    }
                }
                diff::Result::Right(r) => {
                    if is_tty {
                        term.line(color::failure(r))?
                    } else {
                        term.line(format!("-{}", r))?
                    }
                }
            };
        }
    }

    term.flush_lines()?;

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

pub async fn projects(args: QueryProjectsArgs, mut workspace: Workspace) -> AppResult {
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
        type_of: args.type_of,
    };

    let mut projects = query_projects(&mut workspace, &options).await?;

    projects.sort_by(|a, d| a.id.cmp(&d.id));

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if args.json {
        let result = QueryProjectsResult { projects, options };

        term.line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else if !projects.is_empty() {
        term.line(
            projects
                .iter()
                .map(|p| format!("{} | {} | {} | {}", p.id, p.source, p.type_of, p.language))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    term.flush_lines()?;

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

pub async fn tasks(args: QueryTasksArgs, mut workspace: Workspace) -> AppResult {
    let options = QueryProjectsOptions {
        alias: args.alias,
        affected: args.affected,
        id: args.id,
        json: args.json,
        language: args.language,
        query: args.query,
        source: args.source,
        tags: None,
        tasks: args.tasks,
        type_of: args.type_of,
    };

    let projects = query_projects(&mut workspace, &options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if options.json {
        let result = QueryTasksResult {
            tasks: FxHashMap::from_iter(
                projects
                    .into_iter()
                    .map(|p| (p.id.clone(), p.tasks.clone())),
            ),
            options,
        };

        term.line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else if !projects.is_empty() {
        for project in projects {
            if project.tasks.is_empty() {
                continue;
            }

            term.line(&project.id)?;

            for (task_id, task) in &project.tasks {
                term.line(format!("\t:{} | {}", task_id, task.command))?;
            }
        }
    }

    term.flush_lines()?;

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

    #[arg(value_enum, long, help = "Filter files based on a touched status")]
    status: Vec<TouchedStatus>,
}

pub async fn touched_files(args: QueryTouchedFilesArgs, workspace: Workspace) -> AppResult {
    let options = &mut QueryTouchedFilesOptions {
        base: args.base,
        default_branch: args.default_branch,
        head: args.head,
        json: args.json,
        local: args.local,
        log: false,
        status: args.status,
    };

    let files = query_touched_files(&workspace, options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if args.json {
        let result = QueryTouchedFilesResult {
            files,
            options: options.clone(),
        };

        term.line(serde_json::to_string_pretty(&result).into_diagnostic()?)?;
    } else if !files.is_empty() {
        term.line(
            files
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    term.flush_lines()?;

    Ok(())
}
