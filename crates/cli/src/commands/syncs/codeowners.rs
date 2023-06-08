use crate::helpers::create_progress_bar;
use moon::{generate_project_graph, load_workspace};
use moon_actions::sync_codeowners;
use starbase::AppResult;
use starbase_styles::color;

pub async fn sync() -> AppResult {
    let done = create_progress_bar("Syncing code owners...");

    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let codeowners_path = sync_codeowners(&workspace, &project_graph).await?;

    done(
        format!(
            "Successfully synced to {}",
            color::path(codeowners_path.strip_prefix(&workspace.root).unwrap())
        ),
        true,
    );

    Ok(())
}
