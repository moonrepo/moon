pub use crate::commands::queries::touched_files::QueryTouchedFilesOptions;
use crate::commands::queries::touched_files::{
    query_touched_files as base_query_touched_files, QueryTouchedFilesResult,
};
use moon_workspace::Workspace;

pub async fn query_touched_files(
    options: &mut QueryTouchedFilesOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    let result = QueryTouchedFilesResult {
        files: base_query_touched_files(&workspace, options).await?,
        options: options.clone(),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
