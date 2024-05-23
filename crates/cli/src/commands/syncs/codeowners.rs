use crate::helpers::create_progress_bar;
use clap::Args;
use moon::generate_project_graph;
use moon_actions::{sync_codeowners, unsync_codeowners};
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct SyncCodeownersArgs {
    #[arg(long, help = "Clean and remove previously generated file")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create file")]
    force: bool,
}

#[system]
pub async fn sync(args: ArgsRef<SyncCodeownersArgs>, workspace: ResourceMut<Workspace>) {
    let done = create_progress_bar("Syncing code owners...");

    if args.clean {
        let codeowners_path = unsync_codeowners(workspace).await?;

        done(
            format!(
                "Successfully removed {}",
                color::path(codeowners_path.strip_prefix(&workspace.root).unwrap())
            ),
            true,
        );
    } else {
        let project_graph = generate_project_graph(workspace).await?;
        let codeowners_path = sync_codeowners(workspace, &project_graph, args.force).await?;

        done(
            format!(
                "Successfully created {}",
                if let Some(path) = codeowners_path {
                    color::path(path.strip_prefix(&workspace.root).unwrap())
                } else {
                    "code owners".into()
                }
            ),
            true,
        );
    }
}
