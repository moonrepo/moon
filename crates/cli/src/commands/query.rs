use crate::helpers::AnyError;
pub use crate::queries::projects::{query_projects, QueryProjectsOptions, QueryProjectsResult};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use moon::load_workspace;
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
    } else {
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
    } else {
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
