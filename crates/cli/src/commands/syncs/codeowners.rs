use moon::{generate_project_graph, load_workspace};
use moon_actions::sync_codeowners;
use starbase::AppResult;

pub async fn sync() -> AppResult {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;

    sync_codeowners(&workspace, &project_graph).await?;

    Ok(())
}
