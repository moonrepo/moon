use crate::helpers::create_progress_bar;
use moon::{generate_project_graph, load_workspace};
use moon_actions::{sync_codeowners, unsync_codeowners};
use starbase::AppResult;
use starbase_styles::color;

pub struct SyncCodeownersOptions {
    pub clean: bool,
    pub force: bool,
}

pub async fn sync(options: SyncCodeownersOptions) -> AppResult {
    let mut workspace = load_workspace().await?;

    let done = create_progress_bar("Syncing code owners...");

    if options.clean {
        let codeowners_path = unsync_codeowners(&workspace).await?;

        done(
            format!(
                "Successfully removed {}",
                color::path(codeowners_path.strip_prefix(&workspace.root).unwrap())
            ),
            true,
        );
    } else {
        let project_graph = generate_project_graph(&mut workspace).await?;
        let codeowners_path = sync_codeowners(&workspace, &project_graph, options.force).await?;

        done(
            format!(
                "Successfully created {}",
                color::path(codeowners_path.strip_prefix(&workspace.root).unwrap())
            ),
            true,
        );
    }

    Ok(())
}
