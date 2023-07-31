pub use crate::queries::hash::query_hash;
pub use crate::queries::hash_diff::{query_hash_diff, QueryHashDiffOptions};
pub use crate::queries::projects::{
    query_projects, QueryProjectsOptions, QueryProjectsResult, QueryTasksResult,
};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use console::Term;
use miette::IntoDiagnostic;
use moon::load_workspace;
use moon_terminal::ExtendedTerm;
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_styles::color;
use std::io::{self, IsTerminal};

pub async fn hash(hash: &str, json: bool) -> AppResult {
    let workspace = load_workspace().await?;
    let result = query_hash(&workspace, hash).await?;

    if !json {
        println!("Hash: {}\n", color::id(result.0));
    }

    println!("{}", result.1);

    Ok(())
}

pub async fn hash_diff(options: &QueryHashDiffOptions) -> AppResult {
    let mut workspace = load_workspace().await?;
    let mut result = query_hash_diff(&mut workspace, options).await?;

    let is_tty = io::stdout().is_terminal();
    let term = Term::buffered_stdout();

    if options.json {
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

pub async fn projects(options: &QueryProjectsOptions) -> AppResult {
    let mut workspace = load_workspace().await?;
    let mut projects = query_projects(&mut workspace, options).await?;

    projects.sort_by(|a, d| a.id.cmp(&d.id));

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if options.json {
        let result = QueryProjectsResult {
            projects,
            options: options.clone(),
        };

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

pub async fn touched_files(options: &mut QueryTouchedFilesOptions) -> AppResult {
    let workspace = load_workspace().await?;
    let files = query_touched_files(&workspace, options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if options.json {
        let result = QueryTouchedFilesResult {
            files,
            options: options.to_owned(),
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

pub async fn tasks(options: &QueryProjectsOptions) -> AppResult {
    let mut workspace = load_workspace().await?;
    let projects = query_projects(&mut workspace, options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let term = Term::buffered_stdout();

    if options.json {
        let result = QueryTasksResult {
            tasks: FxHashMap::from_iter(
                projects
                    .into_iter()
                    .map(|p| (p.id.clone(), p.tasks.clone())),
            ),
            options: options.to_owned(),
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
