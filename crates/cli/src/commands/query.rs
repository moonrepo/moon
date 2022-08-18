pub use crate::queries::projects::{query_projects, QueryProjectsOptions, QueryProjectsResult};
pub use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use moon_workspace::Workspace;
use std::io;
use std::io::prelude::*;

pub async fn projects(options: &QueryProjectsOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    let result = QueryProjectsResult {
        projects: query_projects(&workspace, options).await?,
        options: options.clone(),
    };

    // Write to stdout directly to avoid broken pipe panics
    let mut stdout = io::stdout().lock();

    writeln!(stdout, "{}", serde_json::to_string_pretty(&result)?)?;

    Ok(())
}

pub async fn touched_files(
    options: &mut QueryTouchedFilesOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    let result = QueryTouchedFilesResult {
        files: query_touched_files(&workspace, options).await?,
        options: options.clone(),
    };

    // Write to stdout directly to avoid broken pipe panics
    let mut stdout = io::stdout().lock();

    writeln!(stdout, "{}", serde_json::to_string_pretty(&result)?)?;

    Ok(())
}
