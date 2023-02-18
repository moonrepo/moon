use crate::helpers::AnyError;
pub use crate::queries::projects::{
    query_projects, QueryProjectsOptions, QueryProjectsResult, QueryTasksResult,
};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use moon::load_workspace;
use rustc_hash::FxHashMap;
use std::io;
use std::io::prelude::*;

pub async fn projects(options: &QueryProjectsOptions) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let mut projects = query_projects(&mut workspace, options).await?;

    projects.sort_by(|a, d| a.id.cmp(&d.id));

    // Write to stdout directly to avoid broken pipe panics
    let mut stdout = io::stdout().lock();

    if options.json {
        let result = QueryProjectsResult {
            projects,
            options: options.clone(),
        };

        writeln!(stdout, "{}", serde_json::to_string_pretty(&result)?)?;
    } else if !projects.is_empty() {
        writeln!(
            stdout,
            "{}",
            projects
                .iter()
                .map(|p| format!("{} | {} | {} | {}", p.id, p.source, p.type_of, p.language))
                .collect::<Vec<_>>()
                .join("\n")
        )?;
    }

    Ok(())
}

pub async fn touched_files(options: &mut QueryTouchedFilesOptions) -> Result<(), AnyError> {
    let workspace = load_workspace().await?;
    let files = query_touched_files(&workspace, options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let mut stdout = io::stdout().lock();

    if options.json {
        let result = QueryTouchedFilesResult {
            files,
            options: options.to_owned(),
        };

        writeln!(stdout, "{}", serde_json::to_string_pretty(&result)?)?;
    } else if !files.is_empty() {
        writeln!(
            stdout,
            "{}",
            files
                .iter()
                .map(|f| f.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n")
        )?;
    }

    Ok(())
}

pub async fn tasks(options: &QueryProjectsOptions) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let projects = query_projects(&mut workspace, options).await?;

    // Write to stdout directly to avoid broken pipe panics
    let mut stdout = io::stdout().lock();

    if options.json {
        let result = QueryTasksResult {
            tasks: FxHashMap::from_iter(projects.into_iter().map(|p| (p.id, p.tasks))),
            options: options.to_owned(),
        };

        writeln!(stdout, "{}", serde_json::to_string_pretty(&result)?)?;
    } else if !projects.is_empty() {
        for project in projects {
            if project.tasks.is_empty() {
                continue;
            }

            writeln!(stdout, "{}", &project.id)?;

            for (task_id, task) in &project.tasks {
                writeln!(stdout, "\t:{} | {}", task_id, task.command)?;
            }
        }
    }

    Ok(())
}
